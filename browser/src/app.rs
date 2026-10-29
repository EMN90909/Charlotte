//! Native Charlotte CEF Views host: tab strip, Chromium page viewport, AI/chat
//! sidebar. There is no Electron and no Node or JS/OS bridge in this process.
use cef::*;
use std::{cell::{Cell,RefCell},rc::Rc,sync::{Arc,Mutex}};
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
        if let Ok(mut tabs)=self.tabs.lock() {
            tabs.activate(selected);
            if let Ok(store)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
                let _=crate::session::save_tabs(&store,&tabs);
            }
        }
        self.page_panel.layout();
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
        let ready=self.views.iter().all(|v|v.browser().is_some());
        if self.restore_pending && ready {
            self.restore_pending=false;
            if let Some(id)=self.views.get(self.restore_index).and_then(browser_id) {
                self.select(id);return
            }
        }
        // Newly created page tabs are activated when their CEF browser is ready.
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
    fn apply(&mut self,command:Command){
        match command{
            Command::Sync=>self.sync(),
            Command::Shields(enabled)=>{
                if let Ok(db)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
                    if let Err(error)=db.set_setting("shields",if enabled{"on"}else{"off"}){
                        eprintln!("Unable to save Shields setting: {error}");
                        return;
                    }
                }else{return}
                if let Ok(mut shield)=self.filter.lock(){shield.set_enabled(enabled)}
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
            let mut views=Vec::new();
            for url in urls.into_iter().take(crate::tabs::MAX_TABS){
                let mut c=client.clone();
                let mut delegate=CharlotteBrowserViewDelegate::new(self.windows.clone());
                if let Some(view)=browser_view_create(Some(&mut c),Some(&CefString::from(url.as_str())),
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
                restore_pending:true,
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
