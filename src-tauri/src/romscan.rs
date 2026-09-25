//! Identifies game files in a folder and matches them to catalog ports: by the game code
//! stored in the file itself when the format has one (N64 cartridge header, GameCube disc
//! header, also inside RVZ / WIA images), otherwise by comparing the file name with the
//! port's No-Intro / Redump title.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

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
    /// The port lists the releases it accepts and this file is none of them.
    pub wrong_version: bool,
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
        "nes" => "nes",
        "gb" | "gbc" => "gb",
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

/// N64 ROMs come in three byte orders; ports check the big-endian (.z64) one.
pub fn to_z64(mut data: Vec<u8>) -> Result<Vec<u8>, String> {
    match data.get(..4) {
        Some([0x80, 0x37, 0x12, 0x40]) => {}
        Some([0x37, 0x80, 0x40, 0x12]) => data.chunks_exact_mut(2).for_each(|c| c.swap(0, 1)),
        Some([0x40, 0x12, 0x37, 0x80]) => data.chunks_exact_mut(4).for_each(|c| c.reverse()),
        _ => return Err("this file is not an N64 ROM".into()),
    }
    Ok(data)
}

type HashKey = (PathBuf, u64, Option<SystemTime>, &'static str);
static HASHES: Mutex<Option<HashMap<HashKey, String>>> = Mutex::new(None);

/// Hash of a game file ("xxh3", "sha1", "md5" or "sha256", lowercase hex), N64 ROMs in .z64
/// order. Remembered per file size and date, since the ROM folder is checked often.
fn file_hash(path: &Path, algo: &'static str) -> Option<String> {
    use sha2::Digest;
    let meta = fs::metadata(path).ok()?;
    let key = (path.to_path_buf(), meta.len(), meta.modified().ok(), algo);
    if let Some(h) = HASHES.lock().ok()?.get_or_insert_with(HashMap::new).get(&key) {
        return Some(h.clone());
    }
    let mut data = fs::read(path).ok()?;
    if console_of(path) == Some("n64") {
        data = to_z64(data).ok()?;
    }
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let hash = match algo {
        "xxh3" => format!("{:016x}", xxhash_rust::xxh3::xxh3_64(&data)),
        "sha1" => hex(&sha1::Sha1::digest(&data)),
        "md5" => hex(&md5::Md5::digest(&data)),
        "sha256" => hex(&sha2::Sha256::digest(&data)),
        _ => return None,
    };
    HASHES.lock().ok()?.get_or_insert_with(HashMap::new).insert(key, hash.clone());
    Some(hash)
}

/// Whether a file is one of the releases a port accepts. The catalog lists them as
/// "algo:hex" in rom.hashes; None when it lists none (the port takes any copy).
pub fn accepts(port: &Value, path: &Path) -> Option<bool> {
    let hashes: Vec<&str> = port["rom"]["hashes"].as_array()?.iter().filter_map(|h| h.as_str()).collect();
    if hashes.is_empty() {
        return None;
    }
    Some(hashes.iter().any(|h| {
        let (algo, want) = h.split_once(':').unwrap_or(("sha1", h));
        let algo = match algo {
            "xxh3" => "xxh3",
            "md5" => "md5",
            "sha256" => "sha256",
            _ => "sha1",
        };
        file_hash(path, algo).is_some_and(|got| got.eq_ignore_ascii_case(want))
    }))
}

/// Release of a game file in No-Intro style, e.g. "Banjo-Kazooie (USA) (Rev 1)", read from the
/// N64 header's region and revision; other files are named by their file name.
pub fn release_label(path: &Path, title: &str) -> String {
    let stem = || path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
    if console_of(path) != Some("n64") {
        return stem();
    }
    let mut b = [0u8; 0x40];
    if fs::File::open(path).and_then(|mut f| f.read_exact(&mut b)).is_err() {
        return stem();
    }
    let Ok(h) = to_z64(b.to_vec()) else { return stem() };
    let region = match h[0x3E] {
        b'E' => "USA",
        b'P' | b'X' | b'Y' => "Europe",
        b'J' => "Japan",
        b'U' => "Australia",
        b'D' => "Germany",
        b'F' => "France",
        b'I' => "Italy",
        b'S' => "Spain",
        _ => return stem(),
    };
    match h[0x3F] {
        0 => format!("{title} ({region})"),
        rev => format!("{title} ({region}) (Rev {rev})"),
    }
}

/// GameCube / Wii game ID (e.g. GZ2E01): at the start of a plain image, and in the copy
/// of the disc header that RVZ and WIA files keep at 0x58.
fn disc_id(path: &Path) -> Option<String> {
    let mut b = [0u8; 0x60];
    fs::File::open(path).ok()?.read_exact(&mut b).ok()?;
    let id = if &b[..3] == b"RVZ" || &b[..3] == b"WIA" { &b[0x58..0x5E] } else { &b[..6] };
    id.iter().all(|c| c.is_ascii_alphanumeric()).then(|| String::from_utf8_lossy(id).into_owned())
}

/// Zip and 7z archives, the way ROMs are usually downloaded.
pub fn is_archive(path: &Path) -> bool {
    let name = path.to_string_lossy().to_lowercase();
    name.ends_with(".zip") || name.ends_with(".7z")
}

/// Unpacks the game file inside a zip or 7z archive into `dest` (kept if already there with
/// the same size) and returns its path.
pub fn unpack_game_file(archive: &Path, dest: &Path) -> Result<PathBuf, String> {
    let fail = |e: String| format!("{}: {e}", archive.display());
    fs::create_dir_all(dest).map_err(|e| fail(e.to_string()))?;
    let target = |name: &str| Path::new(name).file_name().map(|n| dest.join(n));
    let unchanged = |path: &Path, size: u64| fs::metadata(path).is_ok_and(|m| m.len() == size);
    if archive.to_string_lossy().to_lowercase().ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| fail(e.to_string()))?).map_err(|e| fail(e.to_string()))?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| fail(e.to_string()))?;
            let Some(out) = target(entry.name()).filter(|p| entry.is_file() && console_of(p).is_some()) else { continue };
            if !unchanged(&out, entry.size()) {
                let mut file = fs::File::create(&out).map_err(|e| fail(e.to_string()))?;
                std::io::copy(&mut entry, &mut file).map_err(|e| fail(e.to_string()))?;
            }
            return Ok(out);
        }
    } else {
        let mut found = None;
        let mut reader = sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty()).map_err(|e| fail(e.to_string()))?;
        reader
            .for_each_entries(|entry, data| {
                let Some(out) = target(&entry.name).filter(|p| !entry.is_directory && console_of(p).is_some()) else {
                    // Solid archives decode in order: skipping means reading through.
                    std::io::copy(data, &mut std::io::sink())?;
                    return Ok(true);
                };
                if !unchanged(&out, entry.size) {
                    std::io::copy(data, &mut fs::File::create(&out)?)?;
                }
                found = Some(out);
                Ok(false)
            })
            .map_err(|e| fail(e.to_string()))?;
        if let Some(out) = found {
            return Ok(out);
        }
    }
    Err(fail("no game file inside".into()))
}

/// Unpacks the archives sitting in a game's folder, so the game files next to them are found.
pub fn unpack_archives_in(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for archive in entries.flatten().map(|e| e.path()).filter(|p| p.is_file() && is_archive(p)) {
        let _ = unpack_game_file(&archive, dir);
    }
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
            // Plain disc images and RVZ files can be GameCube, Wii or PS2; the header decides.
            let disc_image = matches!(kind, "iso" | "gc") && matches!(console, "gc" | "wii" | "ps2");
            if kind != console && !disc_image {
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
                    wrong_version: accepts(port, &file) == Some(false),
                });
            }
        }
    }
    // Best file first for each port: accepted releases, original copies, header matches, shorter paths.
    found.sort_by(|a, b| {
        (a.port.as_str(), a.wrong_version, a.modified, a.by != "code", a.path.len())
            .cmp(&(b.port.as_str(), b.wrong_version, b.modified, b.by != "code", b.path.len()))
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
    fn hashes() {
        let file = std::env::temp_dir().join("portshelf-hash-test.bin");
        fs::write(&file, b"abc").unwrap();
        assert_eq!(file_hash(&file, "sha1").unwrap(), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(file_hash(&file, "md5").unwrap(), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(file_hash(&file, "sha256").unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(file_hash(&file, "xxh3").unwrap(), "78af5f94892f3950");
        let port = serde_json::json!({ "rom": { "hashes": ["md5:900150983CD24FB0D6963F7D28E17F72"] } });
        assert_eq!(accepts(&port, &file), Some(true));
        assert_eq!(accepts(&serde_json::json!({ "rom": { "hashes": ["sha1:00"] } }), &file), Some(false));
        assert_eq!(accepts(&serde_json::json!({}), &file), None);
        let _ = fs::remove_file(file);
    }

    #[test]
    fn kinds() {
        assert_eq!(console_of(Path::new("a/Donkey Kong 64 (USA).n64")), Some("n64"));
        assert_eq!(console_of(Path::new("TP.rvz")), Some("gc"));
        assert_eq!(console_of(Path::new("readme.txt")), None);
    }
}
