//! Persist only browser navigation state; never record file:// or unsafe URLs.
use crate::storage::Storage;
use url::Url;
const KEY: &str = "last_page_url";
const DEFAULT_URL: &str = "https://www.google.com/";
fn valid_page_url(value: &str) -> bool {
    value.len() <= 8192 &&
    Url::parse(value).ok().is_some_and(|url|
        matches!(url.scheme(), "https" | "http") && url.host_str().is_some() &&
        url.username().is_empty() && url.password().is_none())
}
pub fn remember_page(store: &Storage, url: &str) -> rusqlite::Result<()> {
    if valid_page_url(url) { store.set_setting(KEY, url) } else { Ok(()) }
}
pub fn start_page(store: &Storage) -> String {
    let requested = std::env::var("CHARLOTTE_START_URL").ok();
    let saved = store.setting(KEY).ok().flatten();
    requested.into_iter().chain(saved)
        .find(|url| valid_page_url(url))
        .unwrap_or_else(|| DEFAULT_URL.to_string())
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
