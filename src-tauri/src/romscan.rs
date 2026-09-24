//! Identifies game files in a folder and matches them to catalog ports: by the game code
//! stored in the file itself when the format has one (N64 cartridge header, GameCube disc
//! header, also inside RVZ / WIA images), otherwise by comparing the file name with the
//! port's No-Intro / Redump title.

use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
pub struct Found {
    pub path: String,
    pub port: String,
    /// "code" when the file's own header matched, "name" when only the file name did.
    pub by: &'static str,
    /// Game code or title read from the file, for display.
    pub detail: String,
    /// Looks like a hack or a patched copy (tags in the name, or a different internal name);
    /// ports check for the original game, so these are not picked by default.
    pub modified: bool,
}

fn console_of(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?.to_lowercase();
    let ext = name.rsplit('.').next()?;
    Some(match ext {
        "z64" | "n64" | "v64" => "n64",
        "rvz" | "wia" | "gcm" | "ciso" | "gcz" => "gc",
        "iso" if name.ends_with(".nkit.iso") => "gc",
        "iso" => "iso", // GameCube or PlayStation 2: decided by the header
        "sfc" | "smc" => "snes",
        "gba" => "gba",
        "md" | "gen" | "smd" => "md",
        "cue" | "chd" => "ps1",
        _ => return None,
    })
}

/// N64 header in big-endian order: internal name at 0x20, game code (e.g. NDOE) at 0x3B.
fn n64_header(path: &Path) -> Option<(String, String)> {
    let mut b = [0u8; 0x40];
    fs::File::open(path).ok()?.read_exact(&mut b).ok()?;
    match &b[..4] {
        [0x80, 0x37, 0x12, 0x40] => {}
        [0x37, 0x80, 0x40, 0x12] => b.chunks_exact_mut(2).for_each(|c| c.swap(0, 1)),
        [0x40, 0x12, 0x37, 0x80] => b.chunks_exact_mut(4).for_each(|c| c.reverse()),
        _ => return None,
    }
    let text = |r: std::ops::Range<usize>| String::from_utf8_lossy(&b[r]).trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string();
    Some((text(0x20..0x34), text(0x3B..0x3F)))
}

/// GameCube / Wii game ID (e.g. GZ2E01): at the start of a plain image, and in the copy
/// of the disc header that RVZ and WIA files keep at 0x58.
fn disc_id(path: &Path) -> Option<String> {
    let mut b = [0u8; 0x60];
    fs::File::open(path).ok()?.read_exact(&mut b).ok()?;
    let id = if &b[..3] == b"RVZ" || &b[..3] == b"WIA" { &b[0x58..0x5E] } else { &b[..6] };
    id.iter().all(|c| c.is_ascii_alphanumeric()).then(|| String::from_utf8_lossy(id).into_owned())
}

/// Folder name for a game: "Legend of Zelda, The - Majora's Mask" -> "legend_of_zelda_majoras_mask".
pub fn game_folder(title: &str) -> String {
    normalize(&title.replace('\'', "")).replace(' ', "_")
}

/// Game files directly inside `dir` (a game's own folder), whatever their names.
pub fn files_in(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.is_file() && console_of(p).is_some()).collect())
        .unwrap_or_default();
    out.sort();
    out
}

fn normalize(title: &str) -> String {
    let base = title.split(" (").next().unwrap_or(title).split(" [").next().unwrap_or(title);
    let base = base.replace(", The", "").replace("The ", "");
    base.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            if depth > 0 {
                walk(&path, depth - 1, out);
            }
        } else if console_of(&path).is_some() {
            out.push(path);
        }
    }
}

/// Matches every game file under `dir` (four folders deep) to the catalog's ports.
/// A file can serve several ports of the same game (Majora's Mask: two ports).
pub fn scan(dir: &Path, catalog: &Value) -> Vec<Found> {
    let mut files = Vec::new();
    walk(dir, 4, &mut files);
    let ports = catalog["ports"].as_array().cloned().unwrap_or_default();
    let mut found = Vec::new();
    for file in files {
        let kind = console_of(&file).unwrap_or_default();
        let mut internal_name = String::new();
        let (codes, detail): (Vec<String>, String) = match kind {
            "n64" => match n64_header(&file) {
                Some((name, code)) => {
                    internal_name = name.clone();
                    (vec![code.clone()], format!("{code} {name}"))
                }
                None => (vec![], String::new()),
            },
            "gc" | "iso" => match disc_id(&file) {
                Some(id) => (vec![id.clone(), id[..4].to_string()], id),
                None => (vec![], String::new()),
            },
            _ => (vec![], String::new()),
        };
        let stem = file.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let file_name = normalize(stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(&stem));
        for port in &ports {
            let console = port["console"].as_str().unwrap_or_default();
            if kind != console && !(kind == "iso" && matches!(console, "gc" | "ps2")) {
                continue;
            }
            let port_codes: Vec<&str> = port["codes"].as_array().map(|c| c.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
            let by_code = codes.iter().any(|c| port_codes.contains(&c.as_str()));
            // A file whose own code says it is another game never matches by name.
            let contradicts = !codes.is_empty() && !port_codes.is_empty() && !by_code;
            let title = normalize(port["title"].as_str().unwrap_or_default());
            let by_name = !contradicts && !title.is_empty() && file_name == title;
            if by_code || by_name {
                let squash = |t: &str| t.replace(' ', "");
                let header_differs = kind == "n64"
                    && !internal_name.is_empty()
                    && !squash(&title).contains(&squash(&normalize(&internal_name)));
                // Any extra words in the file name ("Splitscreen", "decompressed") may mean a hack.
                let name_differs = squash(&file_name) != squash(&title);
                let tagged = stem.contains('[') || stem.to_lowercase().contains("hack");
                found.push(Found {
                    path: file.to_string_lossy().into_owned(),
                    port: port["id"].as_str().unwrap_or_default().to_string(),
                    by: if by_code { "code" } else { "name" },
                    detail: if detail.is_empty() { stem.clone() } else { detail.clone() },
                    modified: tagged || header_differs || name_differs,
                });
            }
        }
    }
    // Best file first for each port: original copies, then header matches, then shorter paths.
    found.sort_by(|a, b| {
        (a.port.as_str(), a.modified, a.by != "code", a.path.len()).cmp(&(b.port.as_str(), b.modified, b.by != "code", b.path.len()))
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(normalize("Legend of Zelda, The - Twilight Princess (USA)"), "legend of zelda twilight princess");
        assert_eq!(normalize("Super Mario World (USA) [!]"), "super mario world");
    }

    #[test]
    fn folders() {
        assert_eq!(game_folder("Legend of Zelda, The - Majora's Mask"), "legend_of_zelda_majoras_mask");
        assert_eq!(game_folder("Chameleon Twist"), "chameleon_twist");
        assert_eq!(game_folder("Dr. Mario 64"), "dr_mario_64");
    }

    #[test]
    fn kinds() {
        assert_eq!(console_of(Path::new("a/Donkey Kong 64 (USA).n64")), Some("n64"));
        assert_eq!(console_of(Path::new("TP.rvz")), Some("gc"));
        assert_eq!(console_of(Path::new("readme.txt")), None);
    }
}
