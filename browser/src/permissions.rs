//! Per-origin permissions, stored in Charlotte's local SQLite settings.
//! Absence of a grant always means denial for direct CEF media callbacks.
//! There is intentionally no wildcard origin or automatic media grant.
use crate::storage::Storage;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Camera,
    Microphone,
    Geolocation,
    Notifications,
    ClipboardRead,
    ClipboardWrite,
    Popups,
    Fullscreen,
    Midi,
    Autoplay,
}

impl Capability {
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "camera" => Some(Self::Camera),
            "microphone" => Some(Self::Microphone),
            "geolocation" => Some(Self::Geolocation),
            "notifications" => Some(Self::Notifications),
            "clipboard_read" | "clipboard-read" => Some(Self::ClipboardRead),
            "clipboard_write" | "clipboard-write" => Some(Self::ClipboardWrite),
            "popups" => Some(Self::Popups),
            "fullscreen" => Some(Self::Fullscreen),
            "midi" => Some(Self::Midi),
            "autoplay" => Some(Self::Autoplay),
            _ => None,
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::Microphone => "microphone",
            Self::Geolocation => "geolocation",
            Self::Notifications => "notifications",
            Self::ClipboardRead => "clipboard_read",
            Self::ClipboardWrite => "clipboard_write",
            Self::Popups => "popups",
            Self::Fullscreen => "fullscreen",
            Self::Midi => "midi",
            Self::Autoplay => "autoplay",
        }
    }
}

/// Site isolation policy: enforces process and cookie partitioning boundaries per origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteIsolationPolicy {
    pub strict_process_isolation: bool,
    pub partition_cookies_by_top_level: bool,
}

impl Default for SiteIsolationPolicy {
    fn default() -> Self {
        Self {
            strict_process_isolation: true,
            partition_cookies_by_top_level: true,
        }
    }
}

impl SiteIsolationPolicy {
    /// Determines whether two origins are cross-site and must be isolated into distinct process boundaries.
    pub fn is_cross_site(origin_a: &str, origin_b: &str) -> bool {
        let (Some(a), Some(b)) = (canonical_origin(origin_a), canonical_origin(origin_b)) else {
            return true;
        };
        a != b
    }

    /// Evaluates whether a subframe is allowed to navigate or access cross-origin resources.
    pub fn is_subframe_navigation_allowed(parent_origin: &str, subframe_url: &str) -> bool {
        let Some(parent) = canonical_origin(parent_origin) else { return false };
        let Some(sub) = canonical_origin(subframe_url) else { return false };
        parent.starts_with("http") && sub.starts_with("http")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision { Allow, Deny }
impl Decision {
    pub fn parse(s: &str) -> Option<Self> {
        match s { "allow" => Some(Self::Allow), "deny" => Some(Self::Deny), _ => None }
    }
    pub const fn as_str(self) -> &'static str {
        match self { Self::Allow => "allowed", Self::Deny => "denied" }
    }
}
/// Reject URL credentials, file/data origins, opaque and malformed origins.
/// A site permission applies to exactly one origin, including its port.
pub fn canonical_origin(raw: &str) -> Option<String> {
    if raw.len() > 2048 { return None; }
    let url = Url::parse(raw).ok()?;
    if !url.username().is_empty() || url.password().is_some() { return None; }
    let host = url.host_str()?;
    if url.scheme() != "https" &&
       !(url.scheme() == "http" && matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]")) {
        return None;
    }
    Some(url.origin().ascii_serialization())
}
fn key(origin: &str, capability: Capability) -> Option<String> {
    let canon = canonical_origin(origin)?;
    Some(format!("perm:v1:{canon}:{}", capability.name()))
}
pub fn get(db: &Storage, origin: &str, capability: Capability) -> rusqlite::Result<Option<Decision>> {
    let Some(k) = key(origin, capability) else { return Ok(None) };
    let value = db.setting(&k)?;
    Ok(match value.as_deref() {
        Some("allowed") => Some(Decision::Allow),
        Some("denied") => Some(Decision::Deny),
        _ => None,
    })
}
pub fn set(db: &Storage, origin: &str, capability: Capability, decision: Decision) -> Result<(), String> {
    let k = key(origin, capability).ok_or_else(|| "Require an HTTPS origin or loopback HTTP origin without credentials".to_owned())?;
    db.set_setting(&k, decision.as_str()).map_err(|e| e.to_string())
}
pub fn reset(db: &Storage, origin: &str, capability: Capability) -> Result<(), String> {
    let k = key(origin, capability).ok_or_else(|| "Invalid origin".to_owned())?;
    db.remove_setting(&k).map_err(|e| e.to_string())?;
    Ok(())
}
/// CEF 156 permission bit mask: NOTIFICATIONS is bit 15; never approve an
/// unrelated permission when multiple bits are combined in one request.
pub const NOTIFICATIONS_BIT:u32 = 1 << 15;
pub fn allowed_notification_prompt(
    db:&Storage, origin:&str, requested:u32
)->rusqlite::Result<Option<Decision>> {
    if requested != NOTIFICATIONS_BIT || canonical_origin(origin).is_none() { return Ok(None) }
    get(db,origin,Capability::Notifications)
}
/// CEF bit flags: device audio=1, device video=2. Desktop screen sharing
/// is never granted by a persisted webcam/mic permission.
pub fn allowed_media_bits(db: &Storage, origin: &str, requested: u32) -> rusqlite::Result<u32> {
    const AUDIO: u32 = 1;
    const VIDEO: u32 = 2;
    const DEVICE_MASK: u32 = AUDIO | VIDEO;
    if requested == 0 || requested & !DEVICE_MASK != 0 { return Ok(0); }
    let mut approved = 0;
    if requested & AUDIO != 0 && get(db, origin, Capability::Microphone)? == Some(Decision::Allow) { approved |= AUDIO }
    if requested & VIDEO != 0 && get(db, origin, Capability::Camera)? == Some(Decision::Allow) { approved |= VIDEO }
    // getUserMedia requires every requested device capability, no partial grants.
    Ok(if approved == requested { requested } else { 0 })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn origin_normalizes_host_case_and_default_port() {
        assert_eq!(canonical_origin("https://ExAmPlE.com:443/path"), Some("https://example.com".into()));
        assert_eq!(canonical_origin("https://example.com/other"), Some("https://example.com".into()));
        assert_ne!(canonical_origin("https://example.com:8443"), canonical_origin("https://example.com"));
    }
    #[test]
    fn rejects_insecure_and_credentialed_origins() {
        for u in ["file:///etc/passwd","data:text/html,abc","http://example.com","https://a:b@example.com","javascript:1"] {
            assert_eq!(canonical_origin(u), None);
        }
        assert!(canonical_origin("http://127.0.0.1:8080").is_some());
    }
    #[test]
    fn notification_permission_is_origin_scoped_and_not_auto_granted() {
        let db=Storage::open(Path::new(":memory:")).unwrap();
        assert_eq!(allowed_notification_prompt(&db,"https://news.example",NOTIFICATIONS_BIT).unwrap(),None);
        set(&db,"https://news.example",Capability::Notifications,Decision::Allow).unwrap();
        assert_eq!(allowed_notification_prompt(&db,"https://news.example/path",NOTIFICATIONS_BIT).unwrap(),Some(Decision::Allow));
        assert_eq!(allowed_notification_prompt(&db,"https://other.example",NOTIFICATIONS_BIT).unwrap(),None);
        assert_eq!(allowed_notification_prompt(&db,"https://news.example",NOTIFICATIONS_BIT | 256).unwrap(),None);
    }
    #[test]
    fn media_grants_are_explicit_and_all_or_nothing() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        let url = "https://example.com/";
        assert_eq!(allowed_media_bits(&db, url, 3).unwrap(), 0);
        set(&db, url, Capability::Camera, Decision::Allow).unwrap();
        assert_eq!(allowed_media_bits(&db, url, 2).unwrap(), 2);
        assert_eq!(allowed_media_bits(&db, url, 3).unwrap(), 0);
        set(&db, url, Capability::Microphone, Decision::Allow).unwrap();
        assert_eq!(allowed_media_bits(&db, url, 3).unwrap(), 3);
        assert_eq!(allowed_media_bits(&db, url, 4).unwrap(), 0);
        assert_eq!(allowed_media_bits(&db, "https://evil.example.com", 2).unwrap(), 0);
        reset(&db, url, Capability::Camera).unwrap();
        assert_eq!(allowed_media_bits(&db, url, 2).unwrap(), 0);
    }
}
