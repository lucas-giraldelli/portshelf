//! A port's own settings files, for the settings panel: RecompFrontend ports keep one JSON
//! object per settings tab (general.json, graphics.json, sound.json, ...).

use crate::library::{self, Install};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A settings file of an installed port, by its name without `.json`.
pub fn config_file(install: &Install, file: &str) -> Option<PathBuf> {
    install.config_dir.as_ref().map(|d| Path::new(d).join(format!("{file}.json")))
}

/// Every settings file of the port, keyed by file name without `.json`.
#[tauri::command]
pub fn get_config(id: String) -> Result<BTreeMap<String, Value>, String> {
    let install = library::installed(&id)?;
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
pub fn set_config(id: String, file: String, key: String, value: Value) -> Result<(), String> {
    let install = library::installed(&id)?;
    if file.contains('/') || file.contains("..") {
        return Err("invalid config file name".into());
    }
    let path = config_file(&install, &file).ok_or("no config directory for this port")?;
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    json.as_object_mut().ok_or("config file is not an object")?.insert(key, value);
    fs::write(path.with_extension("json.bak"), text).map_err(|e| e.to_string())?;
    fs::write(&path, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
