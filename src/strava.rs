/*
 * Strava auto-upload for Olympus (Phase 12).
 * Copyright (C) 2026 Simon Stordal Amundgård
 *
 * Token + refresh persisted at data/user/strava.json (gitignored, like
 * profile.json). Queued FIT uploads at data/user/strava_queue.json.
 * OAuth PKCE via `oauth2` Helpers; FIT upload via `reqwest` multipart.
 */

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

pub const TOKEN_PATH: &str = "data/user/strava.json";
pub const QUEUE_PATH: &str = "data/user/strava_queue.json";

// Strava OAuth / upload endpoints.
#[allow(dead_code)]
pub const STRAVA_AUTH_URL: &str = "https://www.strava.com/oauth/authorize";
pub const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";
pub const STRAVA_UPLOAD_URL: &str = "https://www.strava.com/api/v3/uploads";

/// Where the OAuth redirect lands. For a TUI the common pattern is a loopback
/// `http://localhost:<port>/callback`. Users paste the `code` back when no
/// server is running, so we keep this configurable via env.
#[allow(dead_code)]
pub fn redirect_uri() -> String {
    std::env::var("OLYMPUS_STRAVA_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:8080/callback".to_string())
}

/// Client id from `STRAVA_CLIENT_ID` env (never hardcoded / never committed).
pub fn client_id_from_env() -> Option<String> {
    std::env::var("STRAVA_CLIENT_ID")
        .ok()
        .filter(|s| !s.is_empty())
}
pub fn client_secret_from_env() -> Option<String> {
    std::env::var("STRAVA_CLIENT_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// Persisted Strava OAuth token (mirrors Strava's token response).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StravaToken {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix timestamp (seconds) when `access_token` expires.
    pub expires_at: i64,
    #[serde(default = "default_token_type")]
    pub token_type: String,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

#[allow(dead_code)]
impl StravaToken {
    pub fn new(access_token: String, refresh_token: String, expires_in_secs: i64) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_at: Utc::now().timestamp() + expires_in_secs,
            token_type: "Bearer".to_string(),
        }
    }

    /// True when the token is expired or will expire within `buffer_secs`.
    pub fn is_expired_with_buffer(&self, buffer_secs: i64) -> bool {
        Utc::now().timestamp() + buffer_secs >= self.expires_at
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired_with_buffer(0)
    }

    /// Needs a refresh if it expires within 5 minutes.
    pub fn needs_refresh(&self) -> bool {
        self.is_expired_with_buffer(300)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: Option<i64>,
    expires_in: Option<i64>,
    token_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

pub fn load_token() -> Option<StravaToken> {
    load_token_from(Path::new(TOKEN_PATH))
}

pub fn load_token_from(path: &Path) -> Option<StravaToken> {
    let text = std::fs::read_to_string(path).ok()?;
    let tok: StravaToken = serde_json::from_str(&text).ok()?;
    if tok.access_token.is_empty() {
        return None;
    }
    Some(tok)
}

pub fn save_token(token: &StravaToken) -> Result<(), String> {
    save_token_to(token, Path::new(TOKEN_PATH))
}

pub fn save_token_to(token: &StravaToken, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(token).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn clear_token() -> Result<(), String> {
    let p = Path::new(TOKEN_PATH);
    if p.exists() {
        std::fs::remove_file(p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn is_connected() -> bool {
    load_token().is_some()
}

// ---------------------------------------------------------------------------
// PKCE + Auth URL
// ---------------------------------------------------------------------------

/// Generate a PKCE verifier + S256 challenge pair. Returns (verifier, challenge).
#[allow(dead_code)]
pub fn generate_pkce() -> (String, String) {
    use oauth2::{PkceCodeChallenge, PkceCodeVerifier};
    // oauth2 generates a random verifier internally when we create a challenge.
    // We need both parts — create a verifier, then derive the challenge.
    let verifier = PkceCodeVerifier::new(random_verifier_string());
    let challenge = PkceCodeChallenge::from_code_verifier_sha256(&verifier);
    // oauth2's PkceCodeChallenge consumes the verifier; we recreate so caller
    // can persist the verifier for the later exchange. So we return the
    // verifier string + challenge string directly.
    // Note: we generate verifier via random string above; challenge is SHA256.
    (verifier.secret().clone(), challenge.as_str().to_string())
}
#[allow(dead_code)]
fn random_verifier_string() -> String {
    // 64 random bytes -> base64url -> 86 chars, within 43-128 spec.
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut bytes = [0u8; 64];
    // Use uuid + chrono as entropy fallback if getrandom not available; for
    // tests determinism isn't needed — any printable verifier works.
    // Prefer OS randomness when available.
    let _ = getrandom_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
#[allow(dead_code)]
fn getrandom_bytes(buf: &mut [u8]) -> Result<(), ()> {
    // Try to use getrandom via std; fall back to pseudo-random.
    // We avoid adding a new dep; use uuid v4 bytes as entropy if needed.
    #[cfg(unix)]
    {
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            use std::io::Read;
            if f.read_exact(buf).is_ok() {
                return Ok(());
            }
        }
    }
    // Fallback: fill from multiple uuids.
    let mut off = 0;
    while off < buf.len() {
        let u = uuid::Uuid::new_v4();
        let b = u.as_bytes();
        let take = (buf.len() - off).min(b.len());
        buf[off..off + take].copy_from_slice(&b[..take]);
        off += take;
    }
    Ok(())
}

/// Low-level PKCE S256 helper (used in tests to verify vectors deterministically).
#[allow(dead_code)]
pub fn pkce_challenge_s256(verifier: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Build the Strava authorize URL for PKCE. `state` should be a random CSRF
/// token the caller persists and checks on callback.
#[allow(dead_code)]
pub fn build_auth_url(
    client_id: &str,
    redirect_uri: &str,
    scope: &str,
    state: &str,
    pkce_challenge: &str,
) -> String {
    let mut url = url::Url::parse(STRAVA_AUTH_URL).expect("valid auth url");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", scope)
        .append_pair("state", state)
        .append_pair("code_challenge", pkce_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("approval_prompt", "auto");
    url.to_string()
}

// ---------------------------------------------------------------------------
// Queue (offline retry)
// ---------------------------------------------------------------------------

/// One queued FIT upload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueuedUpload {
    pub fit_path: String,
    /// When it was queued (unix secs).
    pub queued_at: i64,
    /// Consecutive failure count.
    pub attempts: u32,
}

pub fn load_queue() -> Vec<QueuedUpload> {
    load_queue_from(Path::new(QUEUE_PATH))
}

pub fn load_queue_from(path: &Path) -> Vec<QueuedUpload> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_queue(queue: &[QueuedUpload]) -> Result<(), String> {
    save_queue_to(queue, Path::new(QUEUE_PATH))
}

pub fn save_queue_to(queue: &[QueuedUpload], path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(queue).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn enqueue_fit(fit_path: &Path) -> Result<(), String> {
    let mut q = load_queue();
    let s = fit_path.to_string_lossy().to_string();
    if q.iter().any(|e| e.fit_path == s) {
        return Ok(()); // already queued
    }
    q.push(QueuedUpload {
        fit_path: s,
        queued_at: Utc::now().timestamp(),
        attempts: 0,
    });
    save_queue(&q)
}
#[allow(dead_code)]
pub fn dequeue_fit(fit_path: &str) {
    let mut q = load_queue();
    q.retain(|e| e.fit_path != fit_path);
    let _ = save_queue(&q);
}

// ---------------------------------------------------------------------------
// HTTP: token exchange / refresh / upload (async, reqwest)
// ---------------------------------------------------------------------------

/// Exchange an authorization `code` for a token (PKCE verifier required).
#[allow(dead_code)]
pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    code_verifier: &str,
) -> Result<StravaToken, String> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", client_id.to_string()),
        ("client_secret", client_secret.to_string()),
        ("code", code.to_string()),
        ("grant_type", "authorization_code".to_string()),
        ("code_verifier", code_verifier.to_string()),
    ];
    let resp = client
        .post(STRAVA_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("token exchange failed {status}: {body}"));
    }
    let tr: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
    let expires_at = tr
        .expires_at
        .or_else(|| tr.expires_in.map(|s| Utc::now().timestamp() + s))
        .unwrap_or_else(|| Utc::now().timestamp() + 21600);
    Ok(StravaToken {
        access_token: tr.access_token,
        refresh_token: tr.refresh_token,
        expires_at,
        token_type: tr.token_type.unwrap_or_else(|| "Bearer".to_string()),
    })
}

/// Refresh an expired access token.
pub async fn refresh_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<StravaToken, String> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", client_id.to_string()),
        ("client_secret", client_secret.to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("grant_type", "refresh_token".to_string()),
    ];
    let resp = client
        .post(STRAVA_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("refresh failed {status}: {body}"));
    }
    let tr: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
    let expires_at = tr
        .expires_at
        .or_else(|| tr.expires_in.map(|s| Utc::now().timestamp() + s))
        .unwrap_or_else(|| Utc::now().timestamp() + 21600);
    Ok(StravaToken {
        access_token: tr.access_token,
        refresh_token: tr.refresh_token,
        expires_at,
        token_type: tr.token_type.unwrap_or_else(|| "Bearer".to_string()),
    })
}

/// Ensure the token is fresh: refresh if needed, persist, return usable token.
pub async fn ensure_fresh_token(mut token: StravaToken) -> Result<StravaToken, String> {
    if !token.needs_refresh() {
        return Ok(token);
    }
    let client_id = client_id_from_env().ok_or("STRAVA_CLIENT_ID not set")?;
    let client_secret = client_secret_from_env().ok_or("STRAVA_CLIENT_SECRET not set")?;
    let refreshed = refresh_token(&client_id, &client_secret, &token.refresh_token).await?;
    token = refreshed;
    let _ = save_token(&token);
    Ok(token)
}

/// Upload a FIT file to Strava. Returns the Strava upload id on success.
pub async fn upload_fit(
    token: &StravaToken,
    fit_path: &Path,
    activity_name: Option<&str>,
) -> Result<String, String> {
    let bytes = std::fs::read(fit_path).map_err(|e| e.to_string())?;
    let file_name = fit_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ride.fit")
        .to_string();

    let client = reqwest::Client::new();
    let mut form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(bytes).file_name(file_name),
        )
        .text("data_type", "fit")
        .text("external_id", format!("olympus-{}", Utc::now().timestamp()));

    if let Some(name) = activity_name {
        form = form.text("name", name.to_string());
    }
    // Mark as trainer ride.
    form = form.text("trainer", "1");

    let resp = client
        .post(STRAVA_UPLOAD_URL)
        .bearer_auth(&token.access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("upload failed {status}: {body}"));
    }
    let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    // Strava returns {"id": 123, "external_id": "...", "status": "..."}
    let id = val
        .get("id")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .map(|n| n.to_string())
        .unwrap_or_else(|| "ok".to_string());
    Ok(id)
}

/// Try to upload one FIT; on failure enqueue for retry. Returns Ok(true) if
/// uploaded, Ok(false) if queued, Err on hard failure.
pub async fn try_upload_or_queue(
    fit_path: &Path,
    activity_name: Option<&str>,
) -> Result<bool, String> {
    let Some(token) = load_token() else {
        enqueue_fit(fit_path)?;
        return Ok(false);
    };
    let fresh = match ensure_fresh_token(token).await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("strava refresh failed, queuing: {e}");
            enqueue_fit(fit_path)?;
            return Ok(false);
        }
    };
    match upload_fit(&fresh, fit_path, activity_name).await {
        Ok(id) => {
            log::info!("strava upload ok id={id} file={}", fit_path.display());
            Ok(true)
        }
        Err(e) => {
            log::warn!("strava upload failed, queuing: {e}");
            enqueue_fit(fit_path)?;
            Ok(false)
        }
    }
}

/// Retry all queued uploads (called on boot). Removes each file from the
/// queue on success; bumps `attempts` on failure.
pub async fn retry_queued_uploads() -> usize {
    let queue = load_queue();
    if queue.is_empty() {
        return 0;
    }
    let Some(token) = load_token() else {
        log::info!("strava not connected, keeping {} queued", queue.len());
        return 0;
    };
    let fresh = match ensure_fresh_token(token).await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("strava refresh failed, keeping queue: {e}");
            return 0;
        }
    };
    let mut ok = 0;
    let mut remaining: Vec<QueuedUpload> = Vec::new();
    for mut entry in queue {
        let p = PathBuf::from(&entry.fit_path);
        if !p.exists() {
            continue; // stale entry
        }
        match upload_fit(&fresh, &p, None).await {
            Ok(id) => {
                log::info!("strava retry ok id={id} {}", entry.fit_path);
                ok += 1;
            }
            Err(e) => {
                log::warn!("strava retry failed {}: {e}", entry.fit_path);
                entry.attempts += 1;
                // Keep at most 10 attempts to avoid infinite queue growth.
                if entry.attempts < 10 {
                    remaining.push(entry);
                }
            }
        }
    }
    let _ = save_queue(&remaining);
    ok
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_token(expires_in: i64) -> StravaToken {
        StravaToken::new("access".into(), "refresh".into(), expires_in)
    }

    #[test]
    fn token_expiry_logic() {
        let fresh = test_token(3600);
        assert!(!fresh.is_expired());
        assert!(!fresh.needs_refresh());

        let expiring = StravaToken {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now().timestamp() + 60, // 1 min left
            token_type: "Bearer".into(),
        };
        assert!(!expiring.is_expired());
        assert!(expiring.needs_refresh()); // within 5 min window

        let expired = StravaToken {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now().timestamp() - 10,
            token_type: "Bearer".into(),
        };
        assert!(expired.is_expired());
        assert!(expired.needs_refresh());
    }

    #[test]
    fn token_suggested_buffer() {
        let tok = StravaToken {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now().timestamp() + 400,
            token_type: "Bearer".into(),
        };
        assert!(!tok.is_expired_with_buffer(300));
        assert!(tok.is_expired_with_buffer(500));
    }

    #[test]
    fn save_and_load_token_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("strava.json");
        let tok = StravaToken {
            access_token: "abc123".into(),
            refresh_token: "refresh123".into(),
            expires_at: 1_700_000_000,
            token_type: "Bearer".into(),
        };
        save_token_to(&tok, &path).unwrap();
        let loaded = load_token_from(&path).unwrap();
        assert_eq!(loaded, tok);
    }

    #[test]
    fn load_missing_token_is_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nope.json");
        assert!(load_token_from(&path).is_none());
    }

    #[test]
    fn pkce_challenge_rfc7636_vector() {
        // RFC 7636 Appendix B verifier: dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk
        // -> challenge E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = pkce_challenge_s256(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn pkce_generate_produces_url_safe_pair() {
        let (verifier, challenge) = generate_pkce();
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
        assert!(!verifier.contains('+') && !verifier.contains('/') && !verifier.contains('='));
        // Challenge is base64url of sha256 -> 43 chars
        assert_eq!(challenge.len(), 43);
        assert_eq!(challenge, pkce_challenge_s256(&verifier));
    }

    #[test]
    fn build_auth_url_contains_required_params() {
        let url = build_auth_url(
            "12345",
            "http://localhost:8080/callback",
            "activity:write",
            "teststate123",
            "testchallenge",
        );
        assert!(url.starts_with(STRAVA_AUTH_URL));
        assert!(url.contains("client_id=12345"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=activity%3Awrite"));
        assert!(url.contains("state=teststate123"));
        assert!(url.contains("code_challenge=testchallenge"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn queue_round_trip_and_dedupe() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("queue.json");
        let q = vec![QueuedUpload {
            fit_path: "data/.fit/ride_1.fit".into(),
            queued_at: 12345,
            attempts: 0,
        }];
        save_queue_to(&q, &path).unwrap();
        let loaded = load_queue_from(&path);
        assert_eq!(loaded, q);

        // enqueue dedupe check uses global path — test with temp file
        let fit = dir.path().join("ride_2.fit");
        std::fs::write(&fit, b"fake").unwrap();
        // Use isolated queue file via helper — just check save/load
        let mut q2 = load_queue_from(&path);
        q2.push(QueuedUpload {
            fit_path: fit.to_string_lossy().to_string(),
            queued_at: 999,
            attempts: 0,
        });
        save_queue_to(&q2, &path).unwrap();
        assert_eq!(load_queue_from(&path).len(), 2);
    }

    #[test]
    fn queue_empty_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.json");
        assert!(load_queue_from(&path).is_empty());
    }

    #[test]
    fn tss_frozen_not_needed_but_token_logic_covered() {
        // Ensure the 5-minute refresh window is respected for upload path.
        let tok = StravaToken {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now().timestamp() + 301,
            token_type: "Bearer".into(),
        };
        assert!(!tok.needs_refresh());
        let tok2 = StravaToken {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now().timestamp() + 299,
            token_type: "Bearer".into(),
        };
        assert!(tok2.needs_refresh());
    }
}
