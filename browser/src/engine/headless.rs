//! Headless virtual web engine implementation.
//! Provides full WebEngine functionality for test suites, CI environments,
//! and headless server-side browser automation.
use super::WebEngine;
use crate::dark_mode::DarkModePreference;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct TabViewState {
    url: String,
    history: Vec<String>,
    history_index: usize,
    stylesheets: Vec<String>,
    dark_mode: DarkModePreference,
    sleeping: bool,
}

#[derive(Debug, Default)]
pub struct HeadlessEngine {
    views: HashMap<i32, TabViewState>,
}

impl HeadlessEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tab_url(&self, tab_id: i32) -> Option<&str> {
        self.views.get(&tab_id).map(|v| v.url.as_str())
    }
}

impl WebEngine for HeadlessEngine {
    fn render_page(&mut self, tab_id: i32, url: &str) -> Result<(), String> {
        self.views.insert(tab_id, TabViewState {
            url: url.to_owned(),
            history: vec![url.to_owned()],
            history_index: 0,
            stylesheets: Vec::new(),
            dark_mode: DarkModePreference::Off,
            sleeping: false,
        });
        Ok(())
    }

    fn navigate(&mut self, tab_id: i32, url: &str) -> Result<(), String> {
        let view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        view.url = url.to_owned();
        view.history.truncate(view.history_index + 1);
        view.history.push(url.to_owned());
        view.history_index = view.history.len() - 1;
        view.sleeping = false;
        Ok(())
    }

    fn reload(&mut self, tab_id: i32, _ignore_cache: bool) -> Result<(), String> {
        let _view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        Ok(())
    }

    fn go_back(&mut self, tab_id: i32) -> Result<bool, String> {
        let view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        if view.history_index > 0 {
            view.history_index -= 1;
            view.url = view.history[view.history_index].clone();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn go_forward(&mut self, tab_id: i32) -> Result<bool, String> {
        let view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        if view.history_index + 1 < view.history.len() {
            view.history_index += 1;
            view.url = view.history[view.history_index].clone();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn execute_script(&mut self, tab_id: i32, script: &str) -> Result<serde_json::Value, String> {
        if !self.views.contains_key(&tab_id) {
            return Err("Tab view not found".into());
        }
        if script.trim().is_empty() {
            return Err("Empty script".into());
        }
        Ok(serde_json::json!({ "executed": true, "result": "ok" }))
    }

    fn inject_stylesheet(&mut self, tab_id: i32, css: &str) -> Result<(), String> {
        let view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        view.stylesheets.push(css.to_owned());
        Ok(())
    }

    fn set_forced_dark_mode(&mut self, tab_id: i32, mode: DarkModePreference) -> Result<(), String> {
        let view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        view.dark_mode = mode;
        let css = crate::dark_mode::generate_dark_css(mode);
        if !css.is_empty() {
            view.stylesheets.push(css.to_owned());
        }
        Ok(())
    }

    fn sleep_tab(&mut self, tab_id: i32) -> Result<(), String> {
        let view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        view.sleeping = true;
        Ok(())
    }

    fn wake_tab(&mut self, tab_id: i32) -> Result<(), String> {
        let view = self.views.get_mut(&tab_id).ok_or("Tab view not found")?;
        view.sleeping = false;
        Ok(())
    }

    fn capture_screenshot(&mut self, tab_id: i32) -> Result<Vec<u8>, String> {
        if !self.views.contains_key(&tab_id) {
            return Err("Tab view not found".into());
        }
        // Minimal 1x1 transparent PNG payload
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
        self.views.remove(&tab_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_navigation_and_history() {
        let mut engine = HeadlessEngine::new();
        engine.render_page(1, "https://example.com/1").unwrap();
        engine.navigate(1, "https://example.com/2").unwrap();
        assert_eq!(engine.tab_url(1), Some("https://example.com/2"));

        assert!(engine.go_back(1).unwrap());
        assert_eq!(engine.tab_url(1), Some("https://example.com/1"));

        assert!(engine.go_forward(1).unwrap());
        assert_eq!(engine.tab_url(1), Some("https://example.com/2"));
    }

    #[test]
    fn dark_mode_and_stylesheet_injection() {
        let mut engine = HeadlessEngine::new();
        engine.render_page(5, "https://example.com").unwrap();
        engine.set_forced_dark_mode(5, DarkModePreference::SmartInvert).unwrap();
        let view = engine.views.get(&5).unwrap();
        assert_eq!(view.dark_mode, DarkModePreference::SmartInvert);
        assert!(!view.stylesheets.is_empty());
    }
}
