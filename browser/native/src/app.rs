//! Charlotte native CEF application, browser window, popup lifecycle, and profile.
use cef::*;
use std::{cell::{Cell, RefCell}, rc::Rc};
use crate::cef_bridge::{CharlotteClient, default_filter};

pub fn run(main_args: &MainArgs, sandbox_info: *mut u8) -> i32 {
    let process_result = execute_process(Some(main_args), None::<&mut App>, sandbox_info);
    if process_result >= 0 { return process_result; }
    if let Err(error) = crate::profile::ensure_profile_dirs() {
        eprintln!("Unable to create Charlotte profile: {error}"); return 1;
    }
    let store = match crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")) {
        Ok(store) => store,
        Err(error) => { eprintln!("Unable to initialize Charlotte database: {error}"); return 1; }
    };
    if let Some(source) = std::env::var_os("CHARLOTTE_IMPORT_ELECTRON_DIR") {
        match crate::legacy::import_dir(&store, std::path::Path::new(&source)) {
            Ok(report) => eprintln!("Electron data imported: {report:?}"),
            Err(error) => { eprintln!("Electron import failed: {error}"); return 1; }
        }
    }
    let cache_path = crate::profile::profile_root().join("cache");
    let cache = cache_path.to_string_lossy();
    let mut application = CharlotteApp::new();
    let settings = Settings {
        no_sandbox: if cfg!(feature = "sandbox") { 0 } else { 1 },
        root_cache_path: CefString::from(cache.as_ref()),
        cache_path: CefString::from(cache.as_ref()),
        ..Default::default()
    };
    if initialize(Some(main_args), Some(&settings), Some(&mut application), sandbox_info) != 1 { return 1; }
    run_message_loop();
    shutdown();
    0
}

wrap_app! {
    struct CharlotteApp;
    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(CharlotteBrowserProcess::new(RefCell::new(None), Rc::new(Cell::new(0))))
        }
    }
}

wrap_browser_process_handler! {
    struct CharlotteBrowserProcess {
        client: RefCell<Option<Client>>,
        windows: Rc<Cell<usize>>,
    }
    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let client = CharlotteClient::new(default_filter());
            *self.client.borrow_mut() = Some(client.clone());
            let initial = std::env::var("CHARLOTTE_START_URL")
                .unwrap_or_else(|_| "https://www.google.com/".to_string());
            let initial = if initial.starts_with("https://") || initial.starts_with("http://") {
                initial
            } else { "https://www.google.com/".to_string() };
            let url = CefString::from(initial.as_str());
            let settings = BrowserSettings::default();
            let mut delegate = CharlotteBrowserViewDelegate::new(self.windows.clone());
            let mut client_copy = client.clone();
            let browser_view = browser_view_create(
                Some(&mut client_copy), Some(&url), Some(&settings),
                None, None, Some(&mut delegate)
            );
            let mut window = CharlotteWindowDelegate::new(RefCell::new(browser_view), self.windows.clone());
            window_create_top_level(Some(&mut window));
        }
    }
}

wrap_browser_view_delegate! {
    struct CharlotteBrowserViewDelegate { windows: Rc<Cell<usize>>, }
    impl ViewDelegate {}
    impl BrowserViewDelegate {
        fn browser_runtime_style(&self) -> RuntimeStyle { RuntimeStyle::CHROME }
        fn on_popup_browser_view_created(
            &self, _browser_view: Option<&mut BrowserView>,
            popup_browser_view: Option<&mut BrowserView>, _is_devtools: i32,
        ) -> i32 {
            let mut delegate = CharlotteWindowDelegate::new(
                RefCell::new(popup_browser_view.cloned()), self.windows.clone()
            );
            window_create_top_level(Some(&mut delegate));
            1
        }
    }
}

wrap_window_delegate! {
    struct CharlotteWindowDelegate {
        browser_view: RefCell<Option<BrowserView>>,
        windows: Rc<Cell<usize>>,
    }
    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size { width: 1280, height: 820 }
        }
    }
    impl PanelDelegate {}
    impl WindowDelegate {
        fn window_runtime_style(&self) -> RuntimeStyle { RuntimeStyle::CHROME }
        fn on_window_created(&self, window: Option<&mut Window>) {
            let view = self.browser_view.borrow();
            let (Some(window), Some(browser_view)) = (window, view.as_ref()) else { return };
            let mut browser_view = View::from(browser_view);
            window.add_child_view(Some(&mut browser_view));
            self.windows.set(self.windows.get().saturating_add(1));
            window.show();
        }
        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            *self.browser_view.borrow_mut() = None;
            let left = self.windows.get().saturating_sub(1);
            self.windows.set(left);
            if left == 0 { quit_message_loop(); }
        }
        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            let view = self.browser_view.borrow();
            let Some(browser) = view.as_ref().and_then(BrowserView::browser) else { return 1 };
            browser.host().map(|host| host.try_close_browser()).unwrap_or(1)
        }
    }
}
