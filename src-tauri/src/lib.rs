//! portshelf backend: the port catalog, the user's library (which ports are
//! installed and how to start them), launching, and reading/writing the ports'
//! own config files so the shelf can edit them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod scrape;

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
    /// A setting written to the port's config right before launching it, used
    /// to skip its own launcher (for ports without a command line flag for it).
    #[serde(default)]
    boot_setting: Option<BootSetting>,
    /// Where the port expects its game data; the shelf only starts a port once this is set up.
    #[serde(default)]
    rom: Option<RomSpec>,
}

/// How each kind of port stores the game it needs.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RomSpec {
    /// RecompFrontend ports: a big-endian copy named <GAME_ID>.z64 in the config directory.
    Stored { file: String },
    /// A path saved in one of the port's JSON settings files (Dusklight's backend.isoPath).
    ConfigKey { file: String, key: String },
    /// Harbour Masters ports: they turn a ROM found next to the executable into an .o2r archive.
    Extracted { archive: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BootSetting {
    file: String,
    key: String,
    value: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Library {
    roms_dir: String,
    installed: BTreeMap<String, Install>,
    /// Per-port choices the user made on the shelf, for installed and catalog-only ports alike.
    #[serde(default)]
    overrides: BTreeMap<String, Override>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Override {
    /// Display name instead of the catalog's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// The cover was picked by hand: automatic scraping leaves it alone.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    cover_locked: bool,
}

fn cache_dir() -> PathBuf {
    home().join(".cache/portshelf")
}

fn covers_dir() -> PathBuf {
    app_dir().join("covers")
}

/// Existing cover files for a port (any extension).
fn cover_files(id: &str) -> Vec<PathBuf> {
    ["png", "jpg", "jpeg", "webp"].iter().map(|ext| covers_dir().join(format!("{id}.{ext}"))).filter(|p| p.exists()).collect()
}

/// Moves a port's current cover files aside (covers/replaced/) before a new one is written.
fn retire_covers(id: &str) -> Result<(), String> {
    let replaced = covers_dir().join("replaced");
    for file in cover_files(id) {
        fs::create_dir_all(&replaced).map_err(|e| e.to_string())?;
        let name = file.file_name().unwrap_or_default();
        fs::rename(&file, replaced.join(name)).map_err(|e| e.to_string())?;
    }
    Ok(())
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
    // Recomp ports skip their launcher with --game <id>; Dusklight with a config flag.
    let candidates: &[(&str, &str, &[&str], Option<&str>)] = &[
        ("dk64", "~/Applications/DK64Recompiled/DK64Recompiled", &["--game", "dk64"], Some("~/.config/DK64Recompiled")),
        ("mt64", "/mnt/main/Roms/mariotennis64recomp/run/play.sh", &["--game", "mt64"], Some("/mnt/main/Roms/mariotennis64recomp/run")),
        ("tp", "~/Applications/Dusklight.AppImage", &[], Some("~/.local/share/TwilitRealm/Dusklight")),
        ("bm64", "~/Applications/BM64Recompiled/BM64Recompiled", &["--game", "bm64"], Some("~/.config/BM64Recompiled")),
        // Harbour Masters ports boot straight into the game and keep their settings next to the AppImage.
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
                    boot_setting: (*id == "tp").then(|| BootSetting {
                        file: "config".into(),
                        key: "backend.skipPreLaunchUI".into(),
                        value: Value::Bool(true),
                    }),
                    rom: Some(match *id {
                        "tp" => RomSpec::ConfigKey { file: "config".into(), key: "backend.isoPath".into() },
                        "oot-soh" => RomSpec::Extracted { archive: "oot.o2r".into() },
                        "mm-2s2h" => RomSpec::Extracted { archive: "mm.o2r".into() },
                        other => RomSpec::Stored { file: format!("{}.z64", other.to_uppercase()) },
                    }),
                },
            );
        }
    }
    Library { roms_dir: "/mnt/main/Roms/ports/roms".into(), installed, overrides: BTreeMap::new() }
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
    // Scanned entries are refreshed (launch flags, game file layout), but a cover the
    // user picked is kept; ports added by hand are left alone.
    let mut lib = load_library()?;
    for (id, mut install) in scan().installed {
        if let Some(old) = lib.installed.get(&id) {
            install.cover = old.cover.clone();
        }
        lib.installed.insert(id, install);
    }
    save_library(&lib)?;
    Ok(lib)
}

/// Starts the port detached from portshelf, so closing the shelf keeps the game running.
#[tauri::command]
fn launch(id: String) -> Result<(), String> {
    let install = installed(&id)?;
    if let Some(boot) = &install.boot_setting {
        set_config(id.clone(), boot.file.clone(), boot.key.clone(), boot.value.clone())?;
    }
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
        // Binding tables and bookkeeping files are not settings.
        if !is_json || matches!(stem.as_str(), "controls" | "achievements" | "mods") {
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

/// Sets or clears (None / empty) the display name of a port.
#[tauri::command]
fn rename(id: String, name: Option<String>) -> Result<Library, String> {
    let mut lib = load_library()?;
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    lib.overrides.entry(id).or_default().name = name;
    save_library(&lib)?;
    Ok(lib)
}

/// Uses an image file the user picked as the cover and stops automatic scraping for it.
#[tauri::command]
fn set_cover(id: String, path: String) -> Result<(), String> {
    let src = Path::new(&path);
    let ext = src.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return Err("pick a PNG, JPEG or WebP image".into());
    }
    retire_covers(&id)?;
    fs::create_dir_all(covers_dir()).map_err(|e| e.to_string())?;
    fs::copy(src, covers_dir().join(format!("{id}.{ext}"))).map_err(|e| format!("{path}: {e}"))?;
    let mut lib = load_library()?;
    lib.overrides.entry(id).or_default().cover_locked = true;
    save_library(&lib)
}

/// Finds box art for `title` on libretro-thumbnails. Without `force` it only fills in
/// missing covers; with `force` it replaces the current one unless the user picked it.
#[tauri::command]
async fn scrape_cover(id: String, console: String, title: String, force: bool) -> Result<Option<String>, String> {
    let lib = load_library()?;
    let locked = lib.overrides.get(&id).is_some_and(|o| o.cover_locked);
    let has_cover = !cover_files(&id).is_empty();
    if locked || (has_cover && !force) {
        return Ok(None);
    }
    let tmp = cache_dir().join(format!("{id}.download.png"));
    let file = tauri::async_runtime::spawn_blocking(move || scrape::fetch_cover(&console, &title, &cache_dir(), &tmp).map(|f| (f, tmp)))
        .await
        .map_err(|e| e.to_string())??;
    retire_covers(&id)?;
    fs::create_dir_all(covers_dir()).map_err(|e| e.to_string())?;
    fs::rename(&file.1, covers_dir().join(format!("{id}.png"))).map_err(|e| e.to_string())?;
    Ok(Some(file.0))
}

/// Forget a hand-picked cover so scraping can replace it again.
#[tauri::command]
fn unlock_cover(id: String) -> Result<(), String> {
    let mut lib = load_library()?;
    lib.overrides.entry(id).or_default().cover_locked = false;
    save_library(&lib)
}

#[derive(Serialize)]
struct RomStatus {
    /// True when the port has what it needs to boot.
    ready: bool,
    /// The game file in use, when known.
    path: Option<String>,
    /// Folder to open the file picker in.
    browse_dir: String,
}

fn config_file(install: &Install, file: &str) -> Option<PathBuf> {
    install.config_dir.as_ref().map(|d| Path::new(d).join(format!("{file}.json")))
}

#[tauri::command]
fn rom_status(id: String, console: String) -> Result<RomStatus, String> {
    let lib = load_library()?;
    let install = lib.installed.get(&id).ok_or_else(|| format!("{id} is not installed"))?;
    let browse_dir = Path::new(&lib.roms_dir).join(&console).to_string_lossy().into();
    let (ready, path) = match &install.rom {
        None => (true, None),
        Some(RomSpec::Stored { file }) => {
            let p = Path::new(install.config_dir.as_deref().unwrap_or_default()).join(file);
            (p.exists(), p.exists().then(|| p.to_string_lossy().into()))
        }
        Some(RomSpec::ConfigKey { file, key }) => {
            let value = config_file(install, file)
                .and_then(|p| fs::read_to_string(p).ok())
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| v.get(key).and_then(|v| v.as_str()).map(String::from));
            (value.as_deref().is_some_and(|p| Path::new(p).exists()), value)
        }
        Some(RomSpec::Extracted { archive }) => {
            let dir = Path::new(install.cwd.as_deref().unwrap_or_default());
            let done = dir.join(archive).exists();
            let rom = fs::read_dir(dir).ok().and_then(|entries| {
                entries.flatten().map(|e| e.path()).find(|p| {
                    matches!(p.extension().and_then(|e| e.to_str()), Some("z64" | "n64" | "v64"))
                })
            });
            (done || rom.is_some(), rom.map(|p| p.to_string_lossy().into()))
        }
    };
    Ok(RomStatus { ready, path, browse_dir })
}

/// N64 ROMs come in three byte orders; RecompFrontend stores the big-endian (.z64) one.
fn to_z64(mut data: Vec<u8>) -> Result<Vec<u8>, String> {
    match data.get(..4) {
        Some([0x80, 0x37, 0x12, 0x40]) => {}
        Some([0x37, 0x80, 0x40, 0x12]) => data.chunks_exact_mut(2).for_each(|c| c.swap(0, 1)),
        Some([0x40, 0x12, 0x37, 0x80]) => data.chunks_exact_mut(4).for_each(|c| c.reverse()),
        _ => return Err("this file is not an N64 ROM".into()),
    }
    Ok(data)
}

/// Sets up the chosen game file the way the port's own launcher would.
#[tauri::command]
fn select_rom(id: String, path: String) -> Result<(), String> {
    let install = installed(&id)?;
    match install.rom.ok_or("this port does not need a game file")? {
        RomSpec::Stored { file } => {
            let dir = install.config_dir.ok_or("no config directory for this port")?;
            let data = to_z64(fs::read(&path).map_err(|e| format!("{path}: {e}"))?)?;
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            fs::write(Path::new(&dir).join(file), data).map_err(|e| e.to_string())
        }
        RomSpec::ConfigKey { file, key } => {
            let dir = install.config_dir.ok_or("no config directory for this port")?;
            let target = Path::new(&dir).join(format!("{file}.json"));
            if !target.exists() {
                fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                fs::write(&target, "{}").map_err(|e| e.to_string())?;
            }
            set_config(id, file, key, Value::String(path))
        }
        RomSpec::Extracted { .. } => {
            // The port extracts it on its next start; a link keeps the ROM in the library.
            let dir = install.cwd.ok_or("no install directory for this port")?;
            let name = Path::new(&path).file_name().ok_or("invalid file")?;
            let link = Path::new(&dir).join(name);
            let _ = fs::remove_file(&link);
            std::os::unix::fs::symlink(&path, &link).map_err(|e| e.to_string())
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Tiling compositors draw no title bar, so GTK adds its own buttons; drop them on
            // Linux. Windows and macOS keep their native title bar.
            #[cfg(target_os = "linux")]
            if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
                window.set_decorations(false)?;
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_catalog, get_library, rescan, launch, get_cover, get_config, set_config, rom_status, select_rom, rename, set_cover, scrape_cover, unlock_cover])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
