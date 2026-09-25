//! Starting a port: a clean environment, a systemd scope for the time played, and the shelf
//! hidden until the game is gone.

use crate::library;
use crate::paths::app_dir;
use crate::{gamepad, playtime, port_settings, roms};
use std::path::Path;
use std::process::{Command, Stdio};

/// Programs started from PortShelf must not inherit the AppImage's own libraries and data
/// paths (they break file managers and can slow down or crash games).
pub fn clean_command(program: &str) -> Command {
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

/// The same command inside a systemd scope, to know when every process of the game is gone.
/// None where there is no systemd user session.
fn in_scope(cmd: &Command, unit: &str) -> Option<Command> {
    if !cfg!(target_os = "linux") || !Path::new("/run/systemd/system").exists() {
        return None;
    }
    let mut wrapped = clean_command("systemd-run");
    wrapped.args(["--user", "--scope", "--quiet", "--collect", "--unit", unit, "--"]);
    wrapped.arg(cmd.get_program()).args(cmd.get_args());
    if let Some(cwd) = cmd.get_current_dir() {
        wrapped.current_dir(cwd);
    }
    wrapped.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    Some(wrapped)
}

/// Starts the port and hides the shelf while it runs; the shelf comes back when the
/// game exits. The game is its own process, so closing the shelf does not stop it.
#[tauri::command]
pub fn launch(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let install = library::installed(&id)?;
    if let Some(boot) = &install.boot_setting {
        port_settings::set_config(id.clone(), boot.file.clone(), boot.key.clone(), boot.value.clone())?;
    }
    let mut cmd = clean_command(&install.exec);
    cmd.args(&install.args);
    if install.game_file_arg {
        if let Some(path) = roms::rom_status(id.clone(), String::new())?.path {
            cmd.arg(path);
        }
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(cwd) = &install.cwd {
        cmd.current_dir(cwd);
    }
    let scoped = in_scope(&cmd, &playtime::unit_name(&id));
    let session = playtime::start(cmd, scoped).map_err(|e| format!("{}: {e}", install.exec))?;
    // Ports with an achievement set are watched while they run.
    if let Some(set) = crate::achievements::set_for(&id) {
        let pid = session.pid();
        std::thread::spawn(move || crate::achievements::watch(pid, set));
    }
    let window = tauri::Manager::get_webview_window(&app, "main");
    if let Some(w) = &window {
        let _ = w.hide();
    }
    std::thread::spawn(move || {
        session.wait(&app_dir(), &id);
        // Shown first, so the interface can restore fullscreen on a mapped window.
        if let Some(w) = window {
            let _ = w.show();
            let _ = w.set_focus();
            gamepad::ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let _ = tauri::Emitter::emit(&app, "game-exited", &id);
    });
    Ok(())
}
