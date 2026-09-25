//! Cover art: files in `~/.config/portshelf/covers/<id>.<ext>`, found online or picked by hand.

use crate::library;
use crate::paths::{cache_dir, covers_dir, expand};
use std::fs;
use std::path::{Path, PathBuf};

const EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// Existing cover files for a port (any extension).
fn cover_files(id: &str) -> Vec<PathBuf> {
    EXTENSIONS.iter().map(|ext| covers_dir().join(format!("{id}.{ext}"))).filter(|p| p.exists()).collect()
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

/// Cover art as a data URL: the library's explicit cover, or covers/<id>.{png,jpg,webp}.
#[tauri::command]
pub fn get_cover(id: String) -> Option<String> {
    let lib = library::load().ok()?;
    let explicit = lib.installed.get(&id).and_then(|i| i.cover.clone()).map(|c| expand(&c));
    let candidates = explicit.into_iter().chain(EXTENSIONS.iter().map(|ext| covers_dir().join(format!("{id}.{ext}"))));
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

/// Uses an image file the user picked as the cover and stops automatic scraping for it.
#[tauri::command]
pub fn set_cover(id: String, path: String) -> Result<(), String> {
    let src = Path::new(&path);
    let ext = src.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
    if !EXTENSIONS.contains(&ext.as_str()) {
        return Err("pick a PNG, JPEG or WebP image".into());
    }
    retire_covers(&id)?;
    fs::create_dir_all(covers_dir()).map_err(|e| e.to_string())?;
    fs::copy(src, covers_dir().join(format!("{id}.{ext}"))).map_err(|e| format!("{path}: {e}"))?;
    library::update(|lib| lib.overrides.entry(id).or_default().cover_locked = true)
}

/// Finds box art for `title` on libretro-thumbnails. Without `force` it only fills in
/// missing covers; with `force` it replaces the current one unless the user picked it.
#[tauri::command]
pub async fn scrape_cover(id: String, console: String, title: String, force: bool) -> Result<Option<String>, String> {
    let lib = library::load()?;
    let locked = lib.overrides.get(&id).is_some_and(|o| o.cover_locked);
    if locked || (!cover_files(&id).is_empty() && !force) {
        return Ok(None);
    }
    let tmp = cache_dir().join(format!("{id}.download.png"));
    let (found, file) = tauri::async_runtime::spawn_blocking(move || crate::scrape::fetch_cover(&console, &title, &cache_dir(), &tmp).map(|f| (f, tmp)))
        .await
        .map_err(|e| e.to_string())??;
    retire_covers(&id)?;
    fs::create_dir_all(covers_dir()).map_err(|e| e.to_string())?;
    fs::rename(&file, covers_dir().join(format!("{id}.png"))).map_err(|e| e.to_string())?;
    Ok(Some(found))
}

/// Forgets a hand-picked cover so scraping can replace it again.
#[tauri::command]
pub fn unlock_cover(id: String) -> Result<(), String> {
    library::update(|lib| lib.overrides.entry(id).or_default().cover_locked = false)
}
