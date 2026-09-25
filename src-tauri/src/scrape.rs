//! Cover art from libretro-thumbnails: one repository per system, box art named
//! after the No-Intro / Redump title ("Legend of Zelda, The - Majora's Mask (USA).png").
//! The file list of each system is fetched once and cached; a title is matched
//! against it and the best regional release is downloaded.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const USER_AGENT: &str = "PortShelf (https://github.com/lucas-giraldelli/portshelf)";

fn repository(console: &str) -> Option<&'static str> {
    Some(match console {
        "n64" => "Nintendo_-_Nintendo_64",
        "gc" => "Nintendo_-_GameCube",
        "snes" => "Nintendo_-_Super_Nintendo_Entertainment_System",
        "gba" => "Nintendo_-_Game_Boy_Advance",
        "ps1" => "Sony_-_PlayStation",
        "ps2" => "Sony_-_PlayStation_2",
        "md" => "Sega_-_Mega_Drive_-_Genesis",
        "x360" => "Microsoft_-_Xbox_360",
        "nes" => "Nintendo_-_Nintendo_Entertainment_System",
        "gb" => "Nintendo_-_Game_Boy",
        "wii" => "Nintendo_-_Wii",
        "arcade" => "MAME",
        _ => return None,
    })
}

fn get_json(url: &str) -> Result<serde_json::Value, String> {
    ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Box art file names for a system, cached in `cache_dir` for a week.
fn index(console: &str, cache_dir: &Path) -> Result<Vec<String>, String> {
    let repo = repository(console).ok_or_else(|| format!("no cover source for {console}"))?;
    let cache = cache_dir.join(format!("libretro-{console}.json"));
    if let Ok(meta) = fs::metadata(&cache) {
        let fresh = meta.modified().ok().and_then(|m| m.elapsed().ok()).is_some_and(|age| age.as_secs() < 7 * 24 * 3600);
        if fresh {
            if let Ok(names) = serde_json::from_str(&fs::read_to_string(&cache).unwrap_or_default()) {
                return Ok(names);
            }
        }
    }
    let base = format!("https://api.github.com/repos/libretro-thumbnails/{repo}/git/trees");
    let root = get_json(&format!("{base}/master"))?;
    let sha = root["tree"]
        .as_array()
        .and_then(|t| t.iter().find(|e| e["path"] == "Named_Boxarts"))
        .and_then(|e| e["sha"].as_str())
        .ok_or("no Named_Boxarts folder")?
        .to_string();
    let tree = get_json(&format!("{base}/{sha}"))?;
    let names: Vec<String> = tree["tree"]
        .as_array()
        .map(|t| t.iter().filter_map(|e| e["path"].as_str().map(String::from)).collect())
        .unwrap_or_default();
    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let _ = fs::write(&cache, serde_json::to_string(&names).unwrap_or_default());
    Ok(names)
}

/// "Legend of Zelda, The - Majora's Mask (USA) (Rev 1)" -> "legend of zelda majora s mask"
fn normalize(title: &str) -> String {
    let base = title.split(" (").next().unwrap_or(title).replace(", The", "").replace("The ", "");
    base.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lower is better: prefer USA, then World and Europe; avoid prototypes and demos.
fn region_rank(file: &str) -> u32 {
    let lower = file.to_lowercase();
    let penalty = ["beta", "proto", "demo", "sample", "kiosk", "(pirate", "(hack"].iter().any(|w| lower.contains(w)) as u32 * 100;
    let region = if file.contains("(USA") { 0 } else if file.contains("World") { 1 } else if file.contains("Europe") { 2 } else { 3 };
    penalty + region * 10 + file.matches('(').count() as u32
}

/// Picks the box art for `title`: exact title match first, then titles containing every word.
pub fn best_match<'a>(title: &str, names: &'a [String]) -> Option<&'a String> {
    let wanted = normalize(title);
    let words: Vec<&str> = wanted.split(' ').collect();
    let exact = names.iter().filter(|n| normalize(n) == wanted).min_by_key(|n| region_rank(n));
    exact.or_else(|| {
        names
            .iter()
            .filter(|n| {
                let have = normalize(n);
                words.iter().all(|w| have.split(' ').any(|h| h == *w))
            })
            .min_by_key(|n| (normalize(n).len(), region_rank(n)))
    })
}

/// Downloads the box art for `title` on `console` to `dest`. Returns the matched file name.
pub fn fetch_cover(console: &str, title: &str, cache_dir: &Path, dest: &PathBuf) -> Result<String, String> {
    let names = index(console, cache_dir)?;
    let file = best_match(title, &names).ok_or_else(|| format!("no box art found for \"{title}\""))?.clone();
    let repo = repository(console).unwrap_or_default();
    let url = format!(
        "https://raw.githubusercontent.com/libretro-thumbnails/{repo}/master/Named_Boxarts/{}",
        urlencoding(&file)
    );
    let mut bytes = Vec::new();
    ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .into_reader()
        .take(20 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"\x89PNG") {
        return Err(format!("{file} is not a PNG"));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(dest, bytes).map_err(|e| e.to_string())?;
    Ok(file)
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_no_intro_names() {
        let names: Vec<String> = [
            "Legend of Zelda, The - Majora's Mask (Europe) (En,Fr,De,Es).png",
            "Legend of Zelda, The - Majora's Mask (USA).png",
            "Legend of Zelda, The - Majora's Mask (USA) (Demo) (Kiosk).png",
            "Mario Kart 64 (USA).png",
            "Mario Tennis (USA).png",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(best_match("Legend of Zelda, The - Majora's Mask", &names).unwrap(), &names[1]);
        assert_eq!(best_match("Zelda Majora's Mask", &names).unwrap(), &names[1]);
        assert_eq!(best_match("Mario Tennis", &names).unwrap(), &names[4]);
        assert!(best_match("Banjo-Kazooie", &names).is_none());
    }
}
