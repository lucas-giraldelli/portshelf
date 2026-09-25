//! PortShelf backend. Each module owns one part of the app and its Tauri commands:
//!
//! - `catalog`: the bundled list of ports and what their install sections mean
//! - `library`: the user's library (installed ports, names, ROM folder)
//! - `ports`: installing ports from their releases or adding ones installed by hand
//! - `roms`: game files: status, handing them to ports, the ROM folder
//! - `launch`: starting a port and bringing the shelf back when it exits
//! - `covers`, `port_settings`, `playtime`: cover art, the ports' own settings, time played
//! - `install`, `romscan`, `scrape`, `gamepad`: downloads, game file identification, cover
//!   search and the controller, with no Tauri commands of their own

mod achievements;
mod catalog;
mod covers;
mod gamepad;
mod install;
mod launch;
mod library;
pub mod overlay;
mod paths;
mod playtime;
mod port_settings;
mod ports;
mod roms;
mod romscan;
mod scrape;

/// Closes the shelf (Esc on the systems screen, then Yes).
#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

/// Hides the pointer while a controller is in use (the CSS cursor only updates on the next mouse move).
#[tauri::command]
fn set_cursor_visible(window: tauri::WebviewWindow, visible: bool) -> Result<(), String> {
    window.set_cursor_visible(visible).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's DMA-BUF renderer hits a Wayland protocol error on NVIDIA; the
    // fallback renderer works everywhere. Respect an explicit choice from the user.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let builder = tauri::Builder::default();
    // Opening PortShelf again brings back the running one, even while it waits hidden for a
    // game. Development builds stay separate, so they can run next to the installed app.
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
            let _ = window.show();
            let _ = window.set_focus();
            gamepad::ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }));
    builder
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            quit,
            set_cursor_visible,
            catalog::get_catalog,
            catalog::platform,
            library::get_library,
            library::rescan,
            library::rename,
            library::add_custom_port,
            ports::add_install,
            ports::install_port,
            roms::rom_status,
            roms::select_rom,
            roms::assign_rom,
            roms::set_roms_dir,
            roms::sync_roms,
            launch::launch,
            covers::get_cover,
            covers::set_cover,
            covers::scrape_cover,
            covers::unlock_cover,
            port_settings::get_config,
            port_settings::set_config,
            playtime::get_playtime,
            overlay::set_overlay_style,
            achievements::get_achievements,
            achievements::get_achievement_summary,
        ])
        .on_window_event(|_, event| {
            if let tauri::WindowEvent::Focused(focused) = event {
                gamepad::ACTIVE.store(*focused, std::sync::atomic::Ordering::Relaxed);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
