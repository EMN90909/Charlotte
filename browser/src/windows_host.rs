//! Actual Windows desktop host for the default (non-CEF) Charlotte build.
//! Wry owns a native Win32 window and real Microsoft WebView2 controls.
//! The Rust/axum UI server remains on loopback; no public server is introduced.
#![cfg(target_os = "windows")]

use crate::{cef_bridge::TabState, ui_server::Ui};
use std::time::{Duration, Instant};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::{
    dpi::{PhysicalPosition, PhysicalSize},
    Rect, WebView, WebViewBuilder,
};

const TOOLBAR_HEIGHT: u32 = 100;

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        position: PhysicalPosition::new(x as i32, y as i32).into(),
        size: PhysicalSize::new(width.max(1), height.max(1)).into(),
    }
}

/// Convert an internal Charlotte URL into the local server route. External
/// webpages are loaded by a real WebView2 renderer, not the state-only engine.
fn display_url(raw: &str, base: &str) -> String {
    if let Some(route) = crate::ui_server::internal_route_for_url(raw) {
        format!("{base}{route}")
    } else if crate::session::valid_page_url(raw) {
        raw.to_owned()
    } else {
        format!("{base}/internal/search")
    }
}

fn active_url(tabs: &TabState) -> Option<String> {
    tabs.lock()
        .ok()
        .and_then(|state| state.active().map(|tab| tab.url.clone()))
}

fn resize_views(chrome: &WebView, page: &WebView, width: u32, height: u32) {
    let top = TOOLBAR_HEIGHT.min(height);
    let _ = chrome.set_bounds(rect(0, 0, width, top));
    let _ = page.set_bounds(rect(0, top, width, height.saturating_sub(top)));
}

/// This must run on the main Windows GUI/COM thread. The command processor
/// runs on a worker thread and updates the shared active tab state.
pub fn run(ui: &Ui) -> Result<(), String> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Charlotte Browser")
        .with_inner_size(LogicalSize::new(1360.0, 860.0))
        .with_min_inner_size(LogicalSize::new(780.0, 520.0))
        .build(&event_loop)
        .map_err(|err| format!("Could not create Charlotte window: {err}"))?;

    let base = format!("http://127.0.0.1:{}", ui.port);
    let initial_url = active_url(&ui.tabs)
        .map(|url| display_url(&url, &base))
        .unwrap_or_else(|| format!("{base}/app"));
    let size = window.inner_size();
    let chrome = WebViewBuilder::new()
        .with_url(&ui.chrome_url())
        .with_bounds(rect(0, 0, size.width, TOOLBAR_HEIGHT))
        .build_as_child(&window)
        .map_err(|err| format!("Could not initialize Charlotte WebView2 toolbar: {err}"))?;
    let page = WebViewBuilder::new()
        .with_url(&initial_url)
        .with_bounds(rect(0, TOOLBAR_HEIGHT, size.width, size.height.saturating_sub(TOOLBAR_HEIGHT)))
        .build_as_child(&window)
        .map_err(|err| format!("Could not initialize Charlotte WebView2 page: {err}. Check the WebView2 runtime installation."))?;

    let tabs = ui.tabs.clone();
    let mut last_tab_url = active_url(&tabs);
    let mut last_refresh = Instant::now();
    let mut last_tab_count = tabs.lock().ok().map(|state| state.all().len()).unwrap_or(0);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(150));
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                resize_views(&chrome, &page, size.width, size.height);
            }
            Event::MainEventsCleared => {
                let current = active_url(&tabs);
                if current != last_tab_url {
                    let target = current.as_deref().unwrap_or("charlotte://search");
                    if let Err(err) = page.load_url(&display_url(target, &base)) {
                        eprintln!("Charlotte navigation failed: {err}");
                    }
                    last_tab_url = current;
                    // Refresh native tab strip after navigation/selection.
                    let _ = chrome.load_url(&format!("{base}/chrome"));
                    last_refresh = Instant::now();
                }
                let count = tabs.lock().ok().map(|state| state.all().len()).unwrap_or(0);
                if count != last_tab_count || last_refresh.elapsed() >= Duration::from_secs(5) {
                    let _ = chrome.load_url(&format!("{base}/chrome"));
                    last_refresh = Instant::now();
                    last_tab_count = count;
                }
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_routing_does_not_expose_internal_scheme_to_webview2() {
        let base = "http://127.0.0.1:9130";
        assert_eq!(display_url("charlotte://settings", base), format!("{base}/internal/settings"));
        assert_eq!(display_url("https://example.org", base), "https://example.org");
        assert_eq!(display_url("file:///C:/Windows", base), format!("{base}/internal/search"));
    }
}
