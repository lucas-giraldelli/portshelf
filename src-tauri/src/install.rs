//! Installs a port from its project's latest GitHub or GitLab release: picks the asset for this
//! operating system, downloads it, unpacks it (zip, tar.gz, a bare AppImage, or an archive
//! nested in the download) into its own folder, and finds the program to start.
//! Only the port is downloaded; the game file always comes from the user.

use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const USER_AGENT: &str = "PortShelf (https://github.com/lucas-giraldelli/portshelf)";

/// Per-OS install rule from the catalog.
#[derive(Deserialize, Clone, Debug)]
pub struct Rule {
    /// Asset name must contain every one of these (case-insensitive).
    pub asset: Vec<String>,
    /// Program to start, relative to the install folder; `*` matches any run of characters.
    /// Without it the installer picks an AppImage or the largest executable.
    #[serde(default)]
    pub exec: Option<String>,
}

pub enum Progress {
    Downloading { done: u64, total: Option<u64> },
    Unpacking,
    Done,
}

/// "owner/name" of a project on a host, from its web address.
fn repo_path(url: &str, host: &str) -> Option<String> {
    let rest = url.strip_prefix(&format!("https://{host}/"))?;
    let mut parts = rest.split('/');
    Some(format!("{}/{}", parts.next()?, parts.next()?))
}

fn get_json(url: &str) -> Result<serde_json::Value, String> {
    ureq::get(url).set("User-Agent", USER_AGENT).call().map_err(|e| format!("{url}: {e}"))?.into_json().map_err(|e| e.to_string())
}

/// Tag and downloads (name, address) of the project's latest release.
fn latest_release(repo_url: &str) -> Result<(String, Vec<(String, String)>), String> {
    let (tag, assets): (&str, Vec<(String, String)>);
    let release;
    if let Some(repo) = repo_path(repo_url, "github.com") {
        // Projects that only publish pre-releases have no "latest": take the newest one.
        release = match get_json(&format!("https://api.github.com/repos/{repo}/releases/latest")) {
            Ok(r) => r,
            Err(_) => get_json(&format!("https://api.github.com/repos/{repo}/releases?per_page=1"))?
                .get(0)
                .cloned()
                .ok_or_else(|| format!("{repo} has no releases"))?,
        };
        tag = release["tag_name"].as_str().unwrap_or("latest");
        assets = release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| Some((a["name"].as_str()?.to_string(), a["browser_download_url"].as_str()?.to_string())))
            .collect();
    } else if let Some(repo) = repo_path(repo_url, "gitlab.com") {
        let releases = get_json(&format!("https://gitlab.com/api/v4/projects/{}/releases?per_page=1", repo.replace('/', "%2F")))?;
        release = releases.get(0).cloned().ok_or_else(|| format!("{repo} has no releases"))?;
        tag = release["tag_name"].as_str().unwrap_or("latest");
        assets = release["assets"]["links"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| Some((a["name"].as_str()?.to_string(), a["direct_asset_url"].as_str().or(a["url"].as_str())?.to_string())))
            .collect();
    } else {
        return Err("the project is not on GitHub or GitLab".into());
    }
    Ok((tag.to_string(), assets))
}

/// Downloads and unpacks the port into `dest`, replacing a previous install there.
/// Returns the release tag and the program to start.
pub fn install(repo_url: &str, rule: &Rule, dest: &Path, work: &Path, mut progress: impl FnMut(Progress)) -> Result<(String, PathBuf), String> {
    let (tag, assets) = latest_release(repo_url)?;
    let wanted: Vec<String> = rule.asset.iter().map(|w| w.to_lowercase()).collect();
    // Signatures and checksums sit next to the real files with the same words in their
    // names; skip them, and prefer the shortest name among the rest (the AppImage itself
    // over "AppImage.tar.gz").
    let (name, url) = assets
        .into_iter()
        .filter(|(name, _)| {
            let name = name.to_lowercase();
            let side_file = [".sig", ".asc", ".sha256", ".sha512", ".md5", ".json", ".txt"].iter().any(|e| name.ends_with(e));
            !side_file && wanted.iter().all(|w| name.contains(w.as_str()))
        })
        .min_by_key(|(name, _)| name.len())
        .ok_or_else(|| format!("no download for this system in {repo_url} {tag}"))?;

    fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let file = work.join(&name);
    download(&url, &file, &mut progress)?;
    let file = with_format_extension(file)?;

    progress(Progress::Unpacking);
    let staging = work.join("staging");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    unpack(&file, &staging)?;
    let _ = fs::remove_file(&file);
    // Release zips often wrap a single archive or AppImage; unpack one more level.
    for _ in 0..2 {
        let entries: Vec<PathBuf> = fs::read_dir(&staging).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).collect();
        let inner = match entries.as_slice() {
            [only] if is_archive(only) => only.clone(),
            _ => break,
        };
        let tmp = work.join("inner");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        unpack(&inner, &tmp)?;
        fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &staging).map_err(|e| e.to_string())?;
    }
    // A single top-level folder is the install itself.
    let entries: Vec<PathBuf> = fs::read_dir(&staging).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).collect();
    let root = match entries.as_slice() {
        [only] if only.is_dir() => only.clone(),
        _ => staging.clone(),
    };

    if dest.exists() {
        fs::remove_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::rename(&root, dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let _ = fs::remove_dir_all(&staging);

    let exec = find_exec(dest, rule.exec.as_deref()).ok_or("could not find the program in the download")?;
    make_executable(&exec)?;
    progress(Progress::Done);
    Ok((tag, exec))
}

fn download(url: &str, to: &Path, progress: &mut impl FnMut(Progress)) -> Result<(), String> {
    let response = ureq::get(url).set("User-Agent", USER_AGENT).call().map_err(|e| format!("{url}: {e}"))?;
    let total = response.header("Content-Length").and_then(|v| v.parse().ok());
    let mut reader = response.into_reader();
    let mut out = fs::File::create(to).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    let mut last = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if done - last >= 1024 * 1024 {
            last = done;
            progress(Progress::Downloading { done, total });
        }
    }
    progress(Progress::Downloading { done, total: Some(done) });
    Ok(())
}

fn lower_name(path: &Path) -> String {
    path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_lowercase()
}

/// GitLab downloads are often named without an extension: add the one the content shows.
fn with_format_extension(file: PathBuf) -> Result<PathBuf, String> {
    let name = lower_name(&file);
    if is_archive(&file) || name.ends_with(".appimage") || name.ends_with(".exe") {
        return Ok(file);
    }
    let mut head = [0u8; 12];
    let n = fs::File::open(&file).and_then(|mut f| f.read(&mut head)).map_err(|e| e.to_string())?;
    let ext = match &head[..n] {
        [b'P', b'K', 3, 4, ..] => ".zip",
        [0x1f, 0x8b, ..] => ".tar.gz",
        [0xfd, b'7', b'z', b'X', b'Z', 0, ..] => ".tar.xz",
        [0x7f, b'E', b'L', b'F', _, _, _, _, b'A', b'I', ..] => ".AppImage",
        _ => return Ok(file),
    };
    let named = file.with_file_name(format!("{}{ext}", file.file_name().unwrap_or_default().to_string_lossy()));
    fs::rename(&file, &named).map_err(|e| e.to_string())?;
    Ok(named)
}

fn is_archive(path: &Path) -> bool {
    let name = lower_name(path);
    name.ends_with(".zip") || name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".tar.xz") || name.ends_with(".tar")
}

fn unpack(file: &Path, to: &Path) -> Result<(), String> {
    let name = lower_name(file);
    let open = || fs::File::open(file).map_err(|e| format!("{}: {e}", file.display()));
    if name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(open()?).map_err(|e| e.to_string())?;
        zip.extract(to).map_err(|e| e.to_string())
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        tar::Archive::new(flate2::read::GzDecoder::new(open()?)).unpack(to).map_err(|e| e.to_string())
    } else if name.ends_with(".tar.xz") {
        tar::Archive::new(xz2::read::XzDecoder::new(open()?)).unpack(to).map_err(|e| e.to_string())
    } else if name.ends_with(".tar") {
        tar::Archive::new(open()?).unpack(to).map_err(|e| e.to_string())
    } else {
        // A bare program (AppImage, .exe): it is the install.
        fs::copy(file, to.join(file.file_name().unwrap_or_default())).map(|_| ()).map_err(|e| e.to_string())
    }
}

fn wildcard(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.to_lowercase(), text.to_lowercase());
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == t;
    }
    let mut rest = t.as_str();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(at) => rest = &rest[at + part.len()..],
                None => return false,
            }
        }
    }
    true
}

fn files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            if depth > 0 {
                files(&path, depth - 1, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn is_program(path: &Path) -> bool {
    let name = lower_name(path);
    if cfg!(windows) {
        return name.ends_with(".exe");
    }
    if name.ends_with(".appimage") {
        return true;
    }
    // ELF binaries (Linux) and Mach-O (macOS) start with a magic number.
    let mut magic = [0u8; 4];
    fs::File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok()
        && (magic == *b"\x7fELF" || magic == [0xcf, 0xfa, 0xed, 0xfe])
}

fn find_exec(dir: &Path, pattern: Option<&str>) -> Option<PathBuf> {
    let mut all = Vec::new();
    files(dir, 2, &mut all);
    if let Some(pattern) = pattern {
        return all.into_iter().find(|p| {
            let rel = p.strip_prefix(dir).unwrap_or(p).to_string_lossy().replace('\\', "/");
            wildcard(pattern, &rel)
        });
    }
    let programs: Vec<PathBuf> = all.into_iter().filter(|p| is_program(p) && !lower_name(p).ends_with(".so") && !lower_name(p).contains(".so.")).collect();
    programs
        .iter()
        .find(|p| lower_name(p).ends_with(".appimage"))
        .cloned()
        .or_else(|| programs.into_iter().max_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0)))
}

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(perms.mode() | 0o755);
        fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    }
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(wildcard("*.appimage", "soh.appimage"));
        assert!(wildcard("*.AppImage", "sub/Goemon64Recompiled.AppImage"));
        assert!(wildcard("BanjoRecompiled", "BanjoRecompiled"));
        assert!(wildcard("pd*", "pd.x86_64"));
        assert!(!wildcard("*.appimage", "readme.txt"));
    }

    #[test]
    fn repo_from_url() {
        assert_eq!(repo_path("https://github.com/HarbourMasters/Shipwright", "github.com").as_deref(), Some("HarbourMasters/Shipwright"));
        assert_eq!(repo_path("https://github.com/HarbourMasters/starship/releases/tag/v1.0.0", "github.com").as_deref(), Some("HarbourMasters/starship"));
        assert_eq!(repo_path("https://gitlab.com/sonicdcer/MarioKart64Recomp", "gitlab.com").as_deref(), Some("sonicdcer/MarioKart64Recomp"));
        assert!(repo_path("https://sm64pc.info/sm64pcbuilder2/", "github.com").is_none());
    }
}
