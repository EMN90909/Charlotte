//! Native Charlotte CEF Views host: tab strip, Chromium page viewport, AI/chat
//! sidebar. There is no Electron and no Node or JS/OS bridge in this process.
#[cfg(feature = "cef-engine")]
mod cef_app {
use cef::*;
use std::{cell::{Cell,RefCell},collections::HashMap,rc::Rc,sync::{Arc,Mutex,mpsc}};
use serde_json::Value;
use crate::{
    cef_bridge::{CharlotteClient,default_filter,TabState},
    ui_server::{self,Ui,Command},
};
thread_local! {
    // CEF View/Browser references never cross to Tokio request threads.
    static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) };
}
struct Controller {
    _ui: Ui, tabs: TabState, filter: crate::cef_bridge::Filter,
    windows: Rc<Cell<usize>>,
    views: Vec<BrowserView>,
    page_panel: Panel,
    toolbar: BrowserView,
    _sidebar: BrowserView,
    client: Client,
    restore_index: usize,
    restore_pending: bool,
    restore_urls: Vec<String>,
    pending_restore: HashMap<i32,String>,
    devtools_observers: HashMap<i32,Registration>,
    devtools_browsers: HashMap<i32,i32>,
}
fn browser_id(view:&BrowserView)->Option<i32> {
    view.browser().map(|browser| browser.identifier())
}
impl Controller {
    fn refresh_chrome(&self) {
        if let Some(frame)=self.toolbar.browser().and_then(|b|b.main_frame()) {
            let url=CefString::from(self._ui.chrome_url().as_str());
            frame.load_url(Some(&url));
        }
    }
    fn refresh_sidebar(&self) {
        if let Some(frame)=self._sidebar.browser().and_then(|b|b.main_frame()) {
            frame.load_url(Some(&CefString::from(self._ui.sidebar_url().as_str())));
        }
    }
    fn select(&mut self, selected:i32) {
        let Some(index)=self.views.iter().position(|view|browser_id(view)==Some(selected)) else{return};
        for (i,view) in self.views.iter().enumerate() {view.set_visible((i==index) as i32);}
        // Saved background URLs remain in Rust until explicitly selected.
        if let Some(url)=self.pending_restore.remove(&selected) {
            if let Some(frame)=self.views[index].browser().and_then(|b|b.main_frame()){
                frame.load_url(Some(&CefString::from(url.as_str())));
            } else { self.pending_restore.insert(selected,url); }
        }
        if let Ok(mut tabs)=self.tabs.lock() {
            if tabs.active_id()!=Some(selected) {
                tabs.activate(selected);
                if let Ok(store)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
                    let _=crate::session::save_tabs(&store,&tabs);
                }
            }
        }
        self.page_panel.layout();
        self._ui.tab_selected(selected);
        if let Ok(tabs)=self.tabs.lock(){
            if let Some(active)=tabs.active(){self._ui.page_changed(&active.url);}
        }
        self.refresh_chrome();
        self.refresh_sidebar();
    }
    fn active_browser(&self)->Option<Browser>{
        let id=self.tabs.lock().ok()?.active_id()?;
        self.views.iter().filter_map(BrowserView::browser).find(|browser|browser.identifier()==id)
    }
    fn sync(&mut self){
        if self.restore_pending {
            // CEF BrowserView may exist before its Browser is initialized.
            // Wait for all saved browser IDs before restoring selection.
            if !self.views.iter().all(|v|v.browser().is_some()) {return}
            self.restore_pending=false;
            if let Ok(mut tabs)=self.tabs.lock() {
                for (index,view) in self.views.iter().enumerate() {
                    if index==self.restore_index {continue}
                    if let (Some(id),Some(url))=(browser_id(view),self.restore_urls.get(index)){
                        if tabs.restore_background(id,url){
                            self.pending_restore.insert(id,url.to_string());
                        }
                    }
                }
            }
            self.restore_urls.clear();
            if let Some(id)=self.views.get(self.restore_index).and_then(browser_id) {
                self.select(id);return
            }
        }
        let active=self.tabs.lock().ok().and_then(|t|t.active_id());
        if let Some(id)=active {self.select(id);}
    }
    fn add(&mut self, url:String){
        if self.views.len()>=crate::tabs::MAX_TABS {return}
        let initial=if crate::session::valid_page_url(&url) {url} else {
            crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite"))
                .map(|db|crate::newtab::start_url(&db))
                .unwrap_or_else(|_| "https://www.google.com/".into())
        };
        let mut client=self.client.clone();
        let mut delegate=CharlotteBrowserViewDelegate::new(self.windows.clone());
        let view=browser_view_create(Some(&mut client),Some(&CefString::from(initial.as_str())),
            Some(&BrowserSettings::default()),None,None,Some(&mut delegate));
        let Some(view)=view else{return};
        for other in &self.views{other.set_visible(0);}
        let mut native=View::from(&view);
        self.page_panel.add_child_view(Some(&mut native));
        self.views.push(view);
        self.page_panel.layout();
    }
    fn automation_request(&mut self,request_id:i32,browser_id:i32,expected_origin:String,
        method:String,params:Value,reply:mpsc::Sender<Result<Value,String>>){
        // CEF references never leave the UI thread. Prevent stale tab actions.
        if !matches!(method.as_str(),"Runtime.evaluate"|"Page.navigate"|"Input.dispatchMouseEvent"|"Page.captureScreenshot"|"Page.handleJavaScriptDialog"|"Page.getFrameTree"|"Accessibility.getFullAXTree")
            || params.to_string().len()>64*1024 {
            let _=reply.send(Err("Unsupported or oversized Charlotte CDP method".into()));return
        }
        let active=self.tabs.lock().ok().and_then(|tabs|tabs.active_id());
        if active!=Some(browser_id){
            let _=reply.send(Err("Automation tab is no longer active".into()));return
        }
        let Some(browser)=self.active_browser() else{
            let _=reply.send(Err("Automation browser has closed".into()));return
        };
        let page=browser.main_frame().map(|f|f.url().to_string()).unwrap_or_default();
        if crate::permissions::canonical_origin(&page)!=Some(expected_origin) {
            let _=reply.send(Err("Automation paused: page origin changed".into()));return
        }
        let Some(host)=browser.host() else{
            let _=reply.send(Err("CEF browser host unavailable".into()));return
        };
        let id=request_id;
        if id<100_000_000||id>1_900_000_000||self.devtools_observers.len()>=16{
            let _=reply.send(Err("Automation DevTools IDs exhausted".into()));return
        }
        let mut observer=CharlotteAutomationObserver::new(id,reply.clone());
        let Some(registration)=host.add_dev_tools_message_observer(Some(&mut observer)) else{
            let _=reply.send(Err("CEF DevTools observer registration failed".into()));return
        };
        let message=serde_json::json!({"id":id,"method":method,"params":params}).to_string();
        self.devtools_observers.insert(id,registration);
        self.devtools_browsers.insert(id,browser_id);
        if host.send_dev_tools_message(Some(message.as_bytes()))!=1 {
            self.devtools_observers.remove(&id);
            self.devtools_browsers.remove(&id);
            let _=reply.send(Err("CEF refused DevTools method".into()));
        }
    }
    fn close_browser_observers(&mut self,browser_id:i32){
        let ids:Vec<i32>=self.devtools_browsers.iter()
            .filter(|(_,id)|**id==browser_id).map(|(request,_)|*request).collect();
        for id in ids{
            self.devtools_observers.remove(&id);
            self.devtools_browsers.remove(&id);
        }
    }
    fn apply(&mut self,command:Command){
        match command{
            Command::Sync=>self.sync(),
            Command::Automation{request_id,browser_id,expected_origin,method,params,reply}=>
                self.automation_request(request_id,browser_id,expected_origin,method,params,reply),
            Command::AutomationFinished(id)=>{
                self.devtools_observers.remove(&id);
                self.devtools_browsers.remove(&id);
            },
            Command::BrowserClosed(id)=>{
                self._ui.tab_closed(id);
                self.close_browser_observers(id);
            },
            Command::BrowserCrashed(id)=>{
                self._ui.tab_crashed(id);
                self.close_browser_observers(id);
            },
            Command::Shields(enabled)=>{
                let was_enabled=self.filter.read().map(|s|s.is_enabled()).unwrap_or(false);
                if let Ok(db)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
                    if let Err(error)=db.set_setting("shields",if enabled{"on"}else{"off"}){
                        eprintln!("Unable to save Shields setting: {error}");
                        return;
                    }
                }else{return}
                if let Ok(mut shield)=self.filter.write(){shield.set_enabled(enabled)}
                if enabled && !was_enabled {
                    crate::cef_bridge::schedule_filter_refresh(self.filter.clone());
                }
                self.refresh_chrome();
            },
            Command::New(url)=>self.add(url),
            Command::Select(id)=>self.select(id),
            Command::Navigate(url)=>{
                self._ui.page_changed(&url);
                if let Some(browser)=self.active_browser(){
                    if let Some(frame)=browser.main_frame() {
                        frame.load_url(Some(&CefString::from(url.as_str())));
                    }
                }
            },
            Command::Back=>{if let Some(b)=self.active_browser(){if b.can_go_back()!=0{b.go_back()}}},
            Command::Forward=>{if let Some(b)=self.active_browser(){if b.can_go_forward()!=0{b.go_forward()}}},
            Command::Reload=>{if let Some(b)=self.active_browser(){b.reload()}},
            Command::HardReload=>{if let Some(b)=self.active_browser(){b.reload_ignore_cache()}},
            Command::Close(id)=>{
                self._ui.tab_closed(id);
                self.pending_restore.remove(&id);
                if self.views.len()==1{
                    self.add(String::new());
                }
                if let Some(index)=self.views.iter().position(|v|browser_id(v)==Some(id)){
                    let view=self.views.remove(index);
                    if let Some(browser)=view.browser(){
                        if let Some(host)=browser.host(){host.close_browser(0);}
                    }
                    let mut child=View::from(&view);
                    self.page_panel.remove_child_view(Some(&mut child));
                    self.page_panel.layout();
                }
                // CEF lifespan callback updates the actual registry after close.
                post_ui_command(Command::Sync);
            }
        }
    }
}
wrap_dev_tools_message_observer! {
    struct CharlotteAutomationObserver {
        message_id:i32,
        reply:mpsc::Sender<Result<Value,String>>,
    }
    impl DevToolsMessageObserver {
        fn on_dev_tools_agent_detached(&self,_browser:Option<&mut Browser>){
            let _=self.reply.send(Err("Chromium DevTools agent detached; automation must be re-planned".into()));
            post_ui_command(Command::AutomationFinished(self.message_id));
        }
        fn on_dev_tools_method_result(
            &self,_browser:Option<&mut Browser>,message_id:i32,
            success:i32,result:Option<&[u8]>,
        ){
            if message_id!=self.message_id {return}
            let response=if success==0{
                Err(result.and_then(|bytes|serde_json::from_slice::<Value>(bytes).ok())
                    .map(|e|format!("Chromium CDP error: {}",e.to_string().chars().take(240).collect::<String>()))
                    .unwrap_or_else(||"Chromium rejected automation command".into()))
            }else{
                result.ok_or_else(||"Missing Chromium CDP response".to_owned())
                    .and_then(|bytes|{
                        if bytes.len()>2_100_000 {
                            Err("Chromium automation result exceeds 2.1 MB".into())
                        }else{
                            serde_json::from_slice::<Value>(bytes)
                                .map_err(|_|"Malformed Chromium DevTools response".into())
                        }
                    })
            };
            let _=self.reply.send(response);
            post_ui_command(Command::AutomationFinished(message_id));
        }
    }
}
wrap_task! {
    struct UiDispatchTask { command: Command, }
    impl Task {
        fn execute(&self) {
            debug_assert_ne!(currently_on(ThreadId::UI),0);
            CONTROLLER.with(|c|{
                if let Ok(mut state)=c.try_borrow_mut(){
                    if let Some(controller)=state.as_mut(){controller.apply(self.command.clone());}
                }
            });
        }
    }
}
pub fn is_restoring()->bool{
    CONTROLLER.with(|cell|cell.try_borrow().ok()
        .and_then(|c|c.as_ref().map(|controller|controller.restore_pending))
        .unwrap_or(false))
}
pub fn notify_navigation(browser_id:i32,url:&str){
    CONTROLLER.with(|cell|{
        if let Ok(state)=cell.try_borrow() {
            if let Some(controller)=state.as_ref(){
                if controller.tabs.lock().ok().and_then(|t|t.active_id())==Some(browser_id){
                    controller._ui.page_changed(url);
                    controller.refresh_sidebar();
                    controller.refresh_chrome();
                }
            }
        }
    });
}
pub fn post_ui_command(command:Command){
    let mut task=UiDispatchTask::new(command);
    if post_task(ThreadId::UI,Some(&mut task))==0 {
        eprintln!("CEF UI command dispatch failed");
    }
}
pub fn run(main_args: &MainArgs, sandbox_info:*mut u8)->i32 {
    let result=execute_process(Some(main_args),None::<&mut App>,sandbox_info);
    if result>=0{return result}
    if let Err(e)=crate::profile::ensure_profile_dirs(){eprintln!("Profile: {e}");return 1}
    let store=match crate::storage::Storage::open(
        &crate::profile::profile_root().join("charlotte.sqlite")){
        Ok(store)=>store,Err(e)=>{eprintln!("SQLite: {e}");return 1}
    };
    if let Some(path)=std::env::var_os("CHARLOTTE_IMPORT_ELECTRON_DIR"){
        if let Err(e)=crate::legacy::import_dir(&store,std::path::Path::new(&path)){
            eprintln!("Legacy import failed: {e}");return 1
        }
    }
    let cache=crate::profile::profile_root().join("cache");
    let name=cache.to_string_lossy();
    let settings=Settings{
        no_sandbox:if cfg!(feature="sandbox"){0}else{1},
        root_cache_path:CefString::from(name.as_ref()),
        cache_path:CefString::from(name.as_ref()),
        ..Default::default()
    };
    let mut app=CharlotteApp::new();
    if initialize(Some(main_args),Some(&settings),Some(&mut app),sandbox_info)!=1{return 1}
    run_message_loop();
    CONTROLLER.with(|cell|{cell.borrow_mut().take();});
    shutdown();0
}
wrap_app! {
    struct CharlotteApp;
    impl App {
        fn browser_process_handler(&self)->Option<BrowserProcessHandler>{
            Some(CharlotteBrowserProcess::new(Rc::new(Cell::new(0))))
        }
    }
}
wrap_browser_process_handler! {
    struct CharlotteBrowserProcess { windows: Rc<Cell<usize>>, }
    impl BrowserProcessHandler {
        fn on_context_initialized(&self){
            let tabs:TabState=Arc::new(Mutex::new(crate::tabs::Tabs::default()));
            let filter=default_filter();
            let ui=match ui_server::start(tabs.clone(),filter.clone()){
                Ok(ui)=>ui,Err(e)=>{eprintln!("Native UI server: {e}");quit_message_loop();return}
            };
            let client=CharlotteClient::new(filter.clone(),tabs.clone());
            let settings=BrowserSettings::default();
            let mut ui_client=CharlotteLocalClient::new();
            let mut chrome_delegate=CharlotteUiViewDelegate::new(1);
            let chrome=browser_view_create(Some(&mut ui_client),
                Some(&CefString::from(ui.chrome_url().as_str())),Some(&settings),None,None,
                Some(&mut chrome_delegate));
            let mut ui_client2=ui_client.clone();
            let mut chat_delegate=CharlotteUiViewDelegate::new(2);
            let sidebar=browser_view_create(Some(&mut ui_client2),
                Some(&CefString::from(ui.sidebar_url().as_str())),Some(&settings),None,None,
                Some(&mut chat_delegate));
            let (Some(toolbar),Some(sidebar))=(chrome,sidebar) else{
                eprintln!("Unable to initialize native Charlotte toolbar/sidebar");
                quit_message_loop();return;
            };
            let store=crate::storage::Storage::open(
                &crate::profile::profile_root().join("charlotte.sqlite")).ok();
            let restore=store.as_ref().map(crate::session::restore_tabs)
                .unwrap_or(crate::session::SavedTabs{urls:vec![],active_index:0});
            let urls=if !restore.urls.is_empty(){restore.urls.clone()}
                else{vec![store.as_ref().map(crate::session::start_page)
                    .unwrap_or_else(||"https://www.google.com/".into())]};
            let Some(panel)=panel_create(None) else{quit_message_loop();return};
            let _fill=panel.set_to_fill_layout();
            let restore_urls=urls;
            let mut views=Vec::new();
            for (index,url) in restore_urls.iter().take(crate::tabs::MAX_TABS).enumerate(){
                // Avoid network and decode work for pages the user has not
                // selected. A lightweight blank CEF view keeps tab IDs stable.
                let initial=if index==restore.active_index {url.as_str()}else{"about:blank"};
                let mut c=client.clone();
                let mut delegate=CharlotteBrowserViewDelegate::new(self.windows.clone());
                if let Some(view)=browser_view_create(Some(&mut c),Some(&CefString::from(initial)),
                    Some(&settings),None,None,Some(&mut delegate)){
                    view.set_visible((views.len()==restore.active_index) as i32);
                    let mut native=View::from(&view);
                    panel.add_child_view(Some(&mut native));
                    views.push(view);
                }
            }
            if views.is_empty(){quit_message_loop();return;}
            let control=Controller{
                _ui:ui,tabs,filter,windows:self.windows.clone(),views,page_panel:panel.clone(),toolbar:toolbar.clone(),
                _sidebar:sidebar.clone(),client,restore_index:restore.active_index,
                restore_pending:true,restore_urls,pending_restore:HashMap::new(),
                devtools_observers:HashMap::new(),
                devtools_browsers:HashMap::new(),
            };
            CONTROLLER.with(|cell|{cell.replace(Some(control));});
            let mut delegate=CharlotteWindowDelegate::new(toolbar,panel,sidebar,self.windows.clone());
            window_create_top_level(Some(&mut delegate));
        }
    }
}
wrap_client! {struct CharlotteLocalClient;impl Client {}}
wrap_browser_view_delegate! {
    struct CharlotteBrowserViewDelegate { windows: Rc<Cell<usize>>, }
    impl ViewDelegate {}
    impl BrowserViewDelegate {
        fn browser_runtime_style(&self)->RuntimeStyle{RuntimeStyle::ALLOY}
        fn on_popup_browser_view_created(
            &self, _browser_view: Option<&mut BrowserView>,
            popup_browser_view: Option<&mut BrowserView>, _is_devtools: i32,
        )->i32{
            let Some(view)=popup_browser_view else{return 0};
            let mut delegate=CharlottePopupWindowDelegate::new(RefCell::new(Some(view.clone())),self.windows.clone());
            window_create_top_level(Some(&mut delegate));
            1
        }
    }
}
wrap_window_delegate! {
    struct CharlottePopupWindowDelegate { view: RefCell<Option<BrowserView>>, windows: Rc<Cell<usize>>, }
    impl ViewDelegate {
        fn preferred_size(&self,_view:Option<&mut View>)->Size{Size{width:1000,height:740}}
    }
    impl PanelDelegate {}
    impl WindowDelegate {
        fn window_runtime_style(&self)->RuntimeStyle{RuntimeStyle::ALLOY}
        fn on_window_created(&self,window:Option<&mut Window>){
            let (Some(window),Some(view))=(window,self.view.borrow().as_ref().cloned()) else{return};
            let _=window.set_to_fill_layout();
            let mut native=View::from(&view);
            window.add_child_view(Some(&mut native));
            self.windows.set(self.windows.get()+1);
            window.show();
        }
        fn on_window_destroyed(&self,_window:Option<&mut Window>){
            self.view.borrow_mut().take();
            let count=self.windows.get().saturating_sub(1);
            self.windows.set(count);if count==0{quit_message_loop()}
        }
        fn can_close(&self,_window:Option<&mut Window>)->i32{
            let Some(view)=self.view.borrow().as_ref().cloned() else{return 1};
            view.browser().and_then(|browser|browser.host())
                .map(|host|host.try_close_browser()).unwrap_or(1)
        }
    }
}
wrap_browser_view_delegate! {
    struct CharlotteUiViewDelegate { kind:i32, }
    impl ViewDelegate {
        fn preferred_size(&self,_view:Option<&mut View>)->Size{
            if self.kind==1{Size{width:1050,height:102}}
            else{Size{width:340,height:760}}
        }
    }
    impl BrowserViewDelegate {
        fn browser_runtime_style(&self)->RuntimeStyle{RuntimeStyle::ALLOY}
    }
}
wrap_window_delegate! {
    struct CharlotteWindowDelegate {
        toolbar:BrowserView, pages:Panel, sidebar:BrowserView,
        windows:Rc<Cell<usize>>,
    }
    impl ViewDelegate {
        fn preferred_size(&self,_view:Option<&mut View>)->Size{Size{width:1440,height:880}}
    }
    impl PanelDelegate {}
    impl WindowDelegate {
        fn window_runtime_style(&self)->RuntimeStyle{RuntimeStyle::ALLOY}
        fn on_window_created(&self,window:Option<&mut Window>){
            let Some(window)=window else{return};
            let layout=window.set_to_box_layout(Some(&BoxLayoutSettings::default()));
            let mut toolbar=View::from(&self.toolbar);
            window.add_child_view(Some(&mut toolbar));
            let Some(row)=panel_create(None) else{return};
            let mut horizontal=BoxLayoutSettings::default();
            horizontal.horizontal=1;
            let row_layout=row.set_to_box_layout(Some(&horizontal));
            let mut pages=View::from(&self.pages);
            row.add_child_view(Some(&mut pages));
            let mut sidebar=View::from(&self.sidebar);
            row.add_child_view(Some(&mut sidebar));
            if let Some(layout)=row_layout{layout.set_flex_for_view(Some(&mut pages),1);}
            let mut row_view=View::from(&row);
            window.add_child_view(Some(&mut row_view));
            if let Some(layout)=layout{layout.set_flex_for_view(Some(&mut row_view),1);}
            window.layout();
            self.windows.set(self.windows.get()+1);
            window.show();
        }
        fn on_window_destroyed(&self,_window:Option<&mut Window>){
            let count=self.windows.get().saturating_sub(1);
            self.windows.set(count);
            if count==0{quit_message_loop()}
        }
        fn can_close(&self,_window:Option<&mut Window>)->i32{
            CONTROLLER.with(|cell|{
                let state=cell.borrow();
                let Some(controller)=state.as_ref() else{return 1};
                let mut can_close=true;
                for browser in controller.views.iter().filter_map(BrowserView::browser){
                    if let Some(host)=browser.host(){
                        if host.try_close_browser()==0 { can_close=false; }
                    }
                }
                if can_close {1}else{0}
            })
        }
    }
}
}
#[cfg(feature = "cef-engine")]
pub use cef_app::*;

#[cfg(not(feature = "cef-engine"))]
mod platform_app {
    use crate::{
        cef_bridge::{default_filter, TabState},
        engine::create_platform_engine,
        tabs::Tabs,
        ui_server::{self, Command},
    };
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver, Sender},
            Arc, Mutex,
        },
        time::Duration,
    };

    static RESTORING: AtomicBool = AtomicBool::new(false);
    static COMMAND_SENDER: Mutex<Option<Sender<Command>>> = Mutex::new(None);
    static PENDING_COMMANDS: Mutex<Vec<Command>> = Mutex::new(Vec::new());
    static ACTIVE_TABS: Mutex<Option<TabState>> = Mutex::new(None);

    pub fn is_restoring() -> bool {
        RESTORING.load(Ordering::Relaxed)
    }

    pub fn notify_navigation(browser_id: i32, url: &str) {
        if let Ok(guard) = ACTIVE_TABS.lock() {
            if let Some(ref tabs_arc) = *guard {
                if let Ok(mut tabs) = tabs_arc.lock() {
                    tabs.navigate(browser_id, url);
                }
            }
        }
    }

    pub fn post_ui_command(command: Command) {
        if let Ok(guard) = COMMAND_SENDER.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(command);
                return;
            }
        }
        if let Ok(mut lock) = PENDING_COMMANDS.lock() {
            lock.push(command);
        }
    }

    pub fn run_platform() -> Result<(), String> {
        let is_private = std::env::args().any(|a| a == "--private" || a == "--incognito");

        // 1. Ensure profile directories exist
        crate::profile::ensure_profile_dirs().map_err(|e| e.to_string())?;
        let profile_dir = crate::profile::profile_root();
        let db_path = profile_dir.join("charlotte.sqlite");
        let storage = crate::storage::Storage::open(&db_path).map_err(|e| e.to_string())?;

        // 2. Check server for new updates automatically on every launch (normal mode)
        let update_outcome = if !is_private {
            crate::updater::check_and_install_on_startup(&storage)
        } else {
            crate::updater::StartupUpdateOutcome {
                checked_server: false,
                update_installed: false,
                open_updates_page: false,
                installed_version: None,
                status_message: "Private browsing window — ephemeral session active".into(),
            }
        };

        // 3. Setup command channel
        let (tx, rx): (Sender<Command>, Receiver<Command>) = mpsc::channel();
        if let Ok(mut guard) = COMMAND_SENDER.lock() {
            *guard = Some(tx.clone());
        }
        if let Ok(mut pending) = PENDING_COMMANDS.lock() {
            for cmd in pending.drain(..) {
                let _ = tx.send(cmd);
            }
        }

        // 4. Initialize tabs and restore previous session (or open charlotte://updates / charlotte://private)
        let mut tabs_inner = Tabs::default();
        tabs_inner.set_private(is_private);
        let default_start = if is_private {
            "charlotte://private".to_owned()
        } else {
            "charlotte://search".to_owned()
        };

        RESTORING.store(true, Ordering::SeqCst);
        let mut initial_tabs: Vec<(i32, String)> = Vec::new();

        if is_private {
            tabs_inner.open(1);
            tabs_inner.navigate(1, "charlotte://private");
            tabs_inner.title(1, "Private search");
            tabs_inner.activate(1);
            initial_tabs.push((1, "charlotte://private".into()));
        } else {
            let saved_tabs = crate::session::restore_tabs(&storage);
            if !saved_tabs.urls.is_empty() {
                tabs_inner.set_layout(saved_tabs.layout);
                for (idx, url) in saved_tabs.urls.iter().enumerate() {
                    let tab_id = (idx as i32) + 1;
                    tabs_inner.open(tab_id);
                    tabs_inner.navigate(tab_id, url);
                    tabs_inner.title(tab_id, &crate::internal_pages::domain_from_url(url));
                    initial_tabs.push((tab_id, url.clone()));
                }
                if let Some(&(active_id, _)) = initial_tabs.get(saved_tabs.active_index) {
                    tabs_inner.activate(active_id);
                }
            }
            if update_outcome.open_updates_page {
                let upd_id = (initial_tabs.len() as i32) + 1;
                tabs_inner.open(upd_id);
                tabs_inner.navigate(upd_id, "charlotte://updates");
                tabs_inner.title(upd_id, "What's New");
                tabs_inner.activate(upd_id);
                initial_tabs.push((upd_id, "charlotte://updates".into()));
            }
        }
        RESTORING.store(false, Ordering::SeqCst);

        let tab_state: TabState = Arc::new(Mutex::new(tabs_inner));
        if let Ok(mut guard) = ACTIVE_TABS.lock() {
            *guard = Some(tab_state.clone());
        }

        // 5. Initialize Shield ad/tracker filter and local UI server
        let filter = default_filter();
        let ui = ui_server::start(tab_state.clone(), filter.clone())?;

        // 6. Initialize platform web engine
        let mut engine = create_platform_engine();
        for (tab_id, url) in &initial_tabs {
            let _ = engine.render_page(*tab_id, url);
        }

        println!("========================================================================");
        if is_private {
            println!("Charlotte Browser v{} — PRIVATE / INCOGNITO WINDOW (Black Theme)", env!("CARGO_PKG_VERSION"));
        } else {
            println!("Charlotte Browser v{} (Autonomous AI Browser)", env!("CARGO_PKG_VERSION"));
        }
        #[cfg(target_os = "windows")]
        println!("Web Engine: Native Win32 + WebView2 (Windows 10/11; legacy ESR not verified)");
        #[cfg(target_os = "linux")]
        println!("Web Engine: WebKitGTK (Linux x86_64 / i686)");
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        println!("Web Engine: Platform Native Engine");
        println!("Updates:    {}", update_outcome.status_message);
        println!("App UI:     {}", ui.app_url());
        println!("Search:     {} (charlotte://search)", ui.internal_url("search"));
        println!("Agent:      {} (charlotte://agent)", ui.internal_url("agent"));
        println!("Private:    {} (charlotte://private)", ui.internal_url("private"));
        println!("Updates UI: {} (charlotte://updates)", ui.internal_url("updates"));
        println!("History:    {} (charlotte://history)", ui.internal_url("history"));
        println!("Settings:   {} (charlotte://settings)", ui.internal_url("settings"));
        println!("Chrome UI:  {}", ui.chrome_url());
        println!("AI Sidebar: {}", ui.sidebar_url());
        println!("Profile:    {}", profile_dir.display());
        println!("Status:     Running — Ready for navigation & autonomous agent tasks");
        println!("========================================================================");

        // Immediate exit mode for tests / CI smoke checks
        if std::env::var("CHARLOTTE_TEST_EXIT").as_deref() == Ok("1")
            || std::env::var("CHARLOTTE_HEADLESS_RUN_ONCE").as_deref() == Ok("1")
        {
            return Ok(());
        }

        // 7. Keep the Rust command processor active while Win32/WebView2
        // owns the main Windows message loop and displays the native window.
        #[cfg(target_os = "windows")]
        let native_ui = ui.clone();
        #[cfg(target_os = "windows")]
        let (window_tx, window_rx) = mpsc::channel::<crate::windows_host::HostCommand>();
        let mut next_tab_id = (initial_tabs.len() as i32) + 2;

        let worker = move || {
        loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => match cmd {
                    Command::Sync => {
                        if !is_private {
                            crate::cef_bridge::persist_tabs(&tab_state);
                        }
                    }
                    Command::New(url) => {
                        let target_url = if crate::session::valid_navigable_url(&url) {
                            url
                        } else {
                            default_start.clone()
                        };
                        let new_id = next_tab_id;
                        next_tab_id += 1;
                        if let Ok(mut tabs) = tab_state.lock() {
                            if tabs.open(new_id) {
                                tabs.navigate(new_id, &target_url);
                                let title = if crate::session::is_internal_url(&target_url) {
                                    "New tab".to_owned()
                                } else {
                                    crate::internal_pages::domain_from_url(&target_url)
                                };
                                tabs.title(new_id, &title);
                                let _ = engine.render_page(new_id, &target_url);
                                if !is_private {
                                    let _ = crate::session::save_tabs(&storage, &tabs);
                                }
                            }
                        }
                        ui.tab_selected(new_id);
                        ui.page_changed(&target_url);
                    }
                    Command::Select(id) => {
                        if let Ok(mut tabs) = tab_state.lock() {
                            tabs.activate(id);
                            if !is_private {
                                let _ = crate::session::save_tabs(&storage, &tabs);
                            }
                            if let Some(active) = tabs.active() {
                                ui.page_changed(&active.url);
                            }
                        }
                        ui.tab_selected(id);
                    }
                    Command::Close(id) => {
                        if let Ok(mut tabs) = tab_state.lock() {
                            tabs.close(id);
                            let _ = engine.close_tab(id);
                            if !is_private {
                                let _ = crate::session::save_tabs(&storage, &tabs);
                            }
                        }
                        ui.tab_closed(id);
                    }
                    Command::Navigate(url) => {
                        if let Ok(mut tabs) = tab_state.lock() {
                            let target_id = match tabs.active_id() {
                                Some(id) => id,
                                None => {
                                    let id = next_tab_id;
                                    next_tab_id += 1;
                                    tabs.open(id);
                                    id
                                }
                            };
                            if tabs.navigate(target_id, &url) {
                                let title = if crate::session::is_internal_url(&url) {
                                    url.trim_start_matches("charlotte://").to_owned()
                                } else {
                                    crate::internal_pages::domain_from_url(&url)
                                };
                                tabs.title(target_id, &title);
                                let _ = engine.navigate(target_id, &url);
                                if !is_private {
                                    let _ = crate::session::save_tabs(&storage, &tabs);
                                    if crate::session::valid_page_url(&url) {
                                        let now = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis() as i64;
                                        let _ = storage.record_visit(&url, &title, now);
                                    }
                                }
                            }
                        }
                        ui.page_changed(&url);
                    }
                    Command::Back => {
                        #[cfg(target_os = "windows")]
                        let _ = window_tx.send(crate::windows_host::HostCommand::Back);
                        #[cfg(not(target_os = "windows"))]
                        if let Ok(tabs) = tab_state.lock() {
                            if let Some(active_id) = tabs.active_id() {
                                let _ = engine.go_back(active_id);
                            }
                        }
                    }
                    Command::Forward => {
                        #[cfg(target_os = "windows")]
                        let _ = window_tx.send(crate::windows_host::HostCommand::Forward);
                        #[cfg(not(target_os = "windows"))]
                        if let Ok(tabs) = tab_state.lock() {
                            if let Some(active_id) = tabs.active_id() {
                                let _ = engine.go_forward(active_id);
                            }
                        }
                    }
                    Command::Reload | Command::HardReload => {
                        #[cfg(target_os = "windows")]
                        let _ = window_tx.send(crate::windows_host::HostCommand::Reload);
                        #[cfg(not(target_os = "windows"))]
                        if let Ok(tabs) = tab_state.lock() {
                            if let Some(active_id) = tabs.active_id() {
                                let _ = engine.reload(active_id, false);
                            }
                        }
                    }
                    Command::Shields(enabled) => {
                        let _ = storage.set_setting("shields", if enabled { "on" } else { "off" });
                        if let Ok(mut shield) = filter.write() {
                            shield.set_enabled(enabled);
                        }
                    }
                    Command::Automation {
                        request_id: _,
                        browser_id,
                        expected_origin: _,
                        method,
                        params,
                        reply,
                    } => {
                        let res = match method.as_str() {
                            "Runtime.evaluate" => {
                                let expr = params.get("expression").and_then(|v| v.as_str()).unwrap_or_default();
                                engine.execute_script(browser_id, expr)
                            }
                            "Page.navigate" => {
                                let target = params.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                                engine.navigate(browser_id, target).map(|_| serde_json::json!({"frameId": "main"}))
                            }
                            "Page.captureScreenshot" => {
                                engine.capture_screenshot(browser_id).map(|bytes| {
                                    serde_json::json!({
                                        "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
                                    })
                                })
                            }
                            _ => Err(format!("Unsupported method {} in platform engine", method)),
                        };
                        let _ = reply.send(res);
                    }
                    Command::AutomationFinished(_) => {}
                    Command::BrowserClosed(id) => {
                        ui.tab_closed(id);
                    }
                    Command::BrowserCrashed(id) => {
                        ui.tab_crashed(id);
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Idle tick: tab state persistence & periodic cleanup
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
        };

        #[cfg(target_os = "windows")]
        {
            std::thread::Builder::new()
                .name("charlotte-platform-commands".into())
                .spawn(worker)
                .map_err(|e| format!("Failed to start browser commands: {e}"))?;
            return crate::windows_host::run(&native_ui, window_rx);
        }
        #[cfg(not(target_os = "windows"))]
        {
            worker();
            Ok(())
        }
    }
}
#[cfg(not(feature = "cef-engine"))]
pub use platform_app::*;
