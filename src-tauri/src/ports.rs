//! Adding ports to the library: downloaded from their release, or pointed at by the user.

use crate::catalog::{self, InstallSpec};
use crate::library::{self, Install, Library};
use crate::paths::{cache_dir, ports_dir};
use crate::roms;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
struct InstallProgress {
    id: String,
    stage: &'static str,
    done: u64,
    total: Option<u64>,
}

/// Records an installed port with its catalog settings, and sets up a game file found for it
/// before it was installed (best effort: if the file moved, the port asks for it again).
fn register(id: &str, spec: InstallSpec, exec: &Path, version: Option<String>) -> Result<Library, String> {
    let cwd = exec.parent().unwrap_or(Path::new("/")).to_path_buf();
    let (config_dir, rom) = spec.layout(&cwd);
    let install = Install {
        exec: exec.to_string_lossy().into_owned(),
        args: spec.args,
        cwd: Some(cwd.to_string_lossy().into_owned()),
        config_dir,
        cover: None,
        boot_setting: spec.boot_setting,
        rom,
        game_file_arg: spec.game_file_arg,
        version,
    };
    let pending = library::update(|lib| {
        lib.installed.insert(id.to_string(), install);
        lib.pending_roms.remove(id)
    })?;
    if let Some(path) = pending {
        let _ = roms::select_rom(id.to_string(), path);
    }
    library::load()
}

/// Adds a port the user installed on their own, pointing at its program. Launch arguments,
/// settings folder and game file handling come from the catalog when it knows the port.
#[tauri::command]
pub fn add_install(id: String, exec: String) -> Result<Library, String> {
    let exec_path = PathBuf::from(&exec);
    if !exec_path.is_file() {
        return Err(format!("{exec} is not a file"));
    }
    register(&id, InstallSpec::of(&catalog::port(&id)?), &exec_path, None)
}

/// Downloads the port's latest release for this system and adds it to the library.
#[tauri::command]
pub async fn install_port(app: tauri::AppHandle, id: String) -> Result<Library, String> {
    use tauri::Emitter;
    let port = catalog::port(&id)?;
    let spec = InstallSpec::of(&port);
    let rule = spec.rule().ok_or("PortShelf does not know how to install this port on this system yet")?;
    let repo = spec.repo.clone().unwrap_or_else(|| port["repo"].as_str().unwrap_or_default().to_string());
    let dest = ports_dir().join(&id);
    let work = cache_dir().join(format!("install-{id}"));
    let (tag, exec) = {
        let (app, id) = (app.clone(), id.clone());
        tauri::async_runtime::spawn_blocking(move || {
            crate::install::install(&repo, &rule, &dest, &work, |p| {
                let (stage, done, total) = match p {
                    crate::install::Progress::Downloading { done, total } => ("downloading", done, total),
                    crate::install::Progress::Unpacking => ("unpacking", 0, None),
                    crate::install::Progress::Done => ("done", 0, None),
                };
                let _ = app.emit("install-progress", InstallProgress { id: id.clone(), stage, done, total });
            })
        })
        .await
        .map_err(|e| e.to_string())??
    };
    register(&id, spec, &exec, Some(tag))
}
