//! Charlotte native tab lifecycle, tab groups, workspaces, and navigation model.
//! Supports horizontal tabs and vertical tabs, tab grouping, workspaces, and
//! resource-conscious tab sleeping.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_TABS: usize = 64;
pub const MAX_CLOSED: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabLayout {
    Horizontal,
    Vertical,
}

impl Default for TabLayout {
    fn default() -> Self {
        Self::Horizontal
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub collapsed: bool,
    pub tab_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub tab_ids: Vec<i32>,
    pub active_tab_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tab {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub loading: bool,
    /// Eco sleep means background rendering is occluded, not destroyed.
    /// Audio, timers and service workers remain owned by engine.
    #[serde(default)]
    pub sleeping: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub media_playing: bool,
    #[serde(default)]
    pub last_accessed_at: u64,
}

impl Tab {
    pub fn safe_url(&self) -> Option<&str> {
        crate::session::valid_page_url(&self.url).then_some(self.url.as_str())
    }
}

fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug)]
pub struct Tabs {
    open: Vec<Tab>,
    active: Option<i32>,
    closed: VecDeque<String>,
    layout: TabLayout,
    groups: Vec<TabGroup>,
    workspaces: Vec<Workspace>,
    active_workspace: String,
    is_private: bool,
}

impl Default for Tabs {
    fn default() -> Self {
        let default_workspace = Workspace {
            id: "default".into(),
            name: "Default".into(),
            icon: "globe".into(),
            tab_ids: Vec::new(),
            active_tab_id: None,
        };
        Self {
            open: Vec::new(),
            active: None,
            closed: VecDeque::new(),
            layout: TabLayout::Horizontal,
            groups: Vec::new(),
            workspaces: vec![default_workspace],
            active_workspace: "default".into(),
            is_private: false,
        }
    }
}

impl Tabs {
    pub fn layout(&self) -> TabLayout {
        self.layout
    }

    pub fn set_layout(&mut self, layout: TabLayout) {
        self.layout = layout;
    }

    pub fn is_private(&self) -> bool {
        self.is_private
    }

    pub fn set_private(&mut self, private: bool) {
        self.is_private = private;
    }

    pub fn pin_tab(&mut self, id: i32, pinned: bool) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        tab.pinned = pinned;
        true
    }

    pub fn groups(&self) -> &[TabGroup] {
        &self.groups
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn active_workspace(&self) -> &str {
        &self.active_workspace
    }

    pub fn open(&mut self, id: i32) -> bool {
        if id <= 0 || self.open.iter().any(|tab| tab.id == id) || self.open.len() >= MAX_TABS {
            return false;
        }
        let now = current_timestamp_secs();
        let ws_id = self.active_workspace.clone();
        self.open.push(Tab {
            id,
            title: "New tab".into(),
            url: String::new(),
            loading: true,
            sleeping: false,
            pinned: false,
            group_id: None,
            workspace_id: Some(ws_id.clone()),
            media_playing: false,
            last_accessed_at: now,
        });
        if let Some(ws) = self.workspaces.iter_mut().find(|w| w.id == ws_id) {
            ws.tab_ids.push(id);
            ws.active_tab_id = Some(id);
        }
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
        // Remove from groups
        for group in &mut self.groups {
            group.tab_ids.retain(|&tid| tid != id);
        }
        // Remove from workspaces
        for ws in &mut self.workspaces {
            ws.tab_ids.retain(|&tid| tid != id);
            if ws.active_tab_id == Some(id) {
                ws.active_tab_id = ws.tab_ids.last().copied();
            }
        }
        if self.active == Some(id) {
            self.active = self.open.get(pos.min(self.open.len().saturating_sub(1))).map(|t| t.id);
        }
        self.update_sleeping();
        true
    }

    pub fn navigate(&mut self, id: i32, url: &str) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        if !crate::session::valid_navigable_url(url) { return false; }
        let normalized = crate::session::normalize_internal_url(url).unwrap_or_else(|| url.to_owned());
        tab.url = normalized;
        tab.loading = true;
        tab.last_accessed_at = current_timestamp_secs();
        true
    }

    /// Restore a saved background tab without making it active.
    pub fn restore_background(&mut self, id: i32, url: &str) -> bool {
        if !crate::session::valid_page_url(url) { return false; }
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        tab.url = url.to_owned();
        tab.title = url::Url::parse(url).ok()
            .and_then(|parsed| parsed.host_str().map(str::to_owned))
            .unwrap_or_else(|| "Restored tab".into());
        tab.loading = false;
        tab.sleeping = true;
        tab.last_accessed_at = current_timestamp_secs();
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
        if let Some(tab) = self.open.iter_mut().find(|t| t.id == id) {
            tab.last_accessed_at = current_timestamp_secs();
            self.active = Some(id);
            if let Some(ws_id) = &tab.workspace_id {
                self.active_workspace = ws_id.clone();
                if let Some(ws) = self.workspaces.iter_mut().find(|w| w.id == *ws_id) {
                    ws.active_tab_id = Some(id);
                }
            }
            self.update_sleeping();
            true
        } else {
            false
        }
    }

    /// Mark active tab not sleeping, background tabs sleeping unless exempt.
    fn update_sleeping(&mut self) {
        for tab in &mut self.open {
            if self.active == Some(tab.id) {
                tab.sleeping = false;
            } else if !tab.media_playing {
                tab.sleeping = true;
            }
        }
    }

    pub fn set_media_playing(&mut self, id: i32, playing: bool) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        tab.media_playing = playing;
        if playing {
            tab.sleeping = false;
        } else if self.active != Some(id) {
            tab.sleeping = true;
        }
        true
    }

    pub fn sleep_tab(&mut self, id: i32) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        if self.active == Some(id) || tab.media_playing {
            return false;
        }
        tab.sleeping = true;
        true
    }

    pub fn wake_tab(&mut self, id: i32) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == id) else { return false };
        tab.sleeping = false;
        tab.last_accessed_at = current_timestamp_secs();
        true
    }

    pub fn auto_sleep_idle_tabs(&mut self, idle_timeout_secs: u64, max_active: usize) -> Vec<i32> {
        let now = current_timestamp_secs();
        let active_id = self.active;
        let mut slept = Vec::new();

        let awake_background_count = self.open.iter().filter(|t| !t.sleeping && Some(t.id) != active_id).count();
        let pressure = awake_background_count > max_active;

        for tab in &mut self.open {
            if Some(tab.id) == active_id || tab.sleeping || tab.media_playing {
                continue;
            }
            let idle = now.saturating_sub(tab.last_accessed_at);
            if idle >= idle_timeout_secs || pressure {
                tab.sleeping = true;
                slept.push(tab.id);
            }
        }
        slept
    }

    // --- Tab Group Methods ---
    pub fn create_group(&mut self, name: &str, color: &str) -> String {
        let id = format!("group_{}", rand::random::<u32>());
        self.groups.push(TabGroup {
            id: id.clone(),
            name: name.chars().take(64).collect(),
            color: color.chars().take(32).collect(),
            collapsed: false,
            tab_ids: Vec::new(),
        });
        id
    }

    pub fn delete_group(&mut self, group_id: &str) -> bool {
        let Some(pos) = self.groups.iter().position(|g| g.id == group_id) else { return false };
        self.groups.remove(pos);
        for tab in &mut self.open {
            if tab.group_id.as_deref() == Some(group_id) {
                tab.group_id = None;
            }
        }
        true
    }

    pub fn add_to_group(&mut self, group_id: &str, tab_id: i32) -> bool {
        if !self.groups.iter().any(|g| g.id == group_id) { return false; }
        let Some(tab) = self.open.iter_mut().find(|t| t.id == tab_id) else { return false };
        // Remove from previous group
        if let Some(prev) = &tab.group_id {
            if let Some(prev_group) = self.groups.iter_mut().find(|g| g.id == *prev) {
                prev_group.tab_ids.retain(|&t| t != tab_id);
            }
        }
        tab.group_id = Some(group_id.to_owned());
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id) {
            if !group.tab_ids.contains(&tab_id) {
                group.tab_ids.push(tab_id);
            }
        }
        true
    }

    pub fn remove_from_group(&mut self, tab_id: i32) -> bool {
        let Some(tab) = self.open.iter_mut().find(|t| t.id == tab_id) else { return false };
        if let Some(prev) = tab.group_id.take() {
            if let Some(group) = self.groups.iter_mut().find(|g| g.id == prev) {
                group.tab_ids.retain(|&t| t != tab_id);
            }
            true
        } else {
            false
        }
    }

    pub fn toggle_group_collapsed(&mut self, group_id: &str) -> bool {
        let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id) else { return false };
        group.collapsed = !group.collapsed;
        true
    }

    // --- Workspaces Methods ---
    pub fn create_workspace(&mut self, name: &str, icon: &str) -> String {
        let id = format!("ws_{}", rand::random::<u32>());
        self.workspaces.push(Workspace {
            id: id.clone(),
            name: name.chars().take(64).collect(),
            icon: icon.chars().take(32).collect(),
            tab_ids: Vec::new(),
            active_tab_id: None,
        });
        id
    }

    pub fn delete_workspace(&mut self, id: &str) -> bool {
        if id == "default" || self.workspaces.len() <= 1 { return false; }
        let Some(pos) = self.workspaces.iter().position(|w| w.id == id) else { return false };
        let removed = self.workspaces.remove(pos);
        // Reassign tabs to default
        for tid in removed.tab_ids {
            if let Some(tab) = self.open.iter_mut().find(|t| t.id == tid) {
                tab.workspace_id = Some("default".into());
            }
            if let Some(def_ws) = self.workspaces.iter_mut().find(|w| w.id == "default") {
                if !def_ws.tab_ids.contains(&tid) {
                    def_ws.tab_ids.push(tid);
                }
            }
        }
        if self.active_workspace == id {
            self.active_workspace = "default".into();
            self.active = self.workspaces.iter().find(|w| w.id == "default").and_then(|w| w.active_tab_id);
            self.update_sleeping();
        }
        true
    }

    pub fn switch_workspace(&mut self, id: &str) -> bool {
        let Some(ws) = self.workspaces.iter().find(|w| w.id == id) else { return false };
        self.active_workspace = id.to_owned();
        if let Some(target_tab) = ws.active_tab_id.or_else(|| ws.tab_ids.first().copied()) {
            self.active = Some(target_tab);
        }
        self.update_sleeping();
        true
    }

    pub fn move_tab_to_workspace(&mut self, tab_id: i32, workspace_id: &str) -> bool {
        if !self.workspaces.iter().any(|w| w.id == workspace_id) { return false; }
        let Some(tab) = self.open.iter_mut().find(|t| t.id == tab_id) else { return false };
        let prev_ws = tab.workspace_id.replace(workspace_id.to_owned());
        if let Some(prev) = prev_ws {
            if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == prev) {
                w.tab_ids.retain(|&tid| tid != tab_id);
                if w.active_tab_id == Some(tab_id) {
                    w.active_tab_id = w.tab_ids.last().copied();
                }
            }
        }
        if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == workspace_id) {
            if !w.tab_ids.contains(&tab_id) {
                w.tab_ids.push(tab_id);
            }
        }
        true
    }

    pub fn active(&self) -> Option<&Tab> {
        let id = self.active?;
        self.open.iter().find(|t| t.id == id)
    }

    pub fn all(&self) -> &[Tab] {
        &self.open
    }

    pub fn active_id(&self) -> Option<i32> {
        self.active
    }

    pub fn last_closed(&mut self) -> Option<String> {
        self.closed.pop_front()
    }

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
        let mut tabs = Tabs::default();
        tabs.open(1);
        tabs.navigate(1, "https://active.example/");
        tabs.open(2);
        tabs.activate(1);
        assert!(tabs.restore_background(2, "https://background.example/path"));
        assert_eq!(tabs.active_id(), Some(1));
        assert_eq!(
            tabs.persistable(),
            (vec!["https://active.example/".into(), "https://background.example/path".into()], 0)
        );
        assert!(!tabs.all()[1].loading);
    }

    #[test]
    fn horizontal_and_vertical_tabs_layout() {
        let mut tabs = Tabs::default();
        assert_eq!(tabs.layout(), TabLayout::Horizontal);
        tabs.set_layout(TabLayout::Vertical);
        assert_eq!(tabs.layout(), TabLayout::Vertical);
    }

    #[test]
    fn tab_groups_creation_and_membership() {
        let mut tabs = Tabs::default();
        tabs.open(10);
        tabs.open(20);
        let gid = tabs.create_group("Research", "#3366cc");
        assert!(tabs.add_to_group(&gid, 10));
        assert!(tabs.add_to_group(&gid, 20));

        let g = tabs.groups().iter().find(|group| group.id == gid).unwrap();
        assert_eq!(g.tab_ids, vec![10, 20]);
        assert_eq!(g.name, "Research");

        assert!(tabs.toggle_group_collapsed(&gid));
        assert!(tabs.groups().iter().find(|group| group.id == gid).unwrap().collapsed);

        assert!(tabs.remove_from_group(10));
        assert!(!tabs.groups().iter().find(|group| group.id == gid).unwrap().tab_ids.contains(&10));
    }

    #[test]
    fn workspaces_management() {
        let mut tabs = Tabs::default();
        tabs.open(1);
        let dev_ws = tabs.create_workspace("Dev", "code");
        assert!(tabs.open(2));
        assert!(tabs.move_tab_to_workspace(2, &dev_ws));

        assert!(tabs.switch_workspace(&dev_ws));
        assert_eq!(tabs.active_workspace(), &dev_ws);
        assert_eq!(tabs.active_id(), Some(2));
    }

    #[test]
    fn tab_sleeping_preserves_media_playing() {
        let mut tabs = Tabs::default();
        tabs.open(1);
        tabs.open(2);
        tabs.activate(1);

        assert!(tabs.all().iter().find(|t| t.id == 2).unwrap().sleeping);
        tabs.set_media_playing(2, true);
        assert!(!tabs.all().iter().find(|t| t.id == 2).unwrap().sleeping);

        // Auto sleep should not sleep tab 2 because media is playing
        let slept = tabs.auto_sleep_idle_tabs(0, 0);
        assert!(!slept.contains(&2));
        assert!(!tabs.all().iter().find(|t| t.id == 2).unwrap().sleeping);
    }
}
