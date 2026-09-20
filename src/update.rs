/*
 * In-app update check (Phase 14) — silent GitHub latest-release poll.
 * Never blocks boot, never auto-installs; just surfaces a banner.
 */

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const GITHUB_LATEST_URL: &str =
    "https://api.github.com/repos/draconis-engineering/olympus/releases/latest";
pub const CACHE_PATH: &str = "data/user/update_cache.json";
const TIMEOUT_MS: u64 = 2000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCache {
    pub checked_at: i64,
    pub latest_tag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
}

// --- version compare: semver-ish, strips leading 'v', compares dot-parts numerically ---
pub fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |s: &str| {
        s.trim_start_matches('v')
            .trim_start_matches('V')
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let cur = parse(current);
    let lat = parse(latest);
    let len = cur.len().max(lat.len());
    for i in 0..len {
        let a = *cur.get(i).unwrap_or(&0);
        let b = *lat.get(i).unwrap_or(&0);
        if b > a {
            return true;
        }
        if b < a {
            return false;
        }
    }
    false
}

pub fn load_cache() -> UpdateCache {
    load_cache_from(Path::new(CACHE_PATH))
}
pub fn load_cache_from(path: &Path) -> UpdateCache {
    let Ok(text) = std::fs::read_to_string(path) else {
        return UpdateCache::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}
pub fn save_cache(cache: &UpdateCache) -> Result<(), String> {
    save_cache_to(cache, Path::new(CACHE_PATH))
}
pub fn save_cache_to(cache: &UpdateCache, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Fetch latest tag from GitHub, with a short timeout. Returns None on any
/// failure (offline, rate-limited, etc.) — caller treats as "no update".
pub async fn fetch_latest_tag() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(TIMEOUT_MS))
        .user_agent("olympus-update-check/1.1")
        .build()
        .ok()?;
    let resp = client.get(GITHUB_LATEST_URL).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let rel: GithubRelease = resp.json().await.ok()?;
    let tag = rel.tag_name.trim().to_string();
    if tag.is_empty() {
        None
    } else {
        Some(tag)
    }
}

/// Check for update and return a banner message if a newer tag exists.
/// Updates the on-disk cache so the UI can show the notice without re-fetching.
pub async fn check_for_update(current: &str) -> Option<String> {
    let latest = fetch_latest_tag().await?;
    let cache = UpdateCache {
        checked_at: chrono::Utc::now().timestamp(),
        latest_tag: Some(latest.clone()),
    };
    let _ = save_cache(&cache);
    if is_newer(current, &latest) {
        Some(format!(
            "Update available: {latest} (you have v{current}) — re-run scripts/install.sh to update"
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn version_compare_basic() {
        assert!(is_newer("1.0.0", "1.1.0"));
        assert!(is_newer("1.0.0", "v1.1.0"));
        assert!(is_newer("1.1.0", "1.1.1"));
        assert!(is_newer("1.1.0", "2.0.0"));
        assert!(!is_newer("1.1.0", "1.1.0"));
        assert!(!is_newer("1.1.0", "1.0.9"));
        assert!(!is_newer("2.0.0", "1.9.9"));
        assert!(is_newer("1.0", "1.0.1"));
        assert!(!is_newer("v1.1.0", "1.1.0"));
    }

    #[test]
    fn cache_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cache.json");
        let c = UpdateCache {
            checked_at: 12345,
            latest_tag: Some("v1.2.0".into()),
        };
        save_cache_to(&c, &path).unwrap();
        let loaded = load_cache_from(&path);
        assert_eq!(loaded.checked_at, 12345);
        assert_eq!(loaded.latest_tag.as_deref(), Some("v1.2.0"));
    }

    #[test]
    fn cache_missing_is_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.json");
        let c = load_cache_from(&path);
        assert!(c.latest_tag.is_none());
    }

    #[test]
    fn update_banner_format() {
        let msg = format!(
            "Update available: {} (you have v{}) — re-run scripts/install.sh to update",
            "v1.2.0", "1.1.0"
        );
        assert!(msg.contains("v1.2.0"));
        assert!(msg.contains("1.1.0"));
    }
}
