//! PortShelf's own achievements: sets written for the ports in the catalog
//! (`catalog/achievements/<port>.json`), checked against the game's memory while it runs, and
//! unlocked locally (`~/.config/portshelf/achievements.json`). Nothing is sent anywhere.

use crate::paths::app_dir;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod memory;

/// Sets bundled with PortShelf, by port id.
const SETS: &[(&str, &str)] = &[("bk", include_str!("../../../catalog/achievements/bk.json"))];

#[derive(Deserialize, Clone, Debug)]
pub struct Set {
    pub port: String,
    /// How the game's memory is read; only "n64-recomp" for now.
    pub memory: String,
    /// All must hold for the game to be in play (not on a title or file select screen).
    #[serde(default)]
    pub playing: Vec<Condition>,
    pub achievements: Vec<Achievement>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Achievement {
    pub id: String,
    pub points: u32,
    pub title: BTreeMap<String, String>,
    pub description: BTreeMap<String, String>,
    #[serde(skip_serializing)]
    pub conditions: Vec<Condition>,
}

/// One memory check: a byte, half word, word or single bit at an N64 address, compared to a value.
#[derive(Deserialize, Clone, Debug)]
pub struct Condition {
    read: String,
    addr: String,
    #[serde(default)]
    bit: u32,
    op: String,
    value: u32,
}

impl Condition {
    fn holds(&self, ram: &mut memory::N64Ram) -> Option<bool> {
        let addr = u32::from_str_radix(self.addr.trim_start_matches("0x"), 16).ok()?;
        let v = match self.read.as_str() {
            "u32" => ram.read(addr, 4).ok()?,
            "u16" => ram.read(addr, 2).ok()?,
            "u8" => ram.read(addr, 1).ok()?,
            "bit" => (ram.read(addr, 1).ok()? >> self.bit) & 1,
            _ => return None,
        };
        Some(match self.op.as_str() {
            "eq" => v == self.value,
            "ne" => v != self.value,
            "gt" => v > self.value,
            "ge" => v >= self.value,
            "lt" => v < self.value,
            "le" => v <= self.value,
            _ => return None,
        })
    }
}

fn all_hold(conditions: &[Condition], ram: &mut memory::N64Ram) -> Option<bool> {
    for c in conditions {
        if !c.holds(ram)? {
            return Some(false);
        }
    }
    Some(true)
}

pub fn set_for(port: &str) -> Option<Set> {
    let (_, json) = SETS.iter().find(|(id, _)| *id == port)?;
    serde_json::from_str(json).map_err(|e| eprintln!("achievements for {port}: {e}")).ok()
}

/// Unlock times (Unix seconds) per port and achievement.
type Unlocked = BTreeMap<String, BTreeMap<String, u64>>;

fn unlocked_path() -> std::path::PathBuf {
    app_dir().join("achievements.json")
}

fn load_unlocked() -> Unlocked {
    std::fs::read_to_string(unlocked_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn record_unlock(port: &str, id: &str) {
    let mut all = load_unlocked();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
    all.entry(port.to_string()).or_default().entry(id.to_string()).or_insert(now);
    if let Ok(text) = serde_json::to_string_pretty(&all) {
        let _ = std::fs::write(unlocked_path(), text);
    }
}

/// Text in the shelf's language, English otherwise.
fn localized(texts: &BTreeMap<String, String>, lang: &str) -> String {
    texts.get(lang).or_else(|| texts.get("en")).cloned().unwrap_or_default()
}

/// Watches a running port for its achievements until the process ends. An achievement unlocks
/// when its conditions hold on two reads in a row while the game is in play; progress already
/// in the save counts as soon as the game is loaded.
pub fn watch(pid: u32, set: Set) {
    let port = set.port.clone();
    let mut done: Vec<String> = load_unlocked().get(&port).map(|m| m.keys().cloned().collect()).unwrap_or_default();
    let mut pending: BTreeMap<String, u32> = BTreeMap::new();
    let alive = || std::path::Path::new(&format!("/proc/{pid}")).exists();
    let mut ram = None;
    while alive() {
        std::thread::sleep(Duration::from_millis(250));
        if set.memory != "n64-recomp" {
            return;
        }
        if ram.is_none() {
            ram = memory::N64Ram::attach(pid);
        }
        let Some(ram) = ram.as_mut() else { continue };
        if all_hold(&set.playing, ram) != Some(true) {
            pending.clear();
            continue;
        }
        for a in &set.achievements {
            if done.contains(&a.id) {
                continue;
            }
            if all_hold(&a.conditions, ram) == Some(true) {
                let hits = pending.entry(a.id.clone()).or_insert(0);
                *hits += 1;
                if *hits >= 2 {
                    record_unlock(&port, &a.id);
                    done.push(a.id.clone());
                    let lang = crate::overlay::language();
                    let _ = crate::overlay::show(&localized(&a.title, &lang), &localized(&a.description, &lang), a.points, None);
                }
            } else {
                pending.remove(&a.id);
            }
        }
    }
}

/// A port's achievements with their unlock times, for the shelf.
#[derive(Serialize)]
pub struct AchievementStatus {
    #[serde(flatten)]
    achievement: Achievement,
    unlocked: Option<u64>,
}

#[tauri::command]
pub fn get_achievements(port: String) -> Vec<AchievementStatus> {
    let unlocked = load_unlocked();
    let times = unlocked.get(&port);
    set_for(&port)
        .map(|set| {
            set.achievements
                .into_iter()
                .map(|a| AchievementStatus { unlocked: times.and_then(|t| t.get(&a.id)).copied(), achievement: a })
                .collect()
        })
        .unwrap_or_default()
}

/// Progress of one port's set, for the shelf's achievements overview.
#[derive(Serialize)]
pub struct Summary {
    port: String,
    unlocked: usize,
    total: usize,
    points: u32,
    total_points: u32,
    /// Unix time of the latest unlock.
    last: Option<u64>,
}

/// Every port with an achievement set and how far the player is in it.
#[tauri::command]
pub fn get_achievement_summary() -> Vec<Summary> {
    let unlocked = load_unlocked();
    SETS.iter()
        .filter_map(|(id, _)| set_for(id))
        .map(|set| {
            let times = unlocked.get(&set.port);
            let got: Vec<&Achievement> = set.achievements.iter().filter(|a| times.is_some_and(|t| t.contains_key(&a.id))).collect();
            Summary {
                port: set.port.clone(),
                unlocked: got.len(),
                total: set.achievements.len(),
                points: got.iter().map(|a| a.points).sum(),
                total_points: set.achievements.iter().map(|a| a.points).sum(),
                last: times.and_then(|t| t.values().max().copied()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sets_parse() {
        for (id, _) in SETS {
            let set = set_for(id).expect("set parses");
            assert_eq!(&set.port, id);
            assert!(!set.achievements.is_empty());
            for a in &set.achievements {
                assert!(a.title.contains_key("en") && a.title.contains_key("pt-BR"), "{}: titles", a.id);
                for c in &a.conditions {
                    assert!(u32::from_str_radix(c.addr.trim_start_matches("0x"), 16).is_ok(), "{}: {}", a.id, c.addr);
                }
            }
        }
    }
}
