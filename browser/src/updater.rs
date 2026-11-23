//! Automatic browser update checker, SHA-256 verifier, staged installer,
//! and server-backed release notes loader for `charlotte://updates`.
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_SERVER_URL: &str = "https://api-charlotte-server.onrender.com";
const KEY_PENDING_UPDATES_VERSION: &str = "updates.pending_notice_version";
const KEY_LAST_ACKNOWLEDGED_VERSION: &str = "updates.last_acknowledged_version";
const KEY_CACHED_UPDATES_JSON: &str = "updates.cached_json";
const KEY_LAST_CHECK_STATUS: &str = "updates.last_check_status";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateHighlight {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowserUpdate {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub id: String,
    pub version: String,
    #[serde(default = "default_channel", deserialize_with = "deserialize_null_default")]
    pub channel: String,
    #[serde(default = "default_all", deserialize_with = "deserialize_null_default")]
    pub platform: String,
    #[serde(default = "default_all", deserialize_with = "deserialize_null_default")]
    pub arch: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub summary: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub release_notes: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub highlights: Vec<UpdateHighlight>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub download_url: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub sha256: String,
    #[serde(default)]
    pub file_size_bytes: i64,
    #[serde(default)]
    pub is_mandatory: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub published_at: String,
}

fn default_channel() -> String {
    "stable".into()
}

fn default_all() -> String {
    "all".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResponse {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub latest: Option<BrowserUpdate>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub recent_updates: Vec<BrowserUpdate>,
    #[serde(default)]
    pub auto_install: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdatesListResponse {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    updates: Vec<BrowserUpdate>,
}

#[derive(Debug, Clone)]
pub struct StartupUpdateOutcome {
    pub checked_server: bool,
    pub update_installed: bool,
    pub open_updates_page: bool,
    pub installed_version: Option<String>,
    pub status_message: String,
}

pub fn server_base_url() -> String {
    std::env::var("CHARLOTTE_SERVER_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SERVER_URL.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

pub fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    }
}

pub fn current_channel() -> String {
    std::env::var("CHARLOTTE_UPDATE_CHANNEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "stable".to_owned())
}

/// Compares two semantic version strings. Returns 1 if a > b, -1 if a < b, 0 if equal.
pub fn compare_semver(a: &str, b: &str) -> i32 {
    let parse = |raw: &str| -> [u64; 3] {
        let trimmed = raw
            .trim()
            .trim_start_matches('v')
            .trim_start_matches('V');
        let core = trimmed.split(['-', '+']).next().unwrap_or("");
        let mut out = [0u64; 3];
        for (i, part) in core.split('.').take(3).enumerate() {
            out[i] = part.parse::<u64>().unwrap_or(0);
        }
        out
    };
    let pa = parse(a);
    let pb = parse(b);
    for i in 0..3 {
        if pa[i] > pb[i] {
            return 1;
        }
        if pa[i] < pb[i] {
            return -1;
        }
    }
    0
}

pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> bool {
    let expected = expected_hex.trim().to_ascii_lowercase();
    if expected.is_empty() {
        return true;
    }
    compute_sha256_hex(bytes) == expected
}

fn http_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(format!("Charlotte/{} ({}; {})", current_version(), current_platform(), current_arch()))
        .build()
        .map_err(|e| e.to_string())
}

/// Checks the Charlotte Cloud server for any newer update.
pub fn check_server_updates(base_url: &str, current_ver: &str) -> Result<UpdateCheckResponse, String> {
    let client = http_client(4)?;
    let endpoint = format!(
        "{}/api/v1/updates/check?version={}&platform={}&arch={}&channel={}",
        base_url.trim_end_matches('/'),
        url::form_urlencoded::byte_serialize(current_ver.as_bytes()).collect::<String>(),
        current_platform(),
        current_arch(),
        current_channel(),
    );
    let resp = client.get(&endpoint).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Server returned HTTP {}", resp.status()));
    }
    resp.json::<UpdateCheckResponse>().map_err(|e| e.to_string())
}

/// Fetches the latest list of published updates directly from the server for `charlotte://updates`.
pub fn fetch_updates_from_server(base_url: &str) -> Result<Vec<BrowserUpdate>, String> {
    let client = http_client(4)?;
    let endpoint = format!(
        "{}/api/v1/updates?platform={}&arch={}&channel={}",
        base_url.trim_end_matches('/'),
        current_platform(),
        current_arch(),
        current_channel(),
    );
    let resp = client.get(&endpoint).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Server returned HTTP {}", resp.status()));
    }
    let parsed = resp.json::<UpdatesListResponse>().map_err(|e| e.to_string())?;
    Ok(parsed.updates)
}

/// Loads updates for `charlotte://updates` in realtime directly from the server.
/// Never fabricates placeholder updates: returns the live server list (possibly empty)
/// or an empty list with the exact server connection error if unreachable.
pub fn load_updates_for_page(_store: &Storage) -> (Vec<BrowserUpdate>, bool, String) {
    let base = server_base_url();
    if std::env::var("CHARLOTTE_OFFLINE_UPDATES").as_deref() == Ok("1") {
        return (
            Vec::new(),
            false,
            format!("Realtime update fetch paused in offline mode (server: {base})"),
        );
    }
    match fetch_updates_from_server(&base) {
        Ok(live) => {
            if live.is_empty() {
                (
                    Vec::new(),
                    true,
                    format!("Connected to {base} — no updates published yet"),
                )
            } else {
                (live, true, format!("Loaded in realtime from {base}"))
            }
        }
        Err(err) => (
            Vec::new(),
            false,
            format!("Unable to load realtime updates from {base}: {err}"),
        ),
    }
}

fn staging_dir_for_version(version: &str) -> PathBuf {
    let safe_ver: String = version
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();
    crate::profile::profile_root()
        .join("updates")
        .join(if safe_ver.is_empty() { "latest".into() } else { safe_ver })
}

/// Launch the independent updater without blocking the desktop UI or
/// attempting to modify the running browser executable. A missing updater
/// is reported, never treated as a successful update.
pub fn spawn_standalone_check() -> Result<(), String> {
    if std::env::var("CHARLOTTE_OFFLINE_UPDATES").as_deref() == Ok("1")
        || std::env::var("CHARLOTTE_TEST_EXIT").as_deref() == Ok("1")
        || std::env::var("CHARLOTTE_HEADLESS_RUN_ONCE").as_deref() == Ok("1")
    {
        return Ok(());
    }
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let sibling = current.parent().ok_or("Charlotte executable has no parent directory")?
        .join(if cfg!(windows) { "charlotte-updater.exe" } else { "charlotte-updater" });
    if !sibling.is_file() {
        return Err(format!("Updater unavailable at {} (check packaged resources)", sibling.display()));
    }
    let mut cmd = std::process::Command::new(sibling);
    cmd.arg("--check");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW for routine checks
    }
    cmd.spawn().map_err(|e| format!("Unable to start separate updater: {e}"))?;
    Ok(())
}

/// Request a verified update installation in a *separate* updater process.
/// Call only in response to explicit user action, not on startup.
pub fn spawn_standalone_install() -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let sibling = current.parent().ok_or("Charlotte executable has no parent directory")?
        .join(if cfg!(windows) { "charlotte-updater.exe" } else { "charlotte-updater" });
    if !sibling.is_file() { return Err("Standalone updater executable is not installed".into()); }
    std::process::Command::new(sibling)
        .arg("--install")
        .spawn().map_err(|e| format!("Could not launch update installer: {e}"))?;
    Ok(())
}

/// Called by the browser during startup. Reading locally cached metadata is
/// safe and immediate; all network update checking happens in the companion
/// charlotte-updater executable. An update notice is not proof it was installed.
pub fn check_and_install_on_startup(store: &Storage) -> StartupUpdateOutcome {
    let pending = store.setting(KEY_PENDING_UPDATES_VERSION)
        .ok().flatten().filter(|value| !value.trim().is_empty());
    StartupUpdateOutcome {
        checked_server: false,
        update_installed: false,
        open_updates_page: pending.as_ref()
            .is_some_and(|ver| compare_semver(ver, current_version()) > 0),
        installed_version: None,
        status_message: last_check_status(store),
    }
}

/// Marks the update screen as seen when the user clicks "Continue" on `charlotte://updates`.
pub fn acknowledge_updates(store: &Storage, version: &str) {
    let _ = store.set_setting(KEY_LAST_ACKNOWLEDGED_VERSION, version);
    let _ = store.remove_setting(KEY_PENDING_UPDATES_VERSION);
}

pub fn pending_update_version(store: &Storage) -> Option<String> {
    store
        .setting(KEY_PENDING_UPDATES_VERSION)
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
}

pub fn last_check_status(store: &Storage) -> String {
    store
        .setting(KEY_LAST_CHECK_STATUS)
        .ok()
        .flatten()
        .unwrap_or_else(|| format!("Charlotte v{} — Auto-updates enabled", current_version()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn semver_comparison_handles_major_minor_patch() {
        assert_eq!(compare_semver("0.5.0", "0.4.0"), 1);
        assert_eq!(compare_semver("0.10.0", "0.9.9"), 1);
        assert_eq!(compare_semver("0.4.0", "0.4.0"), 0);
        assert_eq!(compare_semver("v0.4.0", "0.4.1"), -1);
    }

    #[test]
    fn sha256_verification_matches_and_rejects_tampering() {
        let payload = b"charlotte-update-binary-payload";
        let expected = compute_sha256_hex(payload);
        assert!(verify_sha256(payload, &expected));
        assert!(!verify_sha256(b"tampered-payload", &expected));
    }

    #[test]
    fn pending_update_notice_opens_and_clears_on_continue() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        db.set_setting(KEY_PENDING_UPDATES_VERSION, "0.5.0").unwrap();
        assert_eq!(pending_update_version(&db).as_deref(), Some("0.5.0"));
        acknowledge_updates(&db, "0.5.0");
        assert_eq!(pending_update_version(&db), None);
        assert_eq!(
            db.setting(KEY_LAST_ACKNOWLEDGED_VERSION).unwrap().as_deref(),
            Some("0.5.0")
        );
    }
}
