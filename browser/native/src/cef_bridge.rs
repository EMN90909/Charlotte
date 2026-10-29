//! CEF browser-process handlers. No Electron IPC, Node, or renderer privileges.
//! CEF invokes resource handlers on its IO thread; DB writes occur on the UI thread.
use cef::*;
use std::sync::{Arc, Mutex};
use crate::{profile, shield::{Decision, Shield, STARTER_RULES}, storage::Storage};

/// Shared rule engine compiled once, not once per network request.
pub type Filter = Arc<Mutex<Shield>>;
pub fn default_filter() -> Filter {
    Arc::new(Mutex::new(Shield::new(&[STARTER_RULES], true)))
}

wrap_client! {
    pub struct CharlotteClient {
        filter: Filter,
    }
    impl Client {
        fn request_handler(&self) -> Option<RequestHandler> {
            Some(CharlotteRequestHandler::new(self.filter.clone()))
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(CharlotteDisplayHandler::new())
        }
    }
}

wrap_request_handler! {
    struct CharlotteRequestHandler { filter: Filter, }
    impl RequestHandler {
        fn resource_request_handler(
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
    struct CharlotteDisplayHandler;
    impl DisplayHandler {
        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            let Some(browser) = browser else { return };
            let Some(mut browser_view) = browser_view_get_for_browser(Some(browser)) else { return };
            let Some(window) = browser_view.window() else { return };
            window.set_title(title);
        }
        fn on_address_change(
            &self, _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            let (Some(frame), Some(url)) = (frame, url) else { return };
            if frame.is_main() == 0 { return }
            let address = url.to_string();
            if !(address.starts_with("https://") || address.starts_with("http://")) { return }
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
            if let Ok(store) = Storage::open(&profile::profile_root().join("charlotte.sqlite")) {
                if let Err(error) = store.record_visit(&address, &address, millis) {
                    eprintln!("Unable to record history: {error}");
                }
            }
        }
    }
}
