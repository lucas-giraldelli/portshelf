//! portshelf backend: the port catalog, the user's library (which ports are
//! installed and how to start them), launching, and reading/writing the ports'
//! own config files so the shelf can edit them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const CATALOG: &str = include_str!("../../catalog/ports.json");

/// How to start an installed port and where its config lives.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Install {
    exec: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    config_dir: Option<String>,
    #[serde(default)]
    cover: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Library {
    roms_dir: String,
    installed: BTreeMap<String, Install>,
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

fn expand(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None => PathBuf::from(path),
    }
}

fn app_dir() -> PathBuf {
    home().join(".config/portshelf")
}

fn library_path() -> PathBuf {
    app_dir().join("library.json")
}

/// First run: look for ports in the places they are usually installed.
fn scan() -> Library {
    let mut installed = BTreeMap::new();
    let candidates: &[(&str, &str, &[&str], Option<&str>)] = &[
        ("dk64", "~/Applications/DK64Recompiled/DK64Recompiled", &[], Some("~/.config/DK64Recompiled")),
        ("mt64", "/mnt/main/Roms/mariotennis64recomp/run/play.sh", &[], Some("/mnt/main/Roms/mariotennis64recomp/run")),
        ("tp", "~/Applications/Dusklight.AppImage", &[], Some("~/.local/share/TwilitRealm/Dusklight")),
        ("bm64", "~/Applications/BM64Recompiled/BM64Recompiled", &[], Some("~/.config/BM64Recompiled")),
        // Harbour Masters ports keep their settings next to the AppImage.
        ("oot-soh", "~/Applications/SoH/soh.appimage", &[], Some("~/Applications/SoH")),
        ("mm-2s2h", "~/Applications/2Ship/2ship.appimage", &[], Some("~/Applications/2Ship")),
    ];
    for (id, exec, args, config_dir) in candidates {
        let exec_path = expand(exec);
        if exec_path.exists() {
            installed.insert(
                id.to_string(),
                Install {
                    exec: exec_path.to_string_lossy().into(),
                    args: args.iter().map(|s| s.to_string()).collect(),
                    cwd: exec_path.parent().map(|p| p.to_string_lossy().into()),
                    config_dir: config_dir.map(|d| expand(d).to_string_lossy().into()),
                    cover: None,
                },
            );
        }
    }
    Library { roms_dir: "/mnt/main/Roms/ports/roms".into(), installed }
}

fn load_library() -> Result<Library, String> {
    let path = library_path();
    if let Ok(text) = fs::read_to_string(&path) {
        return serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()));
    }
    let lib = scan();
    save_library(&lib)?;
    Ok(lib)
}

fn save_library(lib: &Library) -> Result<(), String> {
    fs::create_dir_all(app_dir()).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(lib).map_err(|e| e.to_string())?;
    fs::write(library_path(), text).map_err(|e| e.to_string())
}

fn installed(id: &str) -> Result<Install, String> {
    load_library()?.installed.get(id).cloned().ok_or_else(|| format!("{id} is not installed"))
}

#[tauri::command]
fn get_catalog() -> Value {
    serde_json::from_str(CATALOG).expect("catalog/ports.json is valid JSON")
}

#[tauri::command]
fn get_library() -> Result<Library, String> {
    load_library()
}

#[tauri::command]
fn rescan() -> Result<Library, String> {
    let mut lib = load_library()?;
    for (id, install) in scan().installed {
        lib.installed.entry(id).or_insert(install);
    }
    save_library(&lib)?;
    Ok(lib)
}

/// Starts the port detached from portshelf, so closing the shelf keeps the game running.
#[tauri::command]
fn launch(id: String) -> Result<(), String> {
    let install = installed(&id)?;
    let mut cmd = Command::new(&install.exec);
    cmd.args(&install.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(cwd) = &install.cwd {
        cmd.current_dir(cwd);
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("{}: {e}", install.exec))
}

/// Cover art as a data URL: the library's explicit cover, or covers/<id>.{png,jpg,webp}.
#[tauri::command]
fn get_cover(id: String) -> Option<String> {
    let lib = load_library().ok()?;
    let explicit = lib.installed.get(&id).and_then(|i| i.cover.clone()).map(|c| expand(&c));
    let candidates = explicit.into_iter().chain(
        ["png", "jpg", "jpeg", "webp"].iter().map(|ext| app_dir().join("covers").join(format!("{id}.{ext}"))),
    );
    for path in candidates {
        if let Ok(bytes) = fs::read(&path) {
            let mime = match path.extension().and_then(|e| e.to_str()) {
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("webp") => "image/webp",
                _ => "image/png",
            };
            use base64::Engine;
            return Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)));
        }
    }
    None
}

/// RecompFrontend ports keep one JSON object per settings tab (general.json,
/// graphics.json, sound.json, ...). Returns them keyed by file stem.
#[tauri::command]
fn get_config(id: String) -> Result<BTreeMap<String, Value>, String> {
    let install = installed(&id)?;
    let dir = install.config_dir.ok_or("no config directory for this port")?;
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("{dir}: {e}"))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let is_json = path.extension().and_then(|e| e.to_str()) == Some("json");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        // controls.json is a large binding table; it gets its own editor later.
        if !is_json || stem == "controls" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap_or_default()) {
            if value.is_object() {
                out.insert(stem, value);
            }
        }
    }
    Ok(out)
}

/// Writes one option back, keeping a .bak of the previous file like the ports do.
#[tauri::command]
fn set_config(id: String, file: String, key: String, value: Value) -> Result<(), String> {
    let install = installed(&id)?;
    let dir = install.config_dir.ok_or("no config directory for this port")?;
    if file.contains('/') || file.contains("..") {
        return Err("invalid config file name".into());
    }
    let path = Path::new(&dir).join(format!("{file}.json"));
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    json.as_object_mut().ok_or("config file is not an object")?.insert(key, value);
    fs::write(path.with_extension("json.bak"), text).map_err(|e| e.to_string())?;
    fs::write(&path, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_catalog, get_library, rescan, launch, get_cover, get_config, set_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
