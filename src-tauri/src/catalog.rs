//! The port catalog (`catalog/ports.json`, bundled into the program) and the install section
//! of its entries: where a port keeps its settings and how it expects its game file.

use crate::library::{self, BootSetting, RomSpec};
use crate::paths::{expand, home};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

const CATALOG: &str = include_str!("../../catalog/ports.json");

/// The catalog as shipped, without the ports the user added.
pub fn bundled() -> Value {
    serde_json::from_str(CATALOG).expect("catalog/ports.json is valid JSON")
}

/// The catalog plus the ports the user added themselves.
pub fn full() -> Value {
    let mut catalog = bundled();
    if let (Ok(lib), Some(ports)) = (library::load(), catalog["ports"].as_array_mut()) {
        ports.extend(lib.custom);
    }
    catalog
}

/// Every port of a catalog.
pub fn ports(catalog: &Value) -> impl Iterator<Item = &Value> {
    catalog["ports"].as_array().into_iter().flatten()
}

/// One port, from the catalog or the ports the user added.
pub fn port(id: &str) -> Result<Value, String> {
    ports(&full()).find(|p| p["id"] == id).cloned().ok_or_else(|| format!("{id} is not in the catalog"))
}

/// The No-Intro / Redump title of a port, or its name.
pub fn title(port: &Value) -> &str {
    port["title"].as_str().or(port["name"].as_str()).unwrap_or_default()
}

/// Catalog install section of a port.
#[derive(Deserialize, Default)]
pub struct InstallSpec {
    /// Where the releases come from, when not the project's own repository
    /// (OpenGOAL: the launcher, not the compiler tools).
    pub repo: Option<String>,
    pub linux: Option<crate::install::Rule>,
    pub windows: Option<crate::install::Rule>,
    pub macos: Option<crate::install::Rule>,
    #[serde(default)]
    pub args: Vec<String>,
    config_dir: Option<String>,
    /// Harbour Masters ports keep settings next to the program.
    #[serde(default)]
    config_in_install: bool,
    rom: Option<RomSpec>,
    pub boot_setting: Option<BootSetting>,
    #[serde(default)]
    pub game_file_arg: bool,
    /// N64: Recompiled ports: where they keep settings and the ROM follows from these two ids.
    recomp: Option<RecompIds>,
}

/// Ids a RecompFrontend / N64ModernRuntime port is built with (read from its source):
/// settings live in the app folder named after `program_id`, and the checked ROM is stored
/// there as `<game_id>.z64`, which is where the port's own launcher looks for it.
#[derive(Deserialize, Default, Clone)]
struct RecompIds {
    program_id: String,
    game_id: String,
}

/// The app folder RecompFrontend uses: ~/.config/<id> on Linux (it does not follow
/// XDG_CONFIG_HOME), %LOCALAPPDATA%\<id> on Windows, Application Support on macOS.
fn recomp_app_folder(program_id: &str) -> PathBuf {
    match std::env::consts::OS {
        "windows" => dirs::data_local_dir().unwrap_or_default().join(program_id),
        "macos" => dirs::data_dir().unwrap_or_default().join(program_id),
        _ => home().join(".config").join(program_id),
    }
}

impl InstallSpec {
    /// The install section of a port (empty when the catalog has none).
    pub fn of(port: &Value) -> Self {
        serde_json::from_value(port["install"].clone()).unwrap_or_default()
    }

    /// The install rule for this operating system.
    pub fn rule(&self) -> Option<crate::install::Rule> {
        match std::env::consts::OS {
            "linux" => self.linux.clone(),
            "windows" => self.windows.clone(),
            "macos" => self.macos.clone(),
            _ => None,
        }
    }

    /// Settings folder and game file layout for a port installed with this program folder.
    pub fn layout(&self, program_dir: &Path) -> (Option<String>, Option<RomSpec>) {
        if let Some(ids) = &self.recomp {
            let dir = recomp_app_folder(&ids.program_id).to_string_lossy().into_owned();
            return (Some(dir), Some(RomSpec::Stored { file: format!("{}.z64", ids.game_id) }));
        }
        let dir = if self.config_in_install {
            Some(program_dir.to_string_lossy().into_owned())
        } else {
            self.config_dir.as_deref().map(|d| expand(d).to_string_lossy().into_owned())
        };
        (dir, self.rom.clone())
    }
}

#[tauri::command]
pub fn get_catalog() -> Value {
    full()
}

/// The operating system, for knowing which catalog install rule applies.
#[tauri::command]
pub fn platform() -> &'static str {
    std::env::consts::OS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recomp_layout_follows_the_ids() {
        let spec = InstallSpec::of(&port("bk").unwrap());
        let (dir, rom) = spec.layout(Path::new("/anywhere"));
        assert_eq!(dir.unwrap(), home().join(".config/BanjoRecompiled").to_string_lossy());
        assert!(matches!(rom, Some(RomSpec::Stored { file }) if file == "bk.n64.us.1.0.z64"));
    }

    #[test]
    fn every_port_is_well_formed() {
        let catalog = bundled();
        let mut ids = std::collections::HashSet::new();
        for port in ports(&catalog) {
            let id = port["id"].as_str().expect("every port has an id");
            assert!(ids.insert(id.to_string()), "{id} appears twice");
            assert!(catalog["consoles"][port["console"].as_str().unwrap_or_default()].is_object(), "{id}: unknown system");
            if !port["install"].is_null() {
                serde_json::from_value::<InstallSpec>(port["install"].clone()).unwrap_or_else(|e| panic!("{id}: {e}"));
            }
        }
    }
}
