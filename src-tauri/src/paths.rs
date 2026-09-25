//! Where PortShelf keeps its files.

use std::path::PathBuf;

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

/// A path from the catalog or the library, with `~/` meaning the home folder.
pub fn expand(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None => PathBuf::from(path),
    }
}

/// Library, time played and covers.
pub fn app_dir() -> PathBuf {
    home().join(".config/portshelf")
}

/// Downloads, cover indexes and unpacked archives; safe to delete.
pub fn cache_dir() -> PathBuf {
    home().join(".cache/portshelf")
}

pub fn covers_dir() -> PathBuf {
    app_dir().join("covers")
}

/// Ports installed by PortShelf, one folder each.
pub fn ports_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| home().join(".local/share")).join("PortShelf").join("ports")
}
