//! Time played per port: from launch until the last process of the game exits, including
//! the ones its launcher starts. On Linux the port runs in its own systemd scope, which ends
//! only when every process in it is gone; elsewhere the launched program itself is waited on.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Shorter sessions are a launcher opened and closed, or a crash at start: not counted.
const MIN_SESSION: Duration = Duration::from_secs(15);

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Playtime {
    pub total_secs: u64,
    /// Unix time the last session ended.
    pub last_played: u64,
    pub sessions: u32,
}

fn path(app_dir: &Path) -> PathBuf {
    app_dir.join("playtime.json")
}

pub fn load(app_dir: &Path) -> BTreeMap<String, Playtime> {
    std::fs::read_to_string(path(app_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn record(app_dir: &Path, id: &str, played: Duration) {
    if played < MIN_SESSION {
        return;
    }
    let mut all = load(app_dir);
    let entry = all.entry(id.to_string()).or_default();
    entry.total_secs += played.as_secs();
    entry.sessions += 1;
    entry.last_played = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
    if let Ok(text) = serde_json::to_string_pretty(&all) {
        let _ = std::fs::write(path(app_dir), text);
    }
}

/// A running game: the program PortShelf started and, on Linux, the scope holding it.
pub struct Session {
    child: Child,
    scope: Option<String>,
    started: Instant,
}

/// Name of the systemd scope for one launch of a port.
pub fn unit_name(id: &str) -> String {
    format!("portshelf-{id}-{}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or_default())
}

/// Starts the game: inside its systemd scope when one is given and systemd-run works,
/// otherwise the plain command.
pub fn start(mut cmd: Command, scoped: Option<Command>) -> std::io::Result<Session> {
    if let Some(mut scoped) = scoped {
        let unit = scoped.get_args().skip_while(|a| *a != "--unit").nth(1).map(|u| u.to_string_lossy().into_owned());
        if let Ok(child) = scoped.spawn() {
            return Ok(Session { child, scope: unit, started: Instant::now() });
        }
    }
    Ok(Session { child: cmd.spawn()?, scope: None, started: Instant::now() })
}

fn scope_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", &format!("{unit}.scope")])
        .status()
        .is_ok_and(|s| s.success())
}

impl Session {
    /// Blocks until the game is gone, then adds the session to the port's time played.
    pub fn wait(mut self, app_dir: &Path, id: &str) {
        let _ = self.child.wait();
        if let Some(unit) = &self.scope {
            while scope_active(unit) {
                std::thread::sleep(Duration::from_secs(2));
            }
        }
        record(app_dir, id, self.started.elapsed());
    }
}
