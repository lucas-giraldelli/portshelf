//! Keeps the catalog and the achievement sets up to date without a new release: on start, the
//! newest `catalog/` files are downloaded from the project's repository and kept in
//! `~/.cache/portshelf/catalog/`. A file is only kept when this build can read it; otherwise the
//! previous copy, or the one bundled with the program, stays in use.

use crate::paths::cache_dir;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::Emitter;

const BASE: &str = "https://raw.githubusercontent.com/lucas-giraldelli/portshelf/main/catalog/";
const USER_AGENT: &str = "PortShelf (https://github.com/lucas-giraldelli/portshelf)";

fn dir() -> PathBuf {
    cache_dir().join("catalog")
}

/// A downloaded catalog file (`ports.json`, `achievements/bk.json`), if there is one.
pub fn cached(name: &str) -> Option<String> {
    fs::read_to_string(dir().join(name)).ok()
}

fn fetch(name: &str) -> Result<String, String> {
    let url = format!("{BASE}{name}");
    ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .into_string()
        .map_err(|e| e.to_string())
}

/// Saves a file when it changed; returns whether it did.
fn store(name: &str, text: &str) -> Result<bool, String> {
    if cached(name).as_deref() == Some(text) {
        return Ok(false);
    }
    let path = dir().join(name);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let partial = path.with_extension("part");
    fs::write(&partial, text).map_err(|e| e.to_string())?;
    fs::rename(&partial, &path).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Downloads the newest catalog and its achievement sets; returns whether anything changed.
pub fn refresh() -> Result<bool, String> {
    let text = fetch("ports.json")?;
    let catalog: Value = serde_json::from_str(&text).map_err(|e| format!("ports.json: {e}"))?;
    crate::catalog::validate(&catalog)?;
    let mut changed = false;
    for id in catalog["achievements"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        let name = format!("achievements/{id}.json");
        match fetch(&name).and_then(|set| crate::achievements::parse(&set).map(|_| set)) {
            Ok(set) => changed |= store(&name, &set)?,
            Err(e) => eprintln!("catalog update: {e}"),
        }
    }
    // The catalog last, so it never lists a set that was not downloaded yet.
    changed |= store("ports.json", &text)?;
    Ok(changed)
}

/// Checks for a newer catalog in the background and tells the interface when one arrived.
pub fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || match refresh() {
        Ok(true) => {
            let _ = app.emit("catalog-updated", ());
        }
        Ok(false) => {}
        Err(e) => eprintln!("catalog update: {e}"),
    });
}
