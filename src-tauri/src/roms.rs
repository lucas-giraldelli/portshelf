//! Game files: whether each installed port has what it needs, handing it a file the way its own
//! launcher would, and keeping the ROM folder (`<root>/<system>/<game>/`) in sync.

use crate::catalog;
use crate::library::{self, RomSpec};
use crate::paths::cache_dir;
use crate::port_settings::{self, config_file};
use crate::romscan;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct RomStatus {
    /// True when the port has what it needs to boot.
    pub ready: bool,
    /// The game file in use, when known.
    pub path: Option<String>,
    /// Folder to open the file picker in.
    browse_dir: String,
    /// Release of a game file in the game's folder that the port does not accept
    /// (for example the Rev 1 when it needs the first release).
    wrong: Option<String>,
}

#[derive(Serialize)]
pub struct RomSummary {
    /// Installed ports with their game file in place.
    ready: usize,
    installed: usize,
    /// Game files set up in this sync (installed ports) or kept for later (not installed).
    assigned: usize,
}

/// Game folder of a port inside the ROM folder: <root>/<system>/<game>.
fn port_folder(root: &Path, port: &Value) -> Option<PathBuf> {
    Some(root.join(port["console"].as_str()?).join(romscan::game_folder(catalog::title(port))))
}

/// Game file picked for a port that is not installed yet; used when it gets installed.
fn remember_rom(id: &str, path: String) -> Result<(), String> {
    library::update(|lib| {
        lib.pending_roms.insert(id.to_string(), path);
    })
}

/// A zip or 7z picked as a game file is unpacked into the game's folder in the ROM folder
/// (or PortShelf's cache when there is none); other files are used where they are.
fn unpack_for(id: &str, path: &str) -> Result<String, String> {
    if !romscan::is_archive(Path::new(path)) {
        return Ok(path.to_string());
    }
    let lib = library::load()?;
    let dest = catalog::port(id)
        .ok()
        .filter(|_| !lib.roms_dir.is_empty())
        .and_then(|port| port_folder(Path::new(&lib.roms_dir), &port))
        .unwrap_or_else(|| cache_dir().join("roms").join(id));
    Ok(romscan::unpack_game_file(Path::new(path), &dest)?.to_string_lossy().into_owned())
}

/// Error for a game file of another release than the port accepts; the interface shows it
/// translated: "wrong-version", the file's release and the accepted ones, tab separated.
fn wrong_version(port: &Value, file: &Path) -> String {
    format!("wrong-version\t{}\t{}", romscan::release_label(file, catalog::title(port)), port["rom"]["needs"].as_str().unwrap_or_default())
}

#[tauri::command]
pub fn rom_status(id: String, console: String) -> Result<RomStatus, String> {
    let lib = library::load()?;
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
                entries.flatten().map(|e| e.path()).find(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("z64" | "n64" | "v64")))
            });
            (done || rom.is_some(), rom.map(|p| p.to_string_lossy().into()))
        }
    };
    let wrong = if ready || lib.roms_dir.is_empty() {
        None
    } else {
        catalog::port(&id).ok().and_then(|port| {
            let files = port_folder(Path::new(&lib.roms_dir), &port).map(|f| romscan::files_in(&f)).unwrap_or_default();
            files.iter().find(|f| romscan::accepts(&port, f) == Some(false)).map(|f| romscan::release_label(f, catalog::title(&port)))
        })
    };
    Ok(RomStatus { ready, path, browse_dir, wrong })
}

/// Puts a game file next to a port without copying it where possible: a symbolic link on
/// Unix; on Windows, where symbolic links need extra rights, a hard link (same drive) or a copy.
fn link_file(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(from, to).map_err(|e| e.to_string());
    #[cfg(not(unix))]
    return fs::hard_link(from, to).or_else(|_| fs::copy(from, to).map(|_| ())).map_err(|e| e.to_string());
}

/// Sets up the chosen game file the way the port's own launcher would.
#[tauri::command]
pub fn select_rom(id: String, path: String) -> Result<(), String> {
    let install = library::installed(&id)?;
    let path = unpack_for(&id, &path)?;
    if let Ok(port) = catalog::port(&id) {
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
            port_settings::set_config(id, file, key, Value::String(path))
        }
        RomSpec::Extracted { .. } => {
            // The port extracts it on its next start; a link keeps the ROM in the library.
            let dir = install.cwd.ok_or("no install directory for this port")?;
            let name = Path::new(&path).file_name().ok_or("invalid file")?;
            let link = Path::new(&dir).join(name);
            let _ = fs::remove_file(&link);
            link_file(Path::new(&path), &link)
        }
    }
}

/// Uses a game file the user picked: for the ports whose game folder it sits in, or else
/// for the ports its game code or name identifies. Returns the ids of the ports it went to.
#[tauri::command]
pub fn assign_rom(path: String) -> Result<Vec<String>, String> {
    let lib = library::load()?;
    let catalog = catalog::bundled();
    // An archive is identified by the game file inside it, unpacked to the cache first;
    // select_rom then unpacks it again into the game's own folder.
    let (archive, path) = if romscan::is_archive(Path::new(&path)) {
        let inside = romscan::unpack_game_file(Path::new(&path), &cache_dir().join("unpacked"))?;
        (Some(path), inside.to_string_lossy().into_owned())
    } else {
        (None, path)
    };
    let parent = Path::new(&path).parent().ok_or("invalid file")?.to_path_buf();
    let root = PathBuf::from(&lib.roms_dir);
    let mut ids: Vec<String> = catalog::ports(&catalog)
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
    let picked = archive.unwrap_or(path);
    for id in &ids {
        if lib.installed.contains_key(id) {
            select_rom(id.clone(), picked.clone())?;
        } else {
            remember_rom(id, unpack_for(id, &picked)?)?;
        }
    }
    let _ = fs::remove_dir_all(cache_dir().join("unpacked"));
    Ok(ids)
}

/// Every game of the catalog gets its folder in the ROM folder.
fn create_game_folders(root: &Path, catalog: &Value) -> Result<(), String> {
    for port in catalog::ports(catalog) {
        if let Some(folder) = port_folder(root, port) {
            fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        }
    }
    Ok(())
}

/// Chooses the ROM folder and creates <system>/<game>/ for every game in the catalog.
#[tauri::command]
pub async fn set_roms_dir(dir: String) -> Result<RomSummary, String> {
    create_game_folders(Path::new(&dir), &catalog::bundled())?;
    library::update(|lib| lib.roms_dir = dir)?;
    sync_roms().await
}

/// Gives every port without a game file the one from the ROM folder: a file in the game's
/// own folder first, otherwise an original copy found anywhere in the ROM folder by its
/// game code or name. Ports that already have a game file are left as they are.
#[tauri::command]
pub async fn sync_roms() -> Result<RomSummary, String> {
    let lib = library::load()?;
    if lib.roms_dir.is_empty() {
        return Ok(RomSummary { ready: 0, installed: lib.installed.len(), assigned: 0 });
    }
    let root = PathBuf::from(&lib.roms_dir);
    let catalog = catalog::bundled();
    // Games added to the catalog since the folder was chosen get their folder too.
    let _ = create_game_folders(&root, &catalog);
    let found = {
        let (root, catalog) = (root.clone(), catalog.clone());
        tauri::async_runtime::spawn_blocking(move || romscan::scan(&root, &catalog)).await.map_err(|e| e.to_string())?
    };
    let mut assigned = 0;
    for port in catalog::ports(&catalog) {
        let id = port["id"].as_str().unwrap_or_default().to_string();
        let console = port["console"].as_str().unwrap_or_default().to_string();
        // Archives in the game's folder are unpacked next to themselves; files of another
        // release than the port accepts are left for the user to see.
        let in_folder = port_folder(&root, port)
            .map(|f| {
                romscan::unpack_archives_in(&f);
                romscan::files_in(&f)
            })
            .unwrap_or_default()
            .into_iter()
            .find(|f| romscan::accepts(port, f) != Some(false));
        let matched = found.iter().find(|f| f.port == id && !f.modified && !f.wrong_version).map(|f| PathBuf::from(&f.path));
        let Some(file) = in_folder.or(matched) else { continue };
        let path = file.to_string_lossy().into_owned();
        if library::load()?.installed.contains_key(&id) {
            if !rom_status(id.clone(), console)?.ready && select_rom(id, path).is_ok() {
                assigned += 1;
            }
        } else if library::update(|lib| lib.pending_roms.insert(id, path.clone()) != Some(path))? {
            assigned += 1;
        }
    }
    let lib = library::load()?;
    let ready = lib
        .installed
        .keys()
        .filter(|id| {
            let console = catalog::ports(&catalog).find(|p| p["id"] == id.as_str()).and_then(|p| p["console"].as_str()).unwrap_or_default();
            rom_status((*id).clone(), console.to_string()).is_ok_and(|s| s.ready)
        })
        .count();
    Ok(RomSummary { ready, installed: lib.installed.len(), assigned })
}
