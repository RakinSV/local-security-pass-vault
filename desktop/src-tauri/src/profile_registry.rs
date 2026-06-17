use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub last_seen_ms: i64,
}

type Registry = HashMap<String, Profile>;

const FILE: &str = "profiles.json";

pub fn load(data_dir: &Path) -> Registry {
    std::fs::read_to_string(data_dir.join(FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(data_dir: &Path, reg: &Registry) {
    if let Ok(json) = serde_json::to_string_pretty(reg) {
        let _ = std::fs::write(data_dir.join(FILE), json);
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Upsert a profile entry. Called by the pipe server on every inbound request.
pub fn upsert(data_dir: &Path, profile_id: &str, email: Option<&str>) {
    let mut reg = load(data_dir);
    let entry = reg.entry(profile_id.to_owned()).or_insert_with(|| Profile {
        id: profile_id.to_owned(),
        email: None,
        name: None,
        last_seen_ms: 0,
    });
    entry.last_seen_ms = now_ms();
    if let Some(e) = email {
        if !e.is_empty() {
            entry.email = Some(e.to_owned());
        }
    }
    save(data_dir, &reg);
}

/// Return all profiles sorted newest-first.
pub fn list(data_dir: &Path) -> Vec<Profile> {
    let mut v: Vec<Profile> = load(data_dir).into_values().collect();
    v.sort_by(|a, b| b.last_seen_ms.cmp(&a.last_seen_ms));
    v
}

/// Set (or clear) a user-defined display name for a profile.
pub fn set_name(data_dir: &Path, profile_id: &str, name: Option<String>) {
    let mut reg = load(data_dir);
    if let Some(entry) = reg.get_mut(profile_id) {
        entry.name = name;
        save(data_dir, &reg);
    }
}
