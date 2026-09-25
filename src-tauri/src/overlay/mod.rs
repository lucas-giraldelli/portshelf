//! Achievement popups over a running game. PortShelf starts itself again with `--overlay <json>`;
//! in that mode it only shows one card and exits. On Linux the card is a Wayland layer-shell
//! surface on the overlay layer, which compositors draw above fullscreen windows (Hyprland,
//! KDE Plasma, Sway); desktops without layer-shell (GNOME, X11) get no popup.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

mod draw;
#[cfg(target_os = "linux")]
mod wayland;

/// Theme colours the card uses, taken from the palette the shelf is showing.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Colors {
    pub panel: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub on_accent: String,
}

/// One achievement card.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Popup {
    /// "Achievement unlocked", in the shelf's language.
    pub label: String,
    pub title: String,
    pub description: String,
    pub points: u32,
    /// PNG shown in the icon tile; a trophy when absent.
    #[serde(default)]
    pub icon: Option<String>,
    pub colors: Colors,
    /// How long the card stays, in seconds.
    #[serde(default = "default_seconds")]
    pub seconds: f32,
}

fn default_seconds() -> f32 {
    5.0
}

/// Entry point of `portshelf --overlay <json>`. Returns false when the popup could not be shown.
pub fn run(json: &str) -> bool {
    let popup: Popup = match serde_json::from_str(json) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("overlay: {e}");
            return false;
        }
    };
    #[cfg(target_os = "linux")]
    return wayland::show(&popup, std::env::var("PORTSHELF_OVERLAY_OUTPUT").ok().as_deref()).map_err(|e| eprintln!("overlay: {e}")).is_ok();
    #[cfg(not(target_os = "linux"))]
    {
        let _ = popup;
        false
    }
}

/// How cards look right now: the label in the shelf's language and the theme's colours.
/// The interface sends it whenever the theme, the light / dark mode or the language changes.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Style {
    pub label: String,
    pub colors: Colors,
    /// The shelf's language, for the achievements' own texts.
    #[serde(default)]
    pub lang: String,
}

static STYLE: Mutex<Option<Style>> = Mutex::new(None);

#[tauri::command]
pub fn set_overlay_style(style: Style) {
    if let Ok(mut current) = STYLE.lock() {
        *current = Some(style);
    }
}

/// The shelf's language as last sent by the interface.
pub fn language() -> String {
    STYLE.lock().ok().and_then(|s| s.as_ref().map(|s| s.lang.clone())).filter(|l| !l.is_empty()).unwrap_or_else(|| "en".into())
}

/// Shows an achievement card over whatever is on screen, in the current style. The card runs
/// in its own process (PortShelf started again with `--overlay`), so it lives on while the
/// shelf is hidden and never blocks it.
pub fn show(title: &str, description: &str, points: u32, icon: Option<String>) -> Result<(), String> {
    let style = STYLE.lock().ok().and_then(|s| s.clone()).ok_or("the interface has not sent its theme yet")?;
    let popup = Popup {
        label: style.label,
        title: title.to_string(),
        description: description.to_string(),
        points,
        icon,
        colors: style.colors,
        seconds: default_seconds(),
    };
    let json = serde_json::to_string(&popup).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe)
        .arg("--overlay")
        .arg(json)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map(|mut child| {
            // Reaped in the background, so it does not linger as a zombie.
            std::thread::spawn(move || child.wait());
        })
        .map_err(|e| e.to_string())
}

/// A sample card, from the settings dialog, to see the popup in the current theme.
#[tauri::command]
pub fn preview_overlay(title: String, description: String) -> Result<(), String> {
    show(&title, &description, 10, None)
}

/// Renders a card to a PNG, for looking at the design without a compositor.
pub fn render_png(json: &str, path: &str, scale: f32) -> Result<(), String> {
    let popup: Popup = serde_json::from_str(json).map_err(|e| e.to_string())?;
    draw::render(&popup, scale, 1.0).save_png(path).map_err(|e| e.to_string())
}
