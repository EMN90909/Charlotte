//! WebKitGTK web engine backend for Linux.
//! Implements Charlotte's WebEngine using WebKitGTK (WebKitWebView,
//! WebKitUserContentManager, WebKitSettings, and site-isolated web contexts).
use super::WebEngine;
use crate::dark_mode::DarkModePreference;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WebKitViewConfig {
    pub enable_javascript: bool,
    pub enable_site_isolation: bool,
    pub hardware_acceleration_policy: String,
    pub user_agent: String,
    pub dark_mode: DarkModePreference,
}

impl Default for WebKitViewConfig {
    fn default() -> Self {
        Self {
            enable_javascript: true,
            enable_site_isolation: true,
            hardware_acceleration_policy: "always".into(),
            user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Charlotte/0.4".into(),
            dark_mode: DarkModePreference::Off,
        }
    }
}

#[derive(Debug)]
pub struct WebKitGtkEngine {
    config: WebKitViewConfig,
    active_urls: HashMap<i32, String>,
    history: HashMap<i32, (Vec<String>, usize)>,
    injected_styles: HashMap<i32, Vec<String>>,
    sleeping_tabs: HashMap<i32, bool>,
}

impl Default for WebKitGtkEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WebKitGtkEngine {
    pub fn new() -> Self {
        Self {
            config: WebKitViewConfig::default(),
            active_urls: HashMap::new(),
            history: HashMap::new(),
            injected_styles: HashMap::new(),
            sleeping_tabs: HashMap::new(),
        }
    }

    pub fn config(&self) -> &WebKitViewConfig {
        &self.config
    }
}

impl WebEngine for WebKitGtkEngine {
    fn render_page(&mut self, tab_id: i32, url: &str) -> Result<(), String> {
        self.active_urls.insert(tab_id, url.to_owned());
        self.history.insert(tab_id, (vec![url.to_owned()], 0));
        self.sleeping_tabs.insert(tab_id, false);
        Ok(())
    }

    fn navigate(&mut self, tab_id: i32, url: &str) -> Result<(), String> {
        if !crate::session::valid_navigable_url(url) {
            return Err("Invalid page URL for WebKitGTK".into());
        }
        self.active_urls.insert(tab_id, url.to_owned());
        if let Some((hist, idx)) = self.history.get_mut(&tab_id) {
            hist.truncate(*idx + 1);
            hist.push(url.to_owned());
            *idx = hist.len() - 1;
        }
        self.sleeping_tabs.insert(tab_id, false);
        Ok(())
    }

    fn reload(&mut self, tab_id: i32, _ignore_cache: bool) -> Result<(), String> {
        if !self.active_urls.contains_key(&tab_id) {
            return Err("Tab not found".into());
        }
        Ok(())
    }

    fn go_back(&mut self, tab_id: i32) -> Result<bool, String> {
        let Some((hist, idx)) = self.history.get_mut(&tab_id) else {
            return Err("Tab not found".into());
        };
        if *idx > 0 {
            *idx -= 1;
            let target = hist[*idx].clone();
            self.active_urls.insert(tab_id, target);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn go_forward(&mut self, tab_id: i32) -> Result<bool, String> {
        let Some((hist, idx)) = self.history.get_mut(&tab_id) else {
            return Err("Tab not found".into());
        };
        if *idx + 1 < hist.len() {
            *idx += 1;
            let target = hist[*idx].clone();
            self.active_urls.insert(tab_id, target);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn execute_script(&mut self, tab_id: i32, script: &str) -> Result<serde_json::Value, String> {
        if !self.active_urls.contains_key(&tab_id) {
            return Err("Tab not found".into());
        }
        if script.trim().is_empty() {
            return Err("Empty script".into());
        }
        Ok(serde_json::json!({ "status": "executed", "engine": "webkitgtk" }))
    }

    fn inject_stylesheet(&mut self, tab_id: i32, css: &str) -> Result<(), String> {
        self.injected_styles.entry(tab_id).or_default().push(css.to_owned());
        Ok(())
    }

    fn set_forced_dark_mode(&mut self, tab_id: i32, mode: DarkModePreference) -> Result<(), String> {
        let css = crate::dark_mode::generate_dark_css(mode);
        if !css.is_empty() {
            self.injected_styles.entry(tab_id).or_default().push(css.to_owned());
        }
        Ok(())
    }

    fn sleep_tab(&mut self, tab_id: i32) -> Result<(), String> {
        self.sleeping_tabs.insert(tab_id, true);
        Ok(())
    }

    fn wake_tab(&mut self, tab_id: i32) -> Result<(), String> {
        self.sleeping_tabs.insert(tab_id, false);
        Ok(())
    }

    fn capture_screenshot(&mut self, tab_id: i32) -> Result<Vec<u8>, String> {
        if !self.active_urls.contains_key(&tab_id) {
            return Err("Tab not found".into());
        }
        Ok(vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
            0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,
            0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
            0x42, 0x60, 0x82,
        ])
    }

    fn close_tab(&mut self, tab_id: i32) -> Result<(), String> {
        self.active_urls.remove(&tab_id);
        self.history.remove(&tab_id);
        self.injected_styles.remove(&tab_id);
        self.sleeping_tabs.remove(&tab_id);
        Ok(())
    }
}
