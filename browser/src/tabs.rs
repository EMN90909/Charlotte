//! Rust-owned CEF tab lifecycle and navigation model.
//! CEF browser IDs are authoritative, not URLs; duplicate-page tabs are valid.
//! Browser objects do not cross threads or get serialized into the profile.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_TABS: usize = 64;
pub const MAX_CLOSED: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tab {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub loading: bool,
    /// Eco sleep means background rendering is occluded, not destroyed.
    /// Audio, timers and service workers remain owned by CEF.
    #[serde(default)]
    pub sleeping: bool,
}

impl Tab {
    pub fn safe_url(&self) -> Option<&str> {
        crate::session::valid_page_url(&self.url).then_some(self.url.as_str())
    }
}

#[derive(Debug, Default)]
pub struct Tabs {
    open: Vec<Tab>,
    active: Option<i32>,
    closed: VecDeque<String>,
}

impl Tabs {
    pub fn open(&mut self, id: i32) -> bool {
        if id <= 0 || self.open.iter().any(|tab| tab.id == id) || self.open.len() >= MAX_TABS {
            return false;
        }
        self.open.push(Tab { id, title: "New tab".into(), url: String::new(), loading: true, sleeping: false });
        self.active = Some(id);
        self.update_sleeping();
        true
    }
    pub fn close(&mut self, id: i32) -> bool {
        let Some(pos) = self.open.iter().position(|t| t.id == id) else { return false };
        let tab = self.open.remove(pos);
        if let Some(url) = tab.safe_url() {
            self.closed.push_front(url.to_owned());
            self.closed.truncate(MAX_CLOSED);
        }
        if self.active == Some(id) {
            self.active = self.open.get(pos.min(self.open.len().saturating_sub(1))).map(|t| t.id);
        }
        self.update_sleeping();
        true
    }
    pub fn navigate(&mut self, id: i32, url: &str) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        if !crate::session::valid_page_url(url) { return false }
        tab.url = url.to_owned();
        tab.loading = true;
        // A hidden CEF page may navigate on its own. Navigation must never
        // activate that tab or unexpectedly interrupt current music/video.
        true
    }
    /// Restore a saved background tab without making it active or initiating
    /// a Chromium navigation. The CEF view starts at about:blank until selected.
    pub fn restore_background(&mut self, id: i32, url: &str) -> bool {
        if !crate::session::valid_page_url(url) { return false; }
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        tab.url = url.to_owned();
        tab.title = url::Url::parse(url).ok()
            .and_then(|parsed| parsed.host_str().map(str::to_owned))
            .unwrap_or_else(|| "Restored tab".into());
        tab.loading = false;
        tab.sleeping = true;
        true
    }
    pub fn title(&mut self, id: i32, title: &str) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        let normalized = title.trim();
        if normalized.is_empty() { return false; }
        tab.title = normalized.chars().take(256).collect();
        true
    }
    pub fn loading(&mut self, id: i32, loading: bool) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        tab.loading = loading;
        true
    }
    pub fn activate(&mut self, id: i32) -> bool {
        if self.open.iter().any(|t| t.id == id) {
            self.active = Some(id);
            self.update_sleeping();
            true
        } else { false }
    }
    /// Do not freeze/discard CEF page processes: that would stop audio,
    /// live calls, web notification contexts and unsaved form state.
    /// Inactive tabs are eco-slept by native View occlusion only.
    fn update_sleeping(&mut self) {
        for tab in &mut self.open {
            tab.sleeping = self.active != Some(tab.id);
        }
    }
    pub fn active(&self) -> Option<&Tab> {
        let id = self.active?;
        self.open.iter().find(|t| t.id == id)
    }
    pub fn all(&self) -> &[Tab] { &self.open }
    pub fn active_id(&self) -> Option<i32> { self.active }
    pub fn last_closed(&mut self) -> Option<String> { self.closed.pop_front() }

    /// Order and active index for crash-safe profile persistence.
    /// Strip transient URLs, which include internal CEF pages and privileged schemes.
    pub fn persistable(&self) -> (Vec<String>, usize) {
        let selected = self.active;
        let mut urls = Vec::new();
        let mut active_index = 0;
        for tab in &self.open {
            if let Some(url) = tab.safe_url() {
                if Some(tab.id) == selected { active_index = urls.len(); }
                urls.push(url.to_string());
            }
        }
        (urls, active_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_pages_use_distinct_cef_ids() {
        let mut tabs = Tabs::default();
        assert!(tabs.open(1));
        assert!(tabs.open(2));
        assert!(!tabs.open(1));
        assert!(tabs.navigate(1, "https://example.com"));
        assert!(tabs.navigate(2, "https://example.com"));
        assert_eq!(tabs.persistable().0.len(), 2);
    }
    #[test]
    fn active_tab_falls_to_neighbor_on_close() {
        let mut tabs = Tabs::default();
        tabs.open(4);
        tabs.open(5);
        tabs.open(6);
        assert!(tabs.close(6));
        assert_eq!(tabs.active_id(), Some(5));
        assert!(tabs.close(4));
        assert_eq!(tabs.active_id(), Some(5));
        assert!(tabs.close(5));
        assert_eq!(tabs.active_id(), None);
    }
    #[test]
    fn invalid_and_privileged_urls_are_not_saved() {
        let mut tabs = Tabs::default();
        tabs.open(1);
        assert!(!tabs.navigate(1, "javascript:alert(1)"));
        assert!(!tabs.navigate(1, "file:///etc/passwd"));
        assert!(!tabs.navigate(1, "https://user:pass@example.org"));
        assert!(tabs.navigate(1, "https://example.com/a"));
        assert_eq!(tabs.persistable().0, vec!["https://example.com/a"]);
    }
    #[test]
    fn closed_tabs_are_limited_to_twenty() {
        let mut tabs = Tabs::default();
        for id in 1..=25 {
            tabs.open(id);
            tabs.navigate(id, &format!("https://example.com/{id}"));
            tabs.close(id);
        }
        assert_eq!(tabs.closed.len(), 20);
        assert_eq!(tabs.last_closed().as_deref(), Some("https://example.com/25"));
    }
    #[test]
    fn background_restore_preserves_active_tab_and_saved_session() {
        let mut tabs=Tabs::default();
        tabs.open(1);
        tabs.navigate(1,"https://active.example/");
        tabs.open(2);
        tabs.activate(1);
        assert!(tabs.restore_background(2,"https://background.example/path"));
        assert_eq!(tabs.active_id(),Some(1));
        assert_eq!(tabs.persistable(),
            (vec!["https://active.example/".into(),"https://background.example/path".into()],0));
        assert!(!tabs.all()[1].loading);
    }
    #[test]
    fn hidden_page_navigation_does_not_steal_active_tab() {
        let mut tabs=Tabs::default();
        tabs.open(1);
        tabs.open(2);
        tabs.activate(1);
        assert!(tabs.navigate(2,"https://background.example/redirect"));
        assert_eq!(tabs.active_id(),Some(1));
        assert!(tabs.all().iter().find(|t|t.id==2).unwrap().sleeping);
    }
    #[test]
    fn eco_sleep_does_not_destroy_background_tab_state() {
        let mut tabs=Tabs::default();
        tabs.open(10);
        tabs.navigate(10,"https://music.example/song");
        tabs.open(20);
        tabs.navigate(20,"https://news.example/");
        assert!(tabs.all().iter().find(|t|t.id==10).unwrap().sleeping);
        assert!(!tabs.all().iter().find(|t|t.id==20).unwrap().sleeping);
        assert_eq!(tabs.persistable().0.len(),2);
        tabs.activate(10);
        assert!(!tabs.all().iter().find(|t|t.id==10).unwrap().sleeping);
        assert!(tabs.all().iter().find(|t|t.id==20).unwrap().sleeping);
    }
    #[test]
    fn caps_open_tabs() {
        let mut tabs = Tabs::default();
        for id in 1..=64 { assert!(tabs.open(id)); }
        assert!(!tabs.open(65));
        assert_eq!(tabs.all().len(), 64);
    }
    #[test]
    fn untrusted_title_is_truncated() {
        let mut tabs = Tabs::default();
        tabs.open(7);
        assert!(tabs.title(7, &"A".repeat(900)));
        assert_eq!(tabs.active().unwrap().title.len(), 256);
    }
}
