//! PortShelf backend: the port catalog, the user's library (which ports are
//! installed and how to start them), launching, and reading/writing the ports'
//! own config files so the shelf can edit them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod gamepad;
mod install;
mod romscan;
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
    /// The port boots straight into the game when given its game file on the command line.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    game_file_arg: bool,
    /// Release tag, for ports installed by PortShelf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
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
    /// Ports added by the user that the catalog does not know (same shape as catalog entries).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    custom: Vec<Value>,
    /// Game files found for ports that are not installed yet.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pending_roms: BTreeMap<String, String>,
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
        ("bm64", "~/Applications/BM64Recompiled/BM64Recompiled", &["--game", "bm64_us"], Some("~/.config/BM64Recompiled")),
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
                    boot_setting: None,
                    game_file_arg: *id == "tp",
                    version: None,
                    rom: Some(match *id {
                        "tp" => RomSpec::ConfigKey { file: "config".into(), key: "backend.isoPath".into() },
                        "oot-soh" => RomSpec::Extracted { archive: "oot.o2r".into() },
                        "mm-2s2h" => RomSpec::Extracted { archive: "mm.o2r".into() },
                        // Recomps store the ROM as <game id>.z64.
                        "bm64" => RomSpec::Stored { file: "bm64_us.z64".into() },
                        other => RomSpec::Stored { file: format!("{}.z64", other.to_uppercase()) },
                    }),
                },
            );
        }
    }
    Library { roms_dir: String::new(), installed, overrides: BTreeMap::new(), custom: Vec::new(), pending_roms: BTreeMap::new() }
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
    full_catalog()
}

/// The bundled catalog plus the ports the user added themselves.
fn full_catalog() -> Value {
    let mut catalog: Value = serde_json::from_str(CATALOG).expect("catalog/ports.json is valid JSON");
    if let (Ok(lib), Some(ports)) = (load_library(), catalog["ports"].as_array_mut()) {
        ports.extend(lib.custom);
    }
    catalog
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

/// Programs started from PortShelf must not inherit the AppImage's own libraries and data
/// paths (they break file managers and can slow down or crash games).
fn clean_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    if std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some() {
        let appdir = std::env::var("APPDIR").unwrap_or_default();
        for var in [
            "LD_LIBRARY_PATH", "LD_PRELOAD", "GIO_EXTRA_MODULES", "GIO_MODULE_DIR", "GTK_PATH", "GTK_EXE_PREFIX",
            "GTK_DATA_PREFIX", "GTK_THEME", "GDK_PIXBUF_MODULE_FILE", "GDK_PIXBUF_MODULEDIR", "GDK_BACKEND",
            "GSETTINGS_SCHEMA_DIR", "GST_PLUGIN_PATH", "GST_PLUGIN_SYSTEM_PATH", "GST_PLUGIN_SCANNER",
            "GST_REGISTRY_REUSE_PLUGIN_SCANNER", "PYTHONHOME", "PYTHONPATH", "PERLLIB", "QT_PLUGIN_PATH",
            "WEBKIT_DISABLE_DMABUF_RENDERER", "APPDIR", "APPIMAGE", "ARGV0", "OWD",
        ] {
            cmd.env_remove(var);
        }
        // Keep the system's entries of path lists, drop the ones inside the AppImage.
        for var in ["PATH", "XDG_DATA_DIRS"] {
            if let Ok(value) = std::env::var(var) {
                let kept: Vec<&str> = value.split(':').filter(|p| !p.is_empty() && (appdir.is_empty() || !p.starts_with(&appdir))).collect();
                cmd.env(var, kept.join(":"));
            }
        }
    }
    cmd
}

/// Starts the port and hides the shelf while it runs; the shelf comes back when the
/// game exits. The game is its own process, so closing the shelf does not stop it.
#[tauri::command]
fn launch(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let install = installed(&id)?;
    if let Some(boot) = &install.boot_setting {
        set_config(id.clone(), boot.file.clone(), boot.key.clone(), boot.value.clone())?;
    }
    let mut cmd = clean_command(&install.exec);
    cmd.args(&install.args);
    if install.game_file_arg {
        let status = rom_status(id.clone(), String::new())?;
        if let Some(path) = status.path {
            cmd.arg(path);
        }
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(cwd) = &install.cwd {
        cmd.current_dir(cwd);
    }
    let mut child = cmd.spawn().map_err(|e| format!("{}: {e}", install.exec))?;
    let window = tauri::Manager::get_webview_window(&app, "main");
    if let Some(w) = &window {
        let _ = w.hide();
    }
    std::thread::spawn(move || {
        let _ = child.wait();
        if let Some(w) = window {
            let _ = w.show();
            let _ = w.set_focus();
            gamepad::ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });
    Ok(())
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

/// Closes the shelf (Esc twice on the systems screen).
#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

/// Catalog install section of a port.
#[derive(Deserialize, Default)]
struct CatalogInstall {
    /// Where the releases come from, when not the project's own repository
    /// (OpenGOAL: the launcher, not the compiler tools).
    repo: Option<String>,
    linux: Option<install::Rule>,
    windows: Option<install::Rule>,
    macos: Option<install::Rule>,
    #[serde(default)]
    args: Vec<String>,
    config_dir: Option<String>,
    /// Harbour Masters ports keep settings next to the program.
    #[serde(default)]
    config_in_install: bool,
    rom: Option<RomSpec>,
    boot_setting: Option<BootSetting>,
    #[serde(default)]
    game_file_arg: bool,
}

fn catalog_port(id: &str) -> Result<Value, String> {
    let catalog = full_catalog();
    catalog["ports"]
        .as_array()
        .and_then(|ports| ports.iter().find(|p| p["id"] == id))
        .cloned()
        .ok_or_else(|| format!("{id} is not in the catalog"))
}

fn ports_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| home().join(".local/share")).join("PortShelf").join("ports")
}

#[derive(Serialize, Clone)]
struct InstallProgress {
    id: String,
    stage: &'static str,
    done: u64,
    total: Option<u64>,
}

#[derive(Serialize)]
struct RomSummary {
    /// Installed ports with their game file in place.
    ready: usize,
    installed: usize,
    /// Game files set up in this sync (installed ports) or kept for later (not installed).
    assigned: usize,
}

/// Game folder of a port inside the ROM folder: <root>/<system>/<game>.
fn port_folder(root: &Path, port: &Value) -> Option<PathBuf> {
    let title = port["title"].as_str().or(port["name"].as_str())?;
    Some(root.join(port["console"].as_str()?).join(romscan::game_folder(title)))
}

/// Game file picked for a port that is not installed yet; used when it gets installed.
#[tauri::command]
fn remember_rom(id: String, path: String) -> Result<(), String> {
    let mut lib = load_library()?;
    lib.pending_roms.insert(id, path);
    save_library(&lib)
}

/// Uses a game file the user picked: for the ports whose game folder it sits in, or else
/// for the ports its game code or name identifies. Returns the ids of the ports it went to.
#[tauri::command]
fn assign_rom(path: String) -> Result<Vec<String>, String> {
    let lib = load_library()?;
    let catalog: Value = serde_json::from_str(CATALOG).map_err(|e| e.to_string())?;
    let file = PathBuf::from(&path);
    let parent = file.parent().ok_or("invalid file")?.to_path_buf();
    let root = PathBuf::from(&lib.roms_dir);
    let mut ids: Vec<String> = catalog["ports"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| !lib.roms_dir.is_empty() && port_folder(&root, p).as_deref() == Some(parent.as_path()))
        .filter_map(|p| p["id"].as_str().map(String::from))
        .collect();
    if ids.is_empty() {
        ids = romscan::scan(&parent, &catalog).into_iter().filter(|f| f.path == path).map(|f| f.port).collect();
        ids.dedup();
    }
    if ids.is_empty() {
        return Err("PortShelf could not tell which game this file is. Put it in the game's own folder.".into());
    }
    for id in &ids {
        if lib.installed.contains_key(id) {
            select_rom(id.clone(), path.clone())?;
        } else {
            remember_rom(id.clone(), path.clone())?;
        }
    }
    Ok(ids)
}

/// Chooses the ROM folder and creates <system>/<game>/ for every game in the catalog.
#[tauri::command]
async fn set_roms_dir(dir: String) -> Result<RomSummary, String> {
    let catalog: Value = serde_json::from_str(CATALOG).map_err(|e| e.to_string())?;
    let root = PathBuf::from(&dir);
    for port in catalog["ports"].as_array().into_iter().flatten() {
        if let Some(folder) = port_folder(&root, port) {
            fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        }
    }
    let mut lib = load_library()?;
    lib.roms_dir = dir;
    save_library(&lib)?;
    sync_roms().await
}

/// Gives every port without a game file the one from the ROM folder: a file in the game's
/// own folder first, otherwise an original copy found anywhere in the ROM folder by its
/// game code or name. Ports that already have a game file are left as they are.
#[tauri::command]
async fn sync_roms() -> Result<RomSummary, String> {
    let lib = load_library()?;
    if lib.roms_dir.is_empty() {
        return Ok(RomSummary { ready: 0, installed: lib.installed.len(), assigned: 0 });
    }
    let root = PathBuf::from(&lib.roms_dir);
    let catalog: Value = serde_json::from_str(CATALOG).map_err(|e| e.to_string())?;
    // Games added to the catalog since the folder was chosen get their folder too.
    for port in catalog["ports"].as_array().into_iter().flatten() {
        if let Some(folder) = port_folder(&root, port) {
            let _ = fs::create_dir_all(folder);
        }
    }
    let found = {
        let (root, catalog) = (root.clone(), catalog.clone());
        tauri::async_runtime::spawn_blocking(move || romscan::scan(&root, &catalog)).await.map_err(|e| e.to_string())?
    };
    let mut assigned = 0;
    for port in catalog["ports"].as_array().into_iter().flatten() {
        let id = port["id"].as_str().unwrap_or_default().to_string();
        let console = port["console"].as_str().unwrap_or_default().to_string();
        // Files of another release than the port accepts are left for the user to see.
        let in_folder = port_folder(&root, port)
            .map(|f| romscan::files_in(&f))
            .unwrap_or_default()
            .into_iter()
            .find(|f| romscan::accepts(port, f) != Some(false));
        let matched = found.iter().find(|f| f.port == id && !f.modified && !f.wrong_version).map(|f| PathBuf::from(&f.path));
        let Some(file) = in_folder.or(matched) else { continue };
        let path = file.to_string_lossy().into_owned();
        if load_library()?.installed.contains_key(&id) {
            if !rom_status(id.clone(), console)?.ready && select_rom(id, path).is_ok() {
                assigned += 1;
            }
        } else {
            let mut lib = load_library()?;
            if lib.pending_roms.get(&id) != Some(&path) {
                lib.pending_roms.insert(id, path);
                save_library(&lib)?;
                assigned += 1;
            }
        }
    }
    let lib = load_library()?;
    let ready = lib
        .installed
        .keys()
        .filter(|id| {
            let console = catalog["ports"].as_array().and_then(|ps| ps.iter().find(|p| p["id"] == id.as_str())).and_then(|p| p["console"].as_str()).unwrap_or_default();
            rom_status((*id).clone(), console.to_string()).is_ok_and(|s| s.ready)
        })
        .count();
    Ok(RomSummary { ready, installed: lib.installed.len(), assigned })
}

/// Registers a port that is not in the catalog, so it can be added like any other.
/// Returns its id.
#[tauri::command]
fn add_custom_port(name: String, console: String) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("give the port a name".into());
    }
    let slug: String = name.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    let mut lib = load_library()?;
    let mut id = format!("custom-{}", slug.trim_matches('-'));
    while lib.custom.iter().any(|p| p["id"] == id.as_str()) {
        id.push('2');
    }
    lib.custom.push(serde_json::json!({
        "id": id, "name": name, "title": name, "console": console, "kind": "custom", "repo": ""
    }));
    save_library(&lib)?;
    Ok(id)
}

/// Adds a port the user installed on their own, pointing at its program. Launch arguments,
/// settings folder and game file handling come from the catalog when it knows the port.
#[tauri::command]
fn add_install(id: String, exec: String) -> Result<Library, String> {
    let port = catalog_port(&id)?;
    let spec: CatalogInstall = serde_json::from_value(port["install"].clone()).unwrap_or_default();
    let exec_path = PathBuf::from(&exec);
    if !exec_path.is_file() {
        return Err(format!("{exec} is not a file"));
    }
    let cwd = exec_path.parent().unwrap_or(Path::new("/")).to_path_buf();
    let config_dir = if spec.config_in_install {
        Some(cwd.to_string_lossy().into_owned())
    } else {
        spec.config_dir.as_deref().map(|d| expand(d).to_string_lossy().into_owned())
    };
    let mut lib = load_library()?;
    lib.installed.insert(
        id.clone(),
        Install {
            exec,
            args: spec.args,
            cwd: Some(cwd.to_string_lossy().into_owned()),
            config_dir,
            cover: None,
            boot_setting: spec.boot_setting,
            rom: spec.rom,
            game_file_arg: spec.game_file_arg,
            version: None,
        },
    );
    let pending = lib.pending_roms.remove(&id);
    save_library(&lib)?;
    if let Some(path) = pending {
        let _ = select_rom(id, path);
    }
    load_library()
}

/// The operating system, for knowing which catalog install rule applies.
#[tauri::command]
fn platform() -> &'static str {
    std::env::consts::OS
}

/// Downloads the port's latest release for this system and adds it to the library.
#[tauri::command]
async fn install_port(app: tauri::AppHandle, id: String) -> Result<Library, String> {
    use tauri::Emitter;
    let port = catalog_port(&id)?;
    let spec: CatalogInstall = serde_json::from_value(port["install"].clone()).unwrap_or_default();
    let rule = match std::env::consts::OS {
        "linux" => spec.linux.clone(),
        "windows" => spec.windows.clone(),
        "macos" => spec.macos.clone(),
        _ => None,
    }
    .ok_or("PortShelf does not know how to install this port on this system yet")?;
    let repo = spec.repo.clone().unwrap_or_else(|| port["repo"].as_str().unwrap_or_default().to_string());
    let dest = ports_dir().join(&id);
    let work = cache_dir().join(format!("install-{id}"));
    let (tag, exec) = {
        let (app, id, dest) = (app.clone(), id.clone(), dest.clone());
        tauri::async_runtime::spawn_blocking(move || {
            install::install(&repo, &rule, &dest, &work, |p| {
                let (stage, done, total) = match p {
                    install::Progress::Downloading { done, total } => ("downloading", done, total),
                    install::Progress::Unpacking => ("unpacking", 0, None),
                    install::Progress::Done => ("done", 0, None),
                };
                let _ = app.emit("install-progress", InstallProgress { id: id.clone(), stage, done, total });
            })
        })
        .await
        .map_err(|e| e.to_string())??
    };
    let cwd = exec.parent().unwrap_or(&dest).to_path_buf();
    let config_dir = if spec.config_in_install {
        Some(cwd.to_string_lossy().into_owned())
    } else {
        spec.config_dir.as_deref().map(|d| expand(d).to_string_lossy().into_owned())
    };
    let mut lib = load_library()?;
    lib.installed.insert(
        id.clone(),
        Install {
            exec: exec.to_string_lossy().into_owned(),
            args: spec.args,
            cwd: Some(cwd.to_string_lossy().into_owned()),
            config_dir,
            cover: None,
            boot_setting: spec.boot_setting,
            rom: spec.rom,
            game_file_arg: spec.game_file_arg,
            version: Some(tag),
        },
    );
    let pending = lib.pending_roms.remove(&id);
    save_library(&lib)?;
    if let Some(path) = pending {
        // Best effort: if the file moved, the port simply asks for it again.
        let _ = select_rom(id, path);
    }
    load_library()
}

/// Hides the pointer while a controller is in use (the CSS cursor only updates on the next mouse move).
#[tauri::command]
fn set_cursor_visible(window: tauri::WebviewWindow, visible: bool) -> Result<(), String> {
    window.set_cursor_visible(visible).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct RomStatus {
    /// True when the port has what it needs to boot.
    ready: bool,
    /// The game file in use, when known.
    path: Option<String>,
    /// Folder to open the file picker in.
    browse_dir: String,
    /// Release of a game file in the game's folder that the port does not accept
    /// (for example the Rev 1 when it needs the first release).
    wrong: Option<String>,
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
    let wrong = if ready || lib.roms_dir.is_empty() {
        None
    } else {
        catalog_port(&id).ok().and_then(|port| {
            let title = port["title"].as_str().or(port["name"].as_str()).unwrap_or_default().to_string();
            let files = port_folder(Path::new(&lib.roms_dir), &port).map(|f| romscan::files_in(&f)).unwrap_or_default();
            files.iter().find(|f| romscan::accepts(&port, f) == Some(false)).map(|f| romscan::release_label(f, &title))
        })
    };
    Ok(RomStatus { ready, path, browse_dir, wrong })
}

/// Error for a game file of another release than the port accepts; the interface shows it
/// translated: "wrong-version", the file's release and the accepted ones, tab separated.
fn wrong_version(port: &Value, file: &Path) -> String {
    let title = port["title"].as_str().or(port["name"].as_str()).unwrap_or_default();
    format!("wrong-version\t{}\t{}", romscan::release_label(file, title), port["rom"]["needs"].as_str().unwrap_or_default())
}

/// Sets up the chosen game file the way the port's own launcher would.
#[tauri::command]
fn select_rom(id: String, path: String) -> Result<(), String> {
    let install = installed(&id)?;
    if let Ok(port) = catalog_port(&id) {
        if romscan::accepts(&port, Path::new(&path)) == Some(false) {
            return Err(wrong_version(&port, Path::new(&path)));
        }
    }
    match install.rom.ok_or("this port does not need a game file")? {
        RomSpec::Stored { file } => {
            let dir = install.config_dir.ok_or("no config directory for this port")?;
            let data = romscan::to_z64(fs::read(&path).map_err(|e| format!("{path}: {e}"))?)?;
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
    // WebKitGTK's DMA-BUF renderer hits a Wayland protocol error on NVIDIA; the
    // fallback renderer works everywhere. Respect an explicit choice from the user.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            gamepad::spawn(app.handle().clone());
            // Tiling compositors draw no title bar, so GTK adds its own buttons; drop them on
            // Linux. Windows and macOS keep their native title bar.
            if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
                #[cfg(target_os = "linux")]
                window.set_decorations(false)?;
                // Development builds are told apart by the title (a compositor rule keeps them aside).
                #[cfg(debug_assertions)]
                window.set_title("PortShelf (dev)")?;
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_catalog, get_library, rescan, launch, get_cover, get_config, set_config, rom_status, select_rom, rename, set_cover, scrape_cover, unlock_cover, set_cursor_visible, quit, install_port, platform, set_roms_dir, sync_roms, remember_rom, assign_rom, add_install, add_custom_port])
        .on_window_event(|_, event| {
            if let tauri::WindowEvent::Focused(focused) = event {
                gamepad::ACTIVE.store(*focused, std::sync::atomic::Ordering::Relaxed);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
