//! CEF browser-process handlers. No Electron IPC, Node, or renderer privileges.
//! CEF invokes resource handlers on its IO thread; DB writes occur on the UI thread.
use cef::*;
use std::sync::{Arc, Mutex};
use crate::{profile, shield::{Decision, Shield, STARTER_RULES}, storage::Storage, tabs::Tabs};

/// Shared rule engine compiled once, not once per network request.
pub type Filter = Arc<Mutex<Shield>>;
pub type TabState = Arc<Mutex<Tabs>>;
pub fn default_filter() -> Filter {
    let enabled = Storage::open(&profile::profile_root().join("charlotte.sqlite"))
        .ok().and_then(|db| db.setting("shields").ok().flatten())
        .as_deref() != Some("off");
    Arc::new(Mutex::new(Shield::new(&[STARTER_RULES], enabled)))
}

wrap_client! {
    pub struct CharlotteClient {
        filter: Filter,
        tabs: TabState,
    }
    impl Client {
        fn request_handler(&self) -> Option<RequestHandler> {
            Some(CharlotteRequestHandler::new(self.filter.clone()))
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(CharlotteDisplayHandler::new(self.tabs.clone()))
        }
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(CharlotteLifeSpanHandler::new(self.tabs.clone()))
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(CharlotteLoadHandler::new(self.filter.clone(), self.tabs.clone()))
        }
        fn permission_handler(&self) -> Option<PermissionHandler> {
            Some(CharlottePermissionHandler::new())
        }
        fn download_handler(&self) -> Option<DownloadHandler> {
            Some(CharlotteDownloadHandler::new())
        }
        fn keyboard_handler(&self) -> Option<KeyboardHandler> {
            Some(CharlotteKeyboardHandler::new())
        }
    }
}


fn persist_tabs(state: &TabState) {
    if crate::app::is_restoring(){return}
    if let Ok(tabs) = state.lock() {
        if let Ok(db) = Storage::open(&profile::profile_root().join("charlotte.sqlite")) {
            if let Err(error) = crate::session::save_tabs(&db, &tabs) {
                eprintln!("Cannot persist native tabs: {error}");
            }
        }
    }
}




// Browser shortcuts are intercepted before webpage JavaScript; only known
// combinations are consumed, so site keyboard events remain functional.
wrap_keyboard_handler! {
    struct CharlotteKeyboardHandler;
    impl KeyboardHandler {
        fn on_pre_key_event(
            &self, browser: Option<&mut Browser>, event: Option<&KeyEvent>,
            _os_event: Option<&mut XEvent>, _is_keyboard_shortcut: Option<&mut i32>,
        ) -> i32 {
            let (Some(browser), Some(event)) = (browser, event) else { return 0 };
            let action = crate::shortcuts::resolve(
                event.windows_key_code, event.modifiers, event.type_ == KeyEventType::RAWKEYDOWN
            );
            let Some(action) = action else { return 0 };
            use crate::shortcuts::Shortcut;
            match action {
                Shortcut::Back => { if browser.can_go_back() != 0 { browser.go_back(); } }
                Shortcut::Forward => { if browser.can_go_forward() != 0 { browser.go_forward(); } }
                Shortcut::Reload => browser.reload(),
                Shortcut::HardReload => browser.reload_ignore_cache(),
                Shortcut::Bookmark => {
                    let Some(frame) = browser.main_frame() else { return 0 };
                    let url = frame.url().to_string();
                    if !crate::session::valid_page_url(&url) { return 0 }
                    let Ok(db) = Storage::open(&profile::profile_root().join("charlotte.sqlite")) else { return 0 };
                    match db.toggle_bookmark(&url, &url) {
                        Ok(true) => eprintln!("Bookmarked: {url}"),
                        Ok(false) => eprintln!("Removed bookmark: {url}"),
                        Err(error) => eprintln!("Bookmark toggle failed: {error}"),
                    }
                }
            }
            1
        }
    }
}

// Chromium owns the file-save dialog and downloaded bytes; the Rust host
// records only terminal outcomes. Never trust server-provided filenames.
wrap_download_handler! {
    struct CharlotteDownloadHandler;
    impl DownloadHandler {
        fn can_download(
            &self, _browser: Option<&mut Browser>, url: Option<&CefString>,
            _request_method: Option<&CefString>,
        ) -> i32 {
            url.map(CefString::to_string)
                .is_some_and(|url| crate::session::valid_page_url(&url)) as i32
        }
        fn on_before_download(
            &self, _browser: Option<&mut Browser>,
            _download_item: Option<&mut DownloadItem>,
            _suggested_name: Option<&CefString>,
            callback: Option<&mut BeforeDownloadCallback>,
        ) -> i32 {
            let Some(callback) = callback else { return 0 };
            // CEF opens its Save dialog: no untrusted path is provided by Charlotte.
            callback.cont(None, 1);
            1
        }
        fn on_download_updated(
            &self, _browser: Option<&mut Browser>,
            download_item: Option<&mut DownloadItem>,
            _callback: Option<&mut DownloadItemCallback>,
        ) {
            let Some(item) = download_item else { return };
            let status = if item.is_complete() != 0 {
                Some(crate::downloads::Status::Completed)
            } else if item.is_canceled() != 0 {
                Some(crate::downloads::Status::Canceled)
            } else if item.is_interrupted() != 0 {
                Some(crate::downloads::Status::Interrupted)
            } else { None };
            let Some(status) = status else { return };
            let Some(record) = crate::downloads::Download::finished(
                item.id(), &item.url().to_string(), &item.full_path().to_string(),
                item.received_bytes(), item.total_bytes(), status,
            ) else { return };
            if let Ok(db) = Storage::open(&profile::profile_root().join("charlotte.sqlite")) {
                if let Err(error) = crate::downloads::record(&db, &record) {
                    eprintln!("Unable to record download: {error}");
                }
            }
        }
    }
}

// Permission requests are always resolved in Rust, never by an untrusted
// renderer. Desktop/screen capture is not implied by webcam/mic permission.
wrap_permission_handler! {
    struct CharlottePermissionHandler;
    impl PermissionHandler {
        fn on_request_media_access_permission(
            &self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>,
            requesting_origin: Option<&CefString>,
            requested_permissions: u32,
            callback: Option<&mut MediaAccessCallback>,
        ) -> i32 {
            let Some(callback) = callback else { return 1 };
            let origin = requesting_origin.map(CefString::to_string).unwrap_or_default();
            let result = Storage::open(&profile::profile_root().join("charlotte.sqlite"))
                .ok().and_then(|db| crate::permissions::allowed_media_bits(&db, &origin, requested_permissions).ok())
                .unwrap_or(0);
            if result == requested_permissions && requested_permissions != 0 {
                callback.cont(requested_permissions);
            } else {
                callback.cancel();
            }
            1
        }
        // Other permissions (notably geolocation) remain with Chrome's
        // native permission UI. We do not grant any unrecognized capability.
    }
}

wrap_life_span_handler! {
    struct CharlotteLifeSpanHandler { tabs: TabState, }
    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let Some(browser) = browser else { return };
            if let Ok(mut tabs) = self.tabs.lock() { tabs.open(browser.identifier()); }
            crate::app::post_ui_command(crate::ui_server::Command::Sync);
        }
        fn on_before_close(&self, browser: Option<&mut Browser>) {
            let Some(browser) = browser else { return };
            if let Ok(mut tabs) = self.tabs.lock() { tabs.close(browser.identifier()); }
            persist_tabs(&self.tabs);
            crate::app::post_ui_command(crate::ui_server::Command::Sync);
        }
    }
}

wrap_request_handler! {
    struct CharlotteRequestHandler { filter: Filter, }
    impl RequestHandler {
        fn get_resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _is_navigation: i32,
            _is_download: i32,
            request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut i32>,
        ) -> Option<ResourceRequestHandler> {
            Some(CharlotteResourceRequest::new(
                self.filter.clone(),
                request_initiator.map(CefString::to_string).unwrap_or_default()
            ))
        }

        fn on_certificate_error(
            &self, _browser: Option<&mut Browser>, _error: Errorcode,
            _url: Option<&CefString>, _ssl: Option<&mut Sslinfo>,
            _callback: Option<&mut Callback>,
        ) -> i32 {
            // Do not silently override invalid TLS certificates.
            0
        }
    }
}

wrap_resource_request_handler! {
    struct CharlotteResourceRequest {
        filter: Filter,
        initiator: String,
    }
    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>,
            request: Option<&mut Request>, _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            let Some(request) = request else { return ReturnValue::CONTINUE };
            let url = request.url().to_string();
            if !url.starts_with("https://") && !url.starts_with("http://") {
                return ReturnValue::CONTINUE;
            }
            let source = if self.initiator.is_empty() { request.referrer_url().to_string() }
                         else { self.initiator.clone() };
            let method = request.method().to_string();
            // Debug variants of CEF's resource types map to the adblock engine's
            // ABP resource names. Unknown types are checked as "other".
            let t = format!("{:?}", request.resource_type()).to_ascii_lowercase();
            let kind = if t.contains("script") { "script" }
                else if t.contains("image") { "image" }
                else if t.contains("stylesheet") { "stylesheet" }
                else if t.contains("xhr") || t.contains("fetch") { "xmlhttprequest" }
                else if t.contains("font") { "font" }
                else if t.contains("media") { "media" }
                else if t.contains("sub_frame") { "subdocument" }
                else if t.contains("main_frame") { "document" }
                else { "other" };
            let decision = match self.filter.lock() {
                Ok(filter) => filter.check(&url, &source, kind, &method),
                Err(_) => return ReturnValue::CANCEL,
            };
            match decision {
                Decision::Block => ReturnValue::CANCEL,
                Decision::Rewrite(new_url) => {
                    // Do not let a filter rewrite to file:// or arbitrary protocols.
                    if new_url.starts_with("https://") || new_url.starts_with("http://") {
                        request.set_url(Some(&CefString::from(new_url.as_str())));
                        ReturnValue::CONTINUE
                    } else { ReturnValue::CANCEL }
                }
                Decision::Allow => ReturnValue::CONTINUE,
            }
        }
        fn on_protocol_execution(
            &self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>,
            _request: Option<&mut Request>, allow_os_execution: Option<&mut i32>,
        ) {
            // Unknown schemes must not execute host commands or external handlers.
            if let Some(allow) = allow_os_execution { *allow = 0; }
        }
    }
}

wrap_display_handler! {
    struct CharlotteDisplayHandler { tabs: TabState, }
    impl DisplayHandler {
        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            let Some(browser) = browser else { return };
            let id=browser.identifier();
            let Some(mut browser_view) = browser_view_get_for_browser(Some(browser)) else { return };
            let Some(window) = browser_view.window() else { return };
            window.set_title(title);
            if let Some(title) = title {
                if let Ok(mut tabs) = self.tabs.lock() {
                    tabs.title(id, &title.to_string());
                }
                persist_tabs(&self.tabs);
            }
        }
        fn on_address_change(
            &self, browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            let (Some(frame), Some(url)) = (frame, url) else { return };
            if frame.is_main() == 0 { return }
            let address = url.to_string();
            if !crate::session::valid_page_url(&address) { return }
            if let Some(browser) = browser {
                if let Ok(mut tabs) = self.tabs.lock() {
                    tabs.navigate(browser.identifier(), &address);
                }
                persist_tabs(&self.tabs);
                crate::app::notify_navigation(browser.identifier(),&address);
            }
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
            if let Ok(store) = Storage::open(&profile::profile_root().join("charlotte.sqlite")) {
                if let Err(error) = crate::session::remember_page(&store, &address) {
                    eprintln!("Unable to persist last active page: {error}");
                }
                if let Err(error) = store.record_visit(&address, &address, millis) {
                    eprintln!("Unable to record history: {error}");
                }
            }
        }
    }
}

wrap_load_handler! {
    struct CharlotteLoadHandler { filter: Filter, tabs: TabState, }
    impl LoadHandler {
        fn on_loading_state_change(
            &self, browser: Option<&mut Browser>, is_loading: i32,
            _can_go_back: i32, _can_go_forward: i32,
        ) {
            if let Some(browser) = browser {
                if let Ok(mut tabs) = self.tabs.lock() {
                    tabs.loading(browser.identifier(), is_loading != 0);
                }
            }
        }
        fn on_load_end(
            &self, _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>, _http_status_code: i32,
        ) {
            let Some(frame) = frame else { return };
            if frame.is_main() == 0 { return }
            let url = frame.url().to_string();
            if !(url.starts_with("https://") || url.starts_with("http://")) { return }
            let selectors = match self.filter.lock() {
                Ok(shield) => shield.cosmetics(&url).hide_selectors.into_iter().collect::<Vec<_>>(),
                Err(_) => return,
            };
            if selectors.is_empty() { return }
            // JSON-encode CSS selectors as data, never interpolate them as JS code.
            let Ok(values) = serde_json::to_string(&selectors) else { return };
            // DOM style changes avoid evaluating any scripts from third-party lists.
            let script = format!(r#"
                (() => {{
                    const selectors = {values};
                    const hide = () => {{
                        for (const selector of selectors) {{
                            try {{
                                document.querySelectorAll(selector).forEach(el => {{
                                    el.style.setProperty('display', 'none', 'important');
                                }});
                            }} catch (_) {{}}
                        }}
                    }};
                    hide();
                    let queued = false;
                    new MutationObserver(() => {{
                        if (!queued) {{
                            queued = true;
                            requestAnimationFrame(() => {{ queued = false; hide(); }});
                        }}
                    }}).observe(document.documentElement, {{childList: true, subtree: true}});
                }})();
            "#);
            frame.execute_java_script(
                Some(&CefString::from(script.as_str())),
                Some(&CefString::from(url.as_str())), 0,
            );
        }
    }
}
