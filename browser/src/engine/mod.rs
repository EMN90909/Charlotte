//! Platform web engine abstractions for Charlotte.
//! Supports WebKitGTK on Linux, WebView2 on Windows, and Headless for headless/test environments.
pub mod webkitgtk;
pub mod webview2;
pub mod headless;

pub use headless::HeadlessEngine;
pub use webkitgtk::WebKitGtkEngine;
pub use webview2::WebView2Engine;

use crate::dark_mode::DarkModePreference;

/// Unified web engine trait implemented by platform renderers (WebKitGTK, WebView2, Headless).
pub trait WebEngine: Send + Sync {
    /// Renders a new web page or view for a given tab ID.
    fn render_page(&mut self, tab_id: i32, url: &str) -> Result<(), String>;

    /// Navigates the tab to a target URL.
    fn navigate(&mut self, tab_id: i32, url: &str) -> Result<(), String>;

    /// Reloads the page.
    fn reload(&mut self, tab_id: i32, ignore_cache: bool) -> Result<(), String>;

    /// Navigates back in history.
    fn go_back(&mut self, tab_id: i32) -> Result<bool, String>;

    /// Navigates forward in history.
    fn go_forward(&mut self, tab_id: i32) -> Result<bool, String>;

    /// Executes JavaScript in the main frame of the specified tab.
    fn execute_script(&mut self, tab_id: i32, script: &str) -> Result<serde_json::Value, String>;

    /// Injects a custom CSS stylesheet into the tab.
    fn inject_stylesheet(&mut self, tab_id: i32, css: &str) -> Result<(), String>;

    /// Applies forced dark mode styles to the tab.
    fn set_forced_dark_mode(&mut self, tab_id: i32, mode: DarkModePreference) -> Result<(), String>;

    /// Puts the tab view to sleep (occlusion or timer throttling).
    fn sleep_tab(&mut self, tab_id: i32) -> Result<(), String>;

    /// Wakes a sleeping tab.
    fn wake_tab(&mut self, tab_id: i32) -> Result<(), String>;

    /// Captures a screenshot of the tab viewport as a PNG/JPEG byte buffer.
    fn capture_screenshot(&mut self, tab_id: i32) -> Result<Vec<u8>, String>;

    /// Closes and deallocates the web view for the tab.
    fn close_tab(&mut self, tab_id: i32) -> Result<(), String>;
}

/// Creates the appropriate web engine for the target operating system.
pub fn create_platform_engine() -> Box<dyn WebEngine> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WebView2Engine::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(WebKitGtkEngine::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Box::new(HeadlessEngine::new())
    }
}
