//! Persist only browser navigation state; never record file:// or unsafe URLs.
use crate::storage::Storage;
use url::Url;
const KEY: &str = "last_page_url";
const TABS_KEY: &str = "tabs.v2";
const MAX_SESSION: usize = 64;
pub fn valid_page_url(value: &str) -> bool {
    value.len() <= 8192 &&
    Url::parse(value).ok().is_some_and(|url|
        matches!(url.scheme(), "https" | "http") && url.host_str().is_some() &&
        url.username().is_empty() && url.password().is_none())
}
pub fn remember_page(store: &Storage, url: &str) -> rusqlite::Result<()> {
    if valid_page_url(url) { store.set_setting(KEY, url) } else { Ok(()) }
}
/// Recover saved tab order without restoring unsafe or privileged URLs.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedTabs {
    pub urls: Vec<String>,
    pub active_index: usize,
}
impl SavedTabs {
    pub fn sanitized(self) -> Self {
        let original_active = self.active_index;
        let mut valid = Vec::new();
        let mut new_active = 0;
        for (i, url) in self.urls.into_iter().take(MAX_SESSION).enumerate() {
            if valid_page_url(&url) {
                if i <= original_active { new_active = valid.len(); }
                valid.push(url);
            }
        }
        if valid.is_empty() { new_active = 0; }
        else { new_active = new_active.min(valid.len() - 1); }
        Self { urls: valid, active_index: new_active }
    }
}
pub fn restore_tabs(store: &Storage) -> SavedTabs {
    store.setting(TABS_KEY).ok().flatten()
        .and_then(|s| serde_json::from_str::<SavedTabs>(&s).ok())
        .unwrap_or(SavedTabs { urls: vec![], active_index: 0 })
        .sanitized()
}
pub fn save_tabs(store: &Storage, tabs: &crate::tabs::Tabs) -> rusqlite::Result<()> {
    let (urls, active_index) = tabs.persistable();
    // When every tab closes during normal shutdown, preserve the last good
    // session rather than overwriting it with an empty array.
    if urls.is_empty() { return Ok(()); }
    let data = SavedTabs { urls, active_index }.sanitized();
    let serialized = serde_json::to_string(&data)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    store.set_setting(TABS_KEY, &serialized)
}
pub fn start_page(store: &Storage) -> String {
    let requested = std::env::var("CHARLOTTE_START_URL").ok();
    let session = restore_tabs(store);
    let selected = session.urls.get(session.active_index).cloned();
    let saved = store.setting(KEY).ok().flatten();
    requested.into_iter().chain(selected).chain(saved)
        .find(|url| valid_page_url(url))
        .unwrap_or_else(|| crate::newtab::start_url(store))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn last_navigation_is_restored_and_unsafe_urls_ignored() {
        let db=Storage::open(Path::new(":memory:")).unwrap();
        remember_page(&db, "https://example.com/a").unwrap();
        assert_eq!(db.setting(KEY).unwrap().as_deref(), Some("https://example.com/a"));
        remember_page(&db, "javascript:alert(1)").unwrap();
        assert_eq!(db.setting(KEY).unwrap().as_deref(), Some("https://example.com/a"));
    }
    #[test]
    fn reject_embedded_credentials() {
        assert!(!valid_page_url("https://admin:pass@example.com"));
        assert!(valid_page_url("http://localhost:8080"));
        assert!(!valid_page_url("file:///tmp/secret"));
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn saves_and_restores_order_and_selected_index() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        let mut tabs = crate::tabs::Tabs::default();
        tabs.open(101);
        tabs.navigate(101, "https://first.example");
        tabs.open(102);
        tabs.navigate(102, "https://second.example");
        tabs.activate(101);
        save_tabs(&db, &tabs).unwrap();
        assert_eq!(restore_tabs(&db), SavedTabs {
            urls: vec!["https://first.example".into(), "https://second.example".into()],
            active_index: 0,
        });
        assert_eq!(start_page(&db), "https://first.example");
    }
    #[test]
    fn sanitizes_corrupt_urls_and_bounds() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        db.set_setting(TABS_KEY, r#"{"urls":["javascript:alert(1)","https://a.example","https://b.example"],"active_index":99}"#).unwrap();
        let session = restore_tabs(&db);
        assert_eq!(session.urls, vec!["https://a.example", "https://b.example"]);
        assert_eq!(session.active_index, 1);
    }
    #[test]
    fn fresh_profile_starts_on_local_charlotte_page() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        // The explicit environment override, if present, takes precedence.
        if std::env::var_os("CHARLOTTE_START_URL").is_none() {
            let first = start_page(&db);
            assert!(first.starts_with("data:text/html;charset=utf-8,"));
            assert!(first.len() > 1000);
        }
    }
    #[test]
    fn malformed_profile_falls_back() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        db.set_setting(TABS_KEY, "not-json").unwrap();
        assert_eq!(restore_tabs(&db).urls.len(), 0);
    }
}
