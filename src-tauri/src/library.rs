//! The user's library (`~/.config/portshelf/library.json`): which ports are installed and how
//! to start them, per-port choices made on the shelf, ports added by hand, and the ROM folder.

use crate::paths::{app_dir, expand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// How to start an installed port and where its config lives.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Install {
    pub exec: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub config_dir: Option<String>,
    #[serde(default)]
    pub cover: Option<String>,
    /// A setting written to the port's config right before launching it, used
    /// to skip its own launcher (for ports without a command line flag for it).
    #[serde(default)]
    pub boot_setting: Option<BootSetting>,
    /// Where the port expects its game data; the shelf only starts a port once this is set up.
    #[serde(default)]
    pub rom: Option<RomSpec>,
    /// The port boots straight into the game when given its game file on the command line.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub game_file_arg: bool,
    /// Release tag, for ports installed by PortShelf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// How each kind of port stores the game it needs.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RomSpec {
    /// RecompFrontend ports: a big-endian copy named <game id>.z64 in the config directory.
    Stored { file: String },
    /// A path saved in one of the port's JSON settings files (Dusklight's backend.isoPath).
    ConfigKey { file: String, key: String },
    /// Harbour Masters ports: they turn a ROM found next to the executable into an .o2r archive.
    Extracted { archive: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BootSetting {
    pub file: String,
    pub key: String,
    pub value: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Library {
    pub roms_dir: String,
    pub installed: BTreeMap<String, Install>,
    /// Per-port choices the user made on the shelf, for installed and catalog-only ports alike.
    #[serde(default)]
    pub overrides: BTreeMap<String, Override>,
    /// Ports added by the user that the catalog does not know (same shape as catalog entries).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<Value>,
    /// Game files found for ports that are not installed yet.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pending_roms: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Override {
    /// Display name instead of the catalog's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The cover was picked by hand: automatic scraping leaves it alone.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cover_locked: bool,
}

fn library_path() -> PathBuf {
    app_dir().join("library.json")
}

/// The library as saved; on first run, the ports found in their usual places.
pub fn load() -> Result<Library, String> {
    let path = library_path();
    if let Ok(text) = fs::read_to_string(&path) {
        return serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()));
    }
    let lib = Library { installed: detect_installed(), ..Default::default() };
    save(&lib)?;
    Ok(lib)
}

pub fn save(lib: &Library) -> Result<(), String> {
    fs::create_dir_all(app_dir()).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(lib).map_err(|e| e.to_string())?;
    fs::write(library_path(), text).map_err(|e| e.to_string())
}

/// Loads the library, applies a change and saves it; returns what the change returns.
pub fn update<T>(change: impl FnOnce(&mut Library) -> T) -> Result<T, String> {
    let mut lib = load()?;
    let out = change(&mut lib);
    save(&lib)?;
    Ok(out)
}

/// One installed port.
pub fn installed(id: &str) -> Result<Install, String> {
    load()?.installed.get(id).cloned().ok_or_else(|| format!("{id} is not installed"))
}

/// Ports installed by hand in the places they usually go, found on first run and on rescan.
fn detect_installed() -> BTreeMap<String, Install> {
    // Ports open on their own launcher; PortShelf only sets up the game file for them.
    let candidates: &[(&str, &str, Option<&str>)] = &[
        ("dk64", "~/Applications/DK64Recompiled/DK64Recompiled", Some("~/.config/DK64Recompiled")),
        ("mt64", "/mnt/main/Roms/mariotennis64recomp/run/play.sh", Some("/mnt/main/Roms/mariotennis64recomp/run")),
        ("tp", "~/Applications/Dusklight.AppImage", Some("~/.local/share/TwilitRealm/Dusklight")),
        ("bm64", "~/Applications/BM64Recompiled/BM64Recompiled", Some("~/.config/BM64Recompiled")),
        // Harbour Masters ports keep their settings next to the AppImage.
        ("oot-soh", "~/Applications/SoH/soh.appimage", Some("~/Applications/SoH")),
        ("mm-2s2h", "~/Applications/2Ship/2ship.appimage", Some("~/Applications/2Ship")),
    ];
    let mut installed = BTreeMap::new();
    for (id, exec, config_dir) in candidates {
        let exec_path = expand(exec);
        if !exec_path.exists() {
            continue;
        }
        let rom = match *id {
            "tp" => RomSpec::ConfigKey { file: "config".into(), key: "backend.isoPath".into() },
            "oot-soh" => RomSpec::Extracted { archive: "oot.o2r".into() },
            "mm-2s2h" => RomSpec::Extracted { archive: "mm.o2r".into() },
            // Recomps store the ROM as <game id>.z64.
            "bm64" => RomSpec::Stored { file: "bm64_us.z64".into() },
            other => RomSpec::Stored { file: format!("{}.z64", other.to_uppercase()) },
        };
        installed.insert(
            id.to_string(),
            Install {
                exec: exec_path.to_string_lossy().into(),
                cwd: exec_path.parent().map(|p| p.to_string_lossy().into()),
                config_dir: config_dir.map(|d| expand(d).to_string_lossy().into()),
                rom: Some(rom),
                ..Default::default()
            },
        );
    }
    installed
}

#[tauri::command]
pub fn get_library() -> Result<Library, String> {
    load()
}

/// Looks for hand-installed ports again. Found entries are refreshed (launch flags, game file
/// layout) but keep a cover the user picked; ports added by hand are left alone.
#[tauri::command]
pub fn rescan() -> Result<Library, String> {
    update(|lib| {
        for (id, mut install) in detect_installed() {
            if let Some(old) = lib.installed.get(&id) {
                install.cover = old.cover.clone();
            }
            lib.installed.insert(id, install);
        }
    })?;
    load()
}

/// Sets or clears (None / empty) the display name of a port.
#[tauri::command]
pub fn rename(id: String, name: Option<String>) -> Result<Library, String> {
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    update(|lib| lib.overrides.entry(id).or_default().name = name)?;
    load()
}

/// Registers a port that is not in the catalog, so it can be added like any other.
/// Returns its id.
#[tauri::command]
pub fn add_custom_port(name: String, console: String) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("give the port a name".into());
    }
    let slug: String = name.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    update(|lib| {
        let mut id = format!("custom-{}", slug.trim_matches('-'));
        while lib.custom.iter().any(|p| p["id"] == id.as_str()) {
            id.push('2');
        }
        lib.custom.push(serde_json::json!({
            "id": id, "name": name, "title": name, "console": console, "kind": "custom", "repo": ""
        }));
        id
    })
}
