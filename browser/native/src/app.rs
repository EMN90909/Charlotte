use cef::*;
use std::cell::RefCell;
pub fn run(main_args: &MainArgs, _command_line: &CommandLine) -> i32 {
    let process_result = execute_process(Some(main_args), None, std::ptr::null_mut());
    if process_result >= 0 { return process_result; }
    if let Err(error) = crate::profile::ensure_profile_dirs() {
        eprintln!("Unable to create Charlotte profile: {error}");
        return 1;
    }
    let _storage = match crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")) {
        Ok(storage) => storage,
        Err(error) => {
            eprintln!("Unable to open Charlotte database: {error}");
            return 1;
        }
    };
    let mut application = CharlotteApp::new();
    let cache = crate::profile::profile_root().join("cache");
    let cache_path = cache.to_string_lossy();
    let settings = Settings {
        no_sandbox: 0,
        root_cache_path: CefString::from(cache_path.as_ref()),
        cache_path: CefString::from(cache_path.as_ref()),
        ..Default::default()
    };
    if initialize(Some(main_args), Some(&settings), Some(&mut application), std::ptr::null_mut()) == 0 { return 1; }
    run_message_loop();
    shutdown();
    0
}
wrap_app! { struct CharlotteApp; impl App { fn browser_process_handler(&self) -> Option<BrowserProcessHandler> { Some(CharlotteBrowserProcess::new(RefCell::new(None))) } } }
wrap_browser_process_handler! {
    struct CharlotteBrowserProcess { client: RefCell<Option<Client>>, }
    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            let client = CharlotteClient::new();
            *self.client.borrow_mut() = Some(client.clone());
            let address = CefString::from(&std::env::var("CHARLOTTE_START_URL").unwrap_or_else(|_| "https://www.google.com/".to_owned()));
            let settings = BrowserSettings::default();
            let mut delegate = CharlotteBrowserViewDelegate::new();
            let mut browser_client = client.clone();
            let browser_view = browser_view_create(Some(&mut browser_client), Some(&address), Some(&settings), None, None, Some(&mut delegate));
            let mut window = CharlotteWindowDelegate::new(RefCell::new(browser_view));
            window_create_top_level(Some(&mut window));
        }
    }
}
wrap_client! { struct CharlotteClient; impl Client {} }
wrap_browser_view_delegate! { struct CharlotteBrowserViewDelegate; impl ViewDelegate {} impl BrowserViewDelegate {} }
wrap_window_delegate! {
    struct CharlotteWindowDelegate { browser_view: RefCell<Option<BrowserView>>, }
    impl ViewDelegate { fn preferred_size(&self, _view: Option<&mut View>) -> Size { Size { width: 1280, height: 820 } } }
    impl PanelDelegate {}
    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) { let view = self.browser_view.borrow(); let (Some(window), Some(browser_view)) = (window, view.as_ref()) else { return; }; let mut browser_view = View::from(browser_view); window.add_child_view(Some(&mut browser_view)); window.show(); }
        fn on_window_destroyed(&self, _window: Option<&mut Window>) { *self.browser_view.borrow_mut() = None; quit_message_loop(); }
        fn can_close(&self, _window: Option<&mut Window>) -> i32 { let view = self.browser_view.borrow(); let Some(browser) = view.as_ref().and_then(BrowserView::browser) else { return 1; }; browser.host().map(|host| host.try_close_browser()).unwrap_or(1) }
    }
}
