//! Local browser chrome + sidebar server. The page is CEF-rendered HTML/CSS
//! but commands, chat, AI requests, profiles and secrets are owned by Rust.
//! Bound only to 127.0.0.1 with an unpredictable per-launch form token.
//! There is no Electron, Node process, websocket proxy or JS-facing OS API.
use std::{net::TcpListener, sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}, thread};
use axum::{extract::{State,Form},http::{HeaderMap,StatusCode,header},response::{Html,IntoResponse,Redirect,Response},routing::{get,post},Json,Router};
use serde::Deserialize;
use crate::{agent::{self,Plan},browser_agent::{self,Decision as BrowserDecision},browser_automation::{self,Action as BrowserAction,Context as BrowserContext},cef_bridge::{Filter,TabState}, mcp, permissions::{self,Capability,Decision}, services::{self, ChatIdentity, Provider, Turn}};
#[derive(Clone, Debug)]
pub enum Command {
    Sync, New(String), Select(i32), Close(i32), Navigate(String),
    Back, Forward, Reload, HardReload, Shields(bool),
    Automation {
        request_id:i32,browser_id:i32,expected_origin:String,method:String,
        params:serde_json::Value,
        reply:std::sync::mpsc::Sender<Result<serde_json::Value,String>>,
    },
    AutomationFinished(i32),
}
#[derive(Clone)]
pub struct Ui {
    pub port: u16,
    pub token: String,
    pub tabs: TabState,
    pub filter: Filter,
    inner: Arc<Mutex<UiInner>>,
}
#[derive(Clone)]
struct PendingTool {
    name:String, server_id:String, args:serde_json::Value,
    question:String, provider:String,
}
#[derive(Clone)]
struct PendingBrowser {
    context:BrowserContext, action:BrowserAction, goal:String,
    provider:String, step:u8, observed_url:String, target_label:String, visual:bool,
    screenshot_fingerprint:Option<u64>,
}
struct UiInner {
    pending_browser:Option<PendingBrowser>,
    pending_tool:Option<PendingTool>,
    ai: Vec<Turn>,
    chat: Vec<(String,String)>,
    chat_base: String,
    chat_token: String,
    chat_cursor: u64,
    room_url: String,
    error: String,
    working: bool,
    stop: Arc<AtomicBool>,
}
#[derive(Deserialize)]
struct ActionForm {
    token: String,
    command: Option<String>,
    id: Option<i32>,
    value: Option<String>,
    server: Option<String>,
    username: Option<String>,
    provider: Option<String>,
    mode: Option<String>,
    prompt_text: Option<String>,
}
fn escape(s: &str) -> String {
    s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;")
        .replace('"',"&quot;").replace('\'',"&#39;")
}
fn input(name: &str, value: &str) -> String {
    format!("<input type=\"hidden\" name=\"{}\" value=\"{}\">",escape(name),escape(value))
}
fn header_html(title: &str, compact: bool) -> String {
    let width=if compact {"100%"} else {"100%"};
    format!(r#"<!doctype html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; base-uri 'none'; object-src 'none'">
<title>{}</title><style>
:root{{color-scheme:dark;--bg:#1b1b18;--card:#292924;--text:#f5eee0;--sub:#a9a394;--line:#3d3b35;--accent:#cbaa7a}}
*{{box-sizing:border-box}}body{{width:{};margin:0;background:var(--bg);color:var(--text);font:13px system-ui,-apple-system,Segoe UI,sans-serif}}
form{{display:inline-flex;align-items:center;gap:7px}}input,select,textarea{{font:inherit;background:#302f29;color:var(--text);border:1px solid var(--line);border-radius:9px;padding:8px;min-width:0}}
textarea{{width:100%;min-height:92px;resize:vertical}}button{{background:var(--card);border:1px solid var(--line);color:var(--text);padding:8px 10px;border-radius:9px;cursor:pointer}}
button:hover,button:focus-visible{{border-color:var(--accent)}}button.primary{{background:var(--accent);color:#1b1b18;border-color:var(--accent);font-weight:700}}
nav{{display:flex;align-items:center;gap:6px;overflow-x:auto;white-space:nowrap}}
.tab{{max-width:185px;display:inline-block;overflow:hidden;text-overflow:ellipsis;vertical-align:middle}}
.tab.active{{color:var(--accent);border-color:var(--accent)}}
small,.muted{{color:var(--sub)}}section{{border-top:1px solid var(--line);padding:17px 14px}}
.stack{{display:flex;flex-direction:column;gap:10px}}.bubble{{padding:10px;border:1px solid var(--line);background:var(--card);border-radius:12px;white-space:pre-wrap;overflow-wrap:anywhere}}
label{{color:var(--sub);font-size:12px}}h2{{font-size:16px;margin:0 0 14px}}.error{{color:#e9aeae}}
</style></head><body>"#,escape(title),width)
}
fn footer() -> &'static str {"</body></html>"}
pub fn resolve_address(raw: &str) -> Option<String> {
    let value=raw.trim();
    if value.is_empty() || value.len()>2048 {return None}
    let maybe=if value.contains("://"){value.to_owned()}
        else if value.contains(' ')||!value.contains('.') {
            let mut url=url::Url::parse("https://www.google.com/search").ok()?;
            url.query_pairs_mut().append_pair("q",value);
            url.to_string()
        } else {format!("https://{value}")};
    crate::session::valid_page_url(&maybe).then_some(maybe)
}
fn chrome(ui:&Ui)->Html<String>{
    let mut html=header_html("Charlotte | Tabs",true);
    html.push_str(r#"<main style="display:flex;flex-direction:column;padding:7px 10px;gap:6px;height:100vh">
<div style="display:flex;gap:7px;align-items:center"><strong style="letter-spacing:.04em;color:var(--accent)">Charlotte</strong>"#);
    let tabs=ui.tabs.lock().ok();
    let active=tabs.as_ref().and_then(|t|t.active());
    let active_id=active.map(|t|t.id);
    let active_url=active.map(|t|t.url.as_str()).unwrap_or("");
    let tok=input("token",&ui.token);
    for (label,action) in [("←","back"),("→","forward"),("↻","reload"),("＋","new")] {
        html.push_str(&format!(r#"<form method="post" action="/action">{}<button name="command" value="{}" title="{}">{}</button></form>"#,tok,action,action,label));
    }
    html.push_str(&format!(r#"<form style="flex:1" method="post" action="/action">{tok}<input name="value" aria-label="Address or search" value="{}" placeholder="Enter address or search" style="flex:1"><button class="primary" name="command" value="navigate">Go</button></form>"#,escape(active_url)));
    let (enabled,blocked,rules)=ui.filter.read().map(|f|(f.is_enabled(),f.blocked_count(),f.rule_count()))
        .unwrap_or((true,0,0));
    let action=if enabled{"shields-off"}else{"shields-on"};
    let state=if enabled{"ON"}else{"OFF"};
    html.push_str(&format!(r#"<form method="post" action="/action">{tok}<button name="command" value="{action}" aria-label="Toggle ad blocker" title="Charlotte Shields: {state}, {rules} loaded filter rules, {blocked} blocked requests">🛡 {state} · {blocked}</button></form>"#));
    html.push_str(r#"<form method="get" action="/chrome"><button title="Refresh tab strip">⟳</button></form></div><nav>"#);
    if let Some(tabs)=tabs{
        for tab in tabs.all(){
            let a=if Some(tab.id)==active_id{" active"}else{""};
            let state=if tab.sleeping{"☾ "}else{""};
            html.push_str(&format!(r#"<form method="post" action="/action">{tok}<input type="hidden" name="id" value="{}"><button class="tab{}" title="{}" name="command" value="select">{}{}</button><button name="command" value="close" aria-label="Close tab">×</button></form>"#,tab.id,a,escape(&tab.url),state,escape(&tab.title)));
        }
    }
    html.push_str(r#"</nav></main>"#);
    html.push_str(footer());
    Html(html)
}
fn sidebar(ui:&Ui)->Html<String>{
    let mut html=header_html("Charlotte | Charlotte AI Agent",true);
    let active_url=ui.tabs.lock().ok().and_then(|tabs|tabs.active().map(|t|t.url.clone()));
    let active_origin=active_url.as_deref().and_then(permissions::canonical_origin);
    html.push_str(r#"<header style="padding:18px 14px"><strong style="font-size:17px;color:var(--accent)">Charlotte</strong><p class="muted">Charlotte AI Agent + MCP tools — native Rust</p></header>"#);
    let token=input("token",&ui.token);
    let Ok(inner)=ui.inner.lock() else {return Html("Sidebar unavailable".into())};
    html.push_str(r#"<section><h2>Ask Charlotte</h2><div class="stack"><form action="/ai" method="post" style="display:flex;flex-direction:column;align-items:stretch">"#);
    html.push_str(&token);
    html.push_str(r#"<label for="provider">Provider</label><select name="provider" id="provider"><option value="openai">OpenAI</option><option value="claude">Claude</option><option value="gemini">Gemini</option><option value="ollama">Ollama</option></select><label for="mode">Charlotte mode</label><select id="mode" name="mode"><option value="chat">Chat + MCP tools</option><option value="browser">Automate this Chromium tab</option><option value="visual">Visual automation (shares a screenshot)</option></select><textarea name="value" aria-label="Ask an AI" maxlength="4000" placeholder="Ask Charlotte a question or give Charlotte a task…"></textarea><button class="primary">Send</button></form>"#);
    let configured=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite"))
        .ok().and_then(|db|mcp::saved_servers(&db).ok()).unwrap_or_default();
    if configured.is_empty(){
        html.push_str("<p class=\"muted\">No MCP tools connected. Configure a server with charlotte-settings mcp add.</p>");
    } else {
        let names=configured.iter().map(|c|escape(&c.id)).collect::<Vec<_>>().join(", ");
        html.push_str(&format!("<p class=\"muted\">MCP servers configured: {names}. Charlotte asks before using any tool.</p>"));
    }
    if let Some(pending)=&inner.pending_browser {
        html.push_str("<div class=\"bubble\"><strong>Charlotte wants to interact with this page</strong>");
        html.push_str(&format!("<p class=\"muted\">Site: {} | Step {}/{}</p><p>{}</p><pre style=\"white-space:pre-wrap;overflow-wrap:anywhere\">{}</pre>",
            escape(&pending.context.origin),pending.step+1,browser_automation::MAX_STEPS,
            escape(&format!("{} — {}",pending.action.description(),pending.target_label)),escape(&serde_json::to_string(&pending.action).unwrap_or_default())));
        html.push_str(&format!(r#"<form action="/browser/approve" method="post">{token}<button name="command" class="primary" value="approve">Approve this browser step</button><button name="command" value="reject">Stop automation</button></form>"#));
        html.push_str("</div>");
    }
    html.push_str("<section><h2>JavaScript dialogs</h2><p class=\"muted\">If an alert, confirm or prompt is open in the current tab, you can respond explicitly. File pickers and OS permission dialogs stay user-controlled.</p>");
    html.push_str(&format!(r#"<form action="/browser/dialog" method="post">{token}<input type="text" name="prompt_text" maxlength="500" placeholder="Prompt reply (optional)"><button name="command" value="accept">Accept open dialog</button><button name="command" value="dismiss">Dismiss open dialog</button></form>"#));
    html.push_str("</section>");
    html.push_str("<p class=\"muted\">Browser mode shares page text with your selected AI provider. Visual mode additionally sends a JPEG screenshot of the visible Chromium viewport. Charlotte omits raw form values, password/file inputs and cookies, but visible page text can contain personal information. Every website-changing step requires approval.</p>");
    if let Some(pending)=&inner.pending_tool {
        html.push_str("<div class=\"bubble\"><strong>Charlotte requests approval to use an MCP tool</strong>");
        html.push_str(&format!("<p>Tool: <code>{}</code></p><pre style=\"white-space:pre-wrap;overflow-wrap:anywhere\">{}</pre>",
            escape(&pending.name),escape(&pending.args.to_string())));
        html.push_str("<small class=\"muted\">Tool output may be sent to your selected AI provider to explain the result.</small>");
        html.push_str(&format!(r#"<form action="/agent/approve" method="post">{token}<button class="primary" name="command" value="approve">Approve once</button><button name="command" value="reject">Deny</button></form>"#));
        html.push_str("</div>");
    }
    for turn in inner.ai.iter().rev().take(12).rev(){
        html.push_str(&format!(r#"<div class="bubble"><small>{}</small><div>{}</div></div>"#,escape(&turn.role),escape(&turn.text)));
    }
    html.push_str("</div></section>");
    html.push_str("<section><h2>Website notifications</h2>");
    if let Some(origin)=&active_origin {
        let permission=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite"))
            .ok().and_then(|db|permissions::get(&db,origin,Capability::Notifications).ok().flatten());
        let status=match permission{
            Some(Decision::Allow)=>"Allowed",
            Some(Decision::Deny)=>"Blocked",
            None=>"Not granted",
        };
        html.push_str(&format!("<p class=\"muted\">{} — {}</p>",escape(origin),status));
        html.push_str(&format!(r#"<form action="/notifications" method="post">{token}<button name="command" value="allow">Allow notifications</button><button name="command" value="deny">Block notifications</button><button name="command" value="reset">Reset</button></form>"#));
        html.push_str("<p class=\"muted\">Permission is managed locally. Background website push delivery depends on CEF's platform support.</p>");
    } else {html.push_str("<p class=\"muted\">Visit an HTTPS website to manage its notification permission.</p>");}
    html.push_str("</section><section><h2>Page chat</h2>");
    if inner.chat_token.is_empty(){
        html.push_str(&format!(r#"<form action="/connect" method="post" style="display:flex;flex-direction:column;align-items:stretch">{token}<label>Chat server</label><input name="server" value="{}" maxlength="300" placeholder="https://chat.example"><label>Username</label><input name="username" minlength="2" maxlength="32" required placeholder="Your name"><button class="primary">Connect</button></form>"#,escape(&inner.chat_base)));
    }else{
        html.push_str(&format!(r#"<div class="muted" style="padding-bottom:10px">Connected to {}</div>"#,escape(&inner.chat_base)));
        for (name,msg) in inner.chat.iter().rev().take(30).rev(){
            html.push_str(&format!(r#"<div class="bubble"><small>{}</small><div>{}</div></div>"#,escape(name),escape(msg)));
        }
        html.push_str(&format!(r#"<form style="display:flex;flex-direction:column;align-items:stretch;margin-top:12px" action="/send" method="post">{token}<textarea name="value" placeholder="Write a message" maxlength="2000" required></textarea><button class="primary">Send chat message</button></form>"#));
    }
    if inner.working { html.push_str(r#"<p class="muted">Working… refresh to see reply</p>"#); }
    if !inner.error.is_empty(){html.push_str(&format!(r#"<p class="error" role="alert">{}</p>"#,escape(&inner.error)));}
    html.push_str(r#"</section><footer style="padding:15px"><form action="/sidebar" method="get"><button>Refresh conversation</button></form></footer>"#);
    html.push_str(footer());Html(html)
}
fn check(ui:&Ui, headers:&HeaderMap, token:&str)->bool {
    let origin=format!("http://127.0.0.1:{}",ui.port);
    // Browsers set Origin on POST forms. Forbid cross-site request forgery
    // and do not accept requests with a missing Origin header.
    let valid_origin=headers.get(header::ORIGIN).and_then(|v|v.to_str().ok())==Some(origin.as_str());
    let valid_host=headers.get(header::HOST).and_then(|v|v.to_str().ok())==Some(&format!("127.0.0.1:{}",ui.port)[..]);
    let valid=token==ui.token;
    valid_origin && valid_host && valid
}
fn forbidden()->Response{(StatusCode::FORBIDDEN,"Invalid local origin or form token").into_response()}
fn reply(path:&'static str)->Response {Redirect::to(path).into_response()}
async fn action(State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let cmd=match form.command.as_deref().unwrap_or(""){
        "new"=>Some(Command::New("".into())),
        "select"=>form.id.map(Command::Select),
        "close"=>form.id.map(Command::Close),
        "navigate"=>form.value.as_deref().and_then(resolve_address).map(Command::Navigate),
        "back"=>Some(Command::Back),
        "forward"=>Some(Command::Forward),
        "reload"=>Some(Command::Reload),
        "hard-reload"=>Some(Command::HardReload),
        "shields-on"=>Some(Command::Shields(true)),
        "shields-off"=>Some(Command::Shields(false)),
        _=>None,
    };
    let Some(cmd)=cmd else{return (StatusCode::BAD_REQUEST,"Invalid command").into_response()};
    crate::app::post_ui_command(cmd);
    reply("/chrome")
}
fn busy(ui:&Ui)->bool{
    if let Ok(mut inner)=ui.inner.lock(){
        if inner.working {return true}
        inner.working=true;inner.error.clear();return false
    }
    true
}
fn finish(ui:&Ui,error:Option<String>){
    if let Ok(mut i)=ui.inner.lock(){i.working=false;i.error=error.unwrap_or_default()}
    crate::app::post_ui_command(Command::Sync);
}
fn provider_model(name:&str)->Option<(Provider,&'static str)>{
    match name {
        "openai"=>Some((Provider::OpenAi,"gpt-4o-mini")),
        "claude"=>Some((Provider::Claude,"claude-sonnet-4-5")),
        "gemini"=>Some((Provider::Gemini,"gemini-2.5-flash")),
        "ollama"=>Some((Provider::Ollama,"llama3.2")),
        _=>None,
    }
}
fn browser_answer(state:&Ui,goal:&str,answer:String){
    if let Ok(mut inner)=state.inner.lock(){
        inner.pending_browser=None;
        inner.ai.push(Turn{role:"user".into(),text:goal.into()});
        inner.ai.push(Turn{role:"assistant".into(),text:answer.clone()});
        if inner.ai.len()>100 {let excess=inner.ai.len()-100;inner.ai.drain(0..excess);}
    }
    if let Ok(db)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
        let time=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_millis() as i64;
        let _=db.add_message("ai","user",goal,time);
        let _=db.add_message("ai","assistant",&answer,time+1);
    }
    finish(state,None);
}
/// Fingerprint only: screenshots are never stored in persistent browser state.
fn screenshot_fingerprint(jpeg:&str)->u64{
    use std::hash::{Hash,Hasher};
    let mut hash=std::collections::hash_map::DefaultHasher::new();
    jpeg.hash(&mut hash);
    hash.finish()
}
fn plan_browser_next(
    state:&Ui,context:BrowserContext,goal:String,provider_name:String,
    step:u8,previous:&str,visual:bool,
)->Result<(),String>{
    let (provider,default_model)=provider_model(&provider_name).ok_or("Unknown AI provider")?;
    let model=if visual&&provider_name=="ollama"{"llava"}else{default_model};
    let mut snapshot=context.observe_settled()?;
    let screenshot=if visual {Some(context.screenshot()?)}else{None};
    if let Some((_,(width,height)))=&screenshot {
        snapshot["imagePixels"]=serde_json::json!({"width":width,"height":height});
    }
    if visual {
        if let Ok(frames)=context.frame_inventory(){
            snapshot["isolatedFrames"]=serde_json::json!(frames);
        }
    }
    let key=crate::credentials::retrieve(&format!("ai:{provider_name}")).ok();
    let history=state.inner.lock().ok().map(|inner|inner.ai.clone()).unwrap_or_default();
    match browser_agent::decide(provider,key.as_deref(),model,&history,
        &goal,step,&snapshot,previous,screenshot.as_ref().map(|(jpeg,_)|jpeg.as_str()))?{
        BrowserDecision::Answer(text)=>{
            browser_answer(state,&goal,text);
        },
        BrowserDecision::Action(BrowserAction::Observe)=>{
            // Observation is performed before every planning step; letting
            // the model repeatedly request it would create an infinite loop.
            browser_answer(state,&goal,
                "Charlotte has inspected this page but needs a more specific next action. You can refine your task.".into());
        },
        BrowserDecision::Action(action)=>{
            if matches!(action,BrowserAction::ClickAt{..})&&!visual {
                return Err("Screenshot-coordinate click requires explicit visual mode".into())
            }
            browser_automation::ensure_observed_target(&action,&snapshot)?;
            if let BrowserAction::Navigate{url}=&action {
                browser_automation::require_same_origin(&context.origin,url)?;
            }
            let target_label=match &action{
                BrowserAction::Click{selector}|BrowserAction::Type{selector,..}=>{
                    snapshot.get("controls").and_then(serde_json::Value::as_array)
                        .and_then(|controls|controls.iter().find(|c|
                            c.get("selector").and_then(serde_json::Value::as_str)==Some(selector.as_str())))
                        .and_then(|c|c.get("label").and_then(serde_json::Value::as_str))
                        .unwrap_or("").to_owned()
                },
                _=>String::new(),
            };
            let observed_url=snapshot.get("url").and_then(serde_json::Value::as_str)
                .ok_or("Chromium observation has no page URL")?.to_owned();
            if let Ok(mut inner)=state.inner.lock(){
                let screenshot_fingerprint=if matches!(action,BrowserAction::ClickAt{..}){
                    screenshot.as_ref().map(|(jpeg,_)|screenshot_fingerprint(jpeg))
                }else{None};
                inner.pending_browser=Some(PendingBrowser{
                    context,action,goal,provider:provider_name,step,observed_url,target_label,
                    visual,screenshot_fingerprint,
                });
            }else{return Err("Automation state lock failed".into())}
            finish(state,None);
        },
    }
    Ok(())
}
async fn browser_approve(
    State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>
)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let approve=match form.command.as_deref(){
        Some("approve")=>true,Some("reject")=>false,
        _=>return (StatusCode::BAD_REQUEST,"Invalid browser approval").into_response(),
    };
    let pending={
        let Ok(mut inner)=ui.inner.lock() else{
            return (StatusCode::CONFLICT,"Automation busy").into_response()
        };
        if inner.working{return (StatusCode::CONFLICT,"Charlotte is already working").into_response()}
        let Some(pending)=inner.pending_browser.take() else {
            return (StatusCode::BAD_REQUEST,"No browser action awaiting approval").into_response()
        };
        if approve {inner.working=true;}
        pending
    };
    if !approve {
        browser_answer(&ui,&pending.goal,
            "Charlotte stopped browser automation at your request. No further actions were executed.".into());
        return reply("/sidebar")
    }
    let state=ui.clone();
    tokio::task::spawn_blocking(move||{
        let mut attempted=false;
        let outcome=(||->Result<(),String>{
            // Check the actual live Chromium URL before executing a selector
            // generated from an earlier DOM snapshot.
            let current=pending.context.observe_settled()?;
            if current.get("url").and_then(serde_json::Value::as_str)!=Some(pending.observed_url.as_str()){
                return Err("Page changed since Charlotte proposed this action. Nothing was clicked or typed; start a new task.".into())
            }
            browser_automation::ensure_observed_target(&pending.action,&current)?;
            if matches!(pending.action,BrowserAction::ClickAt{..}){
                let (new_image,_)=pending.context.screenshot()?;
                if pending.screenshot_fingerprint!=Some(screenshot_fingerprint(&new_image)){
                    return Err("The visual page changed since Charlotte proposed the click; nothing was clicked. Re-plan this step.".into())
                }
            }
            attempted=true;
            let action_outcome=pending.context.act(&pending.action)?;
            if let BrowserAction::Navigate{url}= &pending.action {
                pending.context.wait_for_navigation(url)?;
            } else {
                std::thread::sleep(std::time::Duration::from_millis(350));
            }
            plan_browser_next(&state,pending.context,pending.goal,
                pending.provider,pending.step+1,&action_outcome.to_string(),pending.visual)
        })();
        if let Err(error)=outcome {
            let message=if attempted {
                format!("Charlotte attempted the approved browser action, but verification failed. It may have changed the page: {error}")
            }else{error};
            finish(&state,Some(message));
        }
    });
    reply("/sidebar")
}
/// User-directed handling for Chromium JS alert/confirm/prompt. Does not
/// bypass native OS file pickers, login, permission or payment dialogs.
async fn browser_dialog(
    State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>
)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let accept=match form.command.as_deref(){
        Some("accept")=>true,Some("dismiss")=>false,
        _=>return (StatusCode::BAD_REQUEST,"Invalid dialog action").into_response(),
    };
    let url=ui.tabs.lock().ok().and_then(|tabs|
        tabs.active().map(|t|(t.id,t.url.clone())));
    let Some((id,url))=url else {
        return (StatusCode::BAD_REQUEST,"No active Chromium tab").into_response()
    };
    let Ok(context)=BrowserContext::new(id,&url) else {
        return (StatusCode::BAD_REQUEST,"Open an HTTPS site first").into_response()
    };
    let prompt_text=form.prompt_text.unwrap_or_default();
    if prompt_text.len()>500{return (StatusCode::BAD_REQUEST,"Dialog response too long").into_response()}
    if busy(&ui){return (StatusCode::CONFLICT,"Charlotte is working").into_response()}
    let state=ui.clone();
    tokio::task::spawn_blocking(move||{
        let action=BrowserAction::Dialog{
            accept,prompt_text:if prompt_text.is_empty(){None}else{Some(prompt_text)}
        };
        let result=context.act(&action);
        match result{
            Ok(_)=>{
                if let Ok(mut inner)=state.inner.lock(){
                    inner.ai.push(Turn{role:"assistant".into(),
                        text:format!("Charlotte {} the active Chromium JavaScript dialog.",if accept{"accepted"}else{"dismissed"})});
                }
                finish(&state,None);
            },
            Err(error)=>finish(&state,Some(format!("Could not resolve the active JavaScript dialog: {error}"))),
        }
    });
    reply("/sidebar")
}
async fn ai(State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let Some(text)=form.value.map(|s|s.trim().to_string()).filter(|s|!s.is_empty()&&s.len()<=4000)
        else{return (StatusCode::BAD_REQUEST,"Message required").into_response()};
    let provider_name=form.provider.unwrap_or_else(||"openai".into());
    let Some((provider,model))=provider_model(&provider_name)
        else{return (StatusCode::BAD_REQUEST,"Unknown provider").into_response()};
    if ui.inner.lock().is_ok_and(|g|g.pending_tool.is_some()||g.pending_browser.is_some()){
        return (StatusCode::CONFLICT,"Resolve Charlotte's pending MCP or Chromium browser action first").into_response()
    }
    let mode=form.mode.as_deref().unwrap_or("chat");
    if !["chat","browser","visual"].contains(&mode){
        return (StatusCode::BAD_REQUEST,"Unknown Charlotte mode").into_response()
    }
    if mode=="browser" || mode=="visual" {
        let current=ui.tabs.lock().ok()
            .and_then(|tabs|tabs.active().map(|t|(t.id,t.url.clone())));
        let Some((id,url))=current else {
            return (StatusCode::BAD_REQUEST,"Open a website in an active tab first").into_response()
        };
        let Ok(context)=BrowserContext::new(id,&url) else {
            return (StatusCode::BAD_REQUEST,"Automation requires an active HTTPS or loopback webpage").into_response()
        };
        if busy(&ui){return (StatusCode::CONFLICT,"Charlotte is already working").into_response()}
        let visual=mode=="visual";
        let state=ui.clone();
        tokio::task::spawn_blocking(move||{
            if let Err(error)=plan_browser_next(&state,context,text,provider_name,0,"Started task",visual){
                finish(&state,Some(error));
            }
        });
        return reply("/sidebar")
    }
    if busy(&ui){return (StatusCode::CONFLICT,"Charlotte is already working").into_response()}
    let state=ui.clone();
    tokio::task::spawn_blocking(move||{
        let history=state.inner.lock().ok().map(|i|i.ai.clone()).unwrap_or_default();
        let key=crate::credentials::retrieve(&format!("ai:{provider_name}")).ok();
        let mut registry=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite"))
            .ok().and_then(|db|mcp::connect_saved(&db).ok())
            .unwrap_or_else(mcp::Registry::new);
        let tools=registry.tools();
        let response=agent::plan(provider,key.as_deref(),model,&history,&text,&tools);
        match response{
            Ok(Plan::Answer(reply))=>{
                if let Ok(mut inner)=state.inner.lock(){
                    inner.ai.push(Turn{role:"user".into(),text:text.clone()});
                    inner.ai.push(Turn{role:"assistant".into(),text:reply.clone()});
                    if inner.ai.len()>100 {let extra=inner.ai.len()-100;inner.ai.drain(0..extra);}
                }
                if let Ok(db)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
                    let time=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
                    let _=db.add_message("ai","user",&text,time);
                    let _=db.add_message("ai","assistant",&reply,time+1);
                }
                finish(&state,None);
            },
            Ok(Plan::Tool{name,args})=>{
                if let Some(tool)=tools.iter().find(|t|t.name==name){
                    if let Ok(mut inner)=state.inner.lock(){
                        inner.pending_tool=Some(PendingTool{
                            name,server_id:tool.server_id.clone(),args,
                            question:text,provider:provider_name,
                        });
                    }
                    finish(&state,None);
                } else {finish(&state,Some("MCP tool was not advertised".into()));}
            },
            Err(error)=>finish(&state,Some(error)),
        }
    });
    reply("/sidebar")
}
/// Tool execution is only reachable from the explicit signed local form.
async fn agent_approve(State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let approved=match form.command.as_deref().unwrap_or(""){
        "approve"=>true,"reject"=>false,
        _=>return (StatusCode::BAD_REQUEST,"Invalid approval action").into_response(),
    };
    let pending={
        let Ok(mut inner)=ui.inner.lock() else {return (StatusCode::CONFLICT,"Busy").into_response()};
        if inner.working {return (StatusCode::CONFLICT,"Charlotte is busy").into_response()}
        let Some(pending)=inner.pending_tool.take() else {
            return (StatusCode::BAD_REQUEST,"No pending MCP tool").into_response()
        };
        if approved {inner.working=true;}
        pending
    };
    if !approved {
        if let Ok(mut inner)=ui.inner.lock(){
            inner.ai.push(Turn{role:"assistant".into(),
                text:format!("Charlotte: Tool {} was denied; no action executed.",pending.name)});
        }
        return reply("/sidebar")
    }
    let state=ui.clone();
    tokio::task::spawn_blocking(move||{
        let result=(||->Result<String,String>{
            let db=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite"))
                .map_err(|e|e.to_string())?;
            let mut registry=mcp::connect_saved(&db)?;
            let verified=registry.tools().iter().any(|tool|
                tool.name==pending.name && tool.server_id==pending.server_id);
            if !verified{return Err("MCP server or tool changed after approval".into())}
            let output=agent::execute_approved(&mut registry,&pending.name,pending.args)?;
            let (provider,model)=provider_model(&pending.provider).ok_or("Invalid AI provider")?;
            let key=crate::credentials::retrieve(&format!("ai:{}",pending.provider)).ok();
            // An MCP action may have succeeded even if the model is offline.
            // Never report a completed external mutation as a failed tool.
            match agent::explain_result(provider,key.as_deref(),model,
                &pending.question,&pending.name,&output) {
                Ok(summary)=>Ok(summary),
                Err(error)=>{
                    let excerpt=output.to_string();
                    let excerpt:String=excerpt.chars().take(2000).collect();
                    Ok(format!("Charlotte executed approved MCP tool {}. AI summary unavailable: {}. Tool response: {}",
                        pending.name,error,excerpt))
                },
            }
        })();
        match result{
            Ok(reply)=>{
                if let Ok(mut inner)=state.inner.lock(){
                    inner.ai.push(Turn{role:"user".into(),text:pending.question.clone()});
                    inner.ai.push(Turn{role:"assistant".into(),text:reply.clone()});
                    if inner.ai.len()>100 {let extra=inner.ai.len()-100;inner.ai.drain(0..extra);}
                }
                if let Ok(db)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
                    let time=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
                    let _=db.add_message("ai","user",&pending.question,time);
                    let _=db.add_message("ai","assistant",&reply,time+1);
                }
                finish(&state,None);
            },
            Err(err)=>finish(&state,Some(err)),
        }
    });
    reply("/sidebar")
}
async fn notification_permission(State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let url=ui.tabs.lock().ok().and_then(|t|t.active().map(|tab|tab.url.clone()));
    let Some(origin)=url.as_deref().and_then(permissions::canonical_origin) else {
        return (StatusCode::BAD_REQUEST,"Active HTTPS website required").into_response()
    };
    let db=match crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
        Ok(db)=>db,Err(_)=>return (StatusCode::INTERNAL_SERVER_ERROR,"Storage error").into_response(),
    };
    let result=match form.command.as_deref().unwrap_or(""){
        "allow"=>permissions::set(&db,&origin,Capability::Notifications,Decision::Allow),
        "deny"=>permissions::set(&db,&origin,Capability::Notifications,Decision::Deny),
        "reset"=>permissions::reset(&db,&origin,Capability::Notifications),
        _=>return (StatusCode::BAD_REQUEST,"Unknown notification setting").into_response(),
    };
    if result.is_err(){return (StatusCode::BAD_REQUEST,"Cannot update site permissions").into_response()}
    reply("/sidebar")
}
async fn connect(State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let base=form.server.unwrap_or_default();
    let username=form.username.unwrap_or_default();
    if username.is_empty() || username.len()>32 || base.len()>300{
        return (StatusCode::BAD_REQUEST,"Invalid chat identity").into_response()
    }
    if busy(&ui){return (StatusCode::CONFLICT,"A request is already running").into_response()}
    let state=ui.clone();
    tokio::task::spawn_blocking(move||{
        let room=state.tabs.lock().ok().and_then(|t|t.active().map(|a|a.url.clone())).unwrap_or_default();
        let identity=ChatIdentity{url:&room,browser:"CharlotteNative",session_id:"native-rust",region:"local",username:Some(&username),token:None};
        match services::chat_connect(&base,&identity){
            Ok(session)=>{
                if let Ok(mut inner)=state.inner.lock(){
                    inner.chat_base=base.clone();inner.chat_token=session.token.clone();inner.chat_cursor=session.cursor;inner.room_url=room.clone();
                    inner.stop.store(true,Ordering::Relaxed);
                    inner.stop=Arc::new(AtomicBool::new(false));
                }
                finish(&state,None);
                // SSE lives on a worker thread and never touches a CEF view.
                let listener=state.clone();
                thread::spawn(move||{
                    let stop=listener.inner.lock().ok().map(|g|g.stop.clone()).unwrap_or_default();
                    let mut cursor=session.cursor;
                    while !stop.load(Ordering::Relaxed){
                        let result=services::chat_events(&base,&session.token,cursor,&stop,|msg|{
                            if let Ok(mut inner)=listener.inner.lock(){
                                inner.chat.push((msg.sender.clone(),msg.content.clone()));
                                inner.chat_cursor=msg.seq;
                                if let Ok(db)=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite")){
                                    let _=db.add_message(&msg.room,&msg.sender,&msg.content,msg.timestamp);
                                }
                                if inner.chat.len()>200 {let excess=inner.chat.len()-200;inner.chat.drain(0..excess);}
                            }
                            crate::app::post_ui_command(Command::Sync);
                        });
                        match result {Ok(next)=>cursor=next,Err(e)=>{
                            if let Ok(mut inner)=listener.inner.lock(){inner.error=e.to_string();}
                        }}
                        thread::sleep(std::time::Duration::from_secs(2));
                    }
                });
            },
            Err(error)=>finish(&state,Some(error.to_string())),
        }
    });
    reply("/sidebar")
}
async fn send(State(ui):State<Ui>,headers:HeaderMap,Form(form):Form<ActionForm>)->Response{
    if !check(&ui,&headers,&form.token){return forbidden()}
    let text=form.value.unwrap_or_default();
    if text.is_empty() || text.len()>2000 {return (StatusCode::BAD_REQUEST,"Invalid chat message").into_response()}
    let (base,token)=match ui.inner.lock(){
        Ok(inner) if !inner.chat_token.is_empty()=>(inner.chat_base.clone(),inner.chat_token.clone()),
        _=>return (StatusCode::UNAUTHORIZED,"Not connected").into_response(),
    };
    if busy(&ui){return (StatusCode::CONFLICT,"A request is already running").into_response()}
    let state=ui.clone();
    tokio::task::spawn_blocking(move||{
        let err=services::chat_send(&base,&token,&text).err().map(|e|e.to_string());
        finish(&state,err);
    });
    reply("/sidebar")
}
fn response_headers(mut response:Response)->Response{
    let h=response.headers_mut();
    h.insert(header::CACHE_CONTROL,"no-store".parse().unwrap());
    h.insert(header::X_CONTENT_TYPE_OPTIONS,"nosniff".parse().unwrap());
    h.insert(header::REFERRER_POLICY,"no-referrer".parse().unwrap());
    h.insert("x-frame-options","DENY".parse().unwrap());
    response
}
// Minimal loopback diagnostics without URLs, chat content or secrets.
async fn health(State(ui):State<Ui>)->Json<serde_json::Value>{
    let (enabled,rules,blocked)=ui.filter.read()
        .map(|shield|(shield.is_enabled(),shield.rule_count(),shield.blocked_count()))
        .unwrap_or((false,0,0));
    Json(serde_json::json!({
        "cef_main_frame_loads":crate::cef_bridge::main_frame_loads(),
        "shields_enabled":enabled,
        "filter_rules":rules,
        "blocked_requests":blocked,
    }))
}
pub fn start(tabs:TabState,filter:Filter)->Result<Ui,String>{
    use rand::{rngs::OsRng,RngCore};
    let mut nonce=[0u8;32];OsRng.fill_bytes(&mut nonce);
    let token: String=nonce.iter().map(|byte|format!("{byte:02x}")).collect();
    let listener=TcpListener::bind("127.0.0.1:0").map_err(|e|e.to_string())?;
    let port=listener.local_addr().map_err(|e|e.to_string())?.port();
    listener.set_nonblocking(true).map_err(|e|e.to_string())?;
    let history=crate::storage::Storage::open(&crate::profile::profile_root().join("charlotte.sqlite"))
        .ok().and_then(|db|db.messages("ai",100).ok())
        .unwrap_or_default().into_iter().map(|m|Turn{role:m.sender,text:m.body}).collect();
    let ui=Ui{
        port,token,tabs,filter,inner:Arc::new(Mutex::new(UiInner{
            pending_browser:None,pending_tool:None,ai:history,chat:Vec::new(),chat_base:std::env::var("CHARLOTTE_CHAT_SERVER").unwrap_or_default(),
            chat_token:String::new(),chat_cursor:0,room_url:String::new(),error:String::new(),working:false,
            stop:Arc::new(AtomicBool::new(false)),
        }))
    };
    let state=ui.clone();
    thread::Builder::new().name("charlotte-native-ui".into()).spawn(move||{
        // The loopback chrome API is I/O-bound; one event-loop thread is
        // sufficient. Blocking AI/network work uses Tokio's on-demand pool.
        let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio UI runtime");
        runtime.block_on(async move{
            let router=Router::new()
                .route("/chrome",get(|State(ui):State<Ui>|async move{chrome(&ui)}))
                .route("/health",get(health))
                .route("/sidebar",get(|State(ui):State<Ui>|async move{sidebar(&ui)}))
                .route("/action",post(action))
                .route("/ai",post(ai))
                .route("/agent/approve",post(agent_approve))
                .route("/browser/approve",post(browser_approve))
                .route("/browser/dialog",post(browser_dialog))
                .route("/notifications",post(notification_permission))
                .route("/connect",post(connect))
                .route("/send",post(send))
                .with_state(state);
            let socket=tokio::net::TcpListener::from_std(listener).expect("native UI listener");
            if let Err(e)=axum::serve(socket,router.layer(axum::middleware::from_fn(|req: axum::extract::Request, next: axum::middleware::Next| async move {response_headers(next.run(req).await)}))).await{
                eprintln!("Native UI stopped: {e}");
            }
        });
    }).map_err(|e|e.to_string())?;
    if let Some(path)=std::env::var_os("CHARLOTTE_UI_PORT_FILE") {
        // Used only by the native desktop smoke harness; contains no secrets.
        if let Err(e)=std::fs::write(&path,ui.port.to_string()){
            eprintln!("Unable to publish native test port: {e}");
        }
    }
    Ok(ui)
}
impl Ui{
    pub fn page_changed(&self,url:&str){
        if let Ok(mut inner)=self.inner.lock() {
            if !inner.chat_token.is_empty() && inner.room_url != url {
                inner.stop.store(true, Ordering::Relaxed);
                inner.chat_token.clear();
                inner.chat.clear();
                inner.chat_cursor=0;
                inner.room_url.clear();
                inner.error="Page changed — reconnect to join this page's chat".into();
            }
        }
    }
    pub fn chrome_url(&self)->String{format!("http://127.0.0.1:{}/chrome",self.port)}
    pub fn sidebar_url(&self)->String{format!("http://127.0.0.1:{}/sidebar",self.port)}
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn escapes_untrusted_text(){
        assert_eq!(escape("<script>\"&"),"&lt;script&gt;&quot;&amp;");
    }
    #[test]
    fn csrf_denies_cross_origin_and_missing_tokens() {
        let tabs=Arc::new(Mutex::new(crate::tabs::Tabs::default()));
        let ui=start(tabs,Arc::new(std::sync::RwLock::new(crate::shield::Shield::new(&[crate::shield::STARTER_RULES],true)))).unwrap();
        let mut headers=HeaderMap::new();
        headers.insert(header::ORIGIN,format!("http://127.0.0.1:{}",ui.port).parse().unwrap());
        headers.insert(header::HOST,format!("127.0.0.1:{}",ui.port).parse().unwrap());
        assert!(check(&ui,&headers,&ui.token));
        assert!(!check(&ui,&headers,"wrong"));
        headers.insert(header::ORIGIN,"https://evil.example".parse().unwrap());
        assert!(!check(&ui,&headers,&ui.token));
        headers.remove(header::ORIGIN);
        assert!(!check(&ui,&headers,&ui.token));
    }
    #[test]
    fn html_escapes_tab_titles_and_uses_html_forms() {
        let tabs=Arc::new(Mutex::new(crate::tabs::Tabs::default()));
        {
            let mut t=tabs.lock().unwrap();
            t.open(42);
            t.title(42,"<img src=x onerror=alert(1)>");
            t.navigate(42,"https://example.org");
        }
        let ui=start(tabs,Arc::new(std::sync::RwLock::new(crate::shield::Shield::new(&[crate::shield::STARTER_RULES],true)))).unwrap();
        let page=chrome(&ui).0;
        assert!(page.contains("&lt;img"));
        assert!(!page.contains("<img src=x"));
        assert!(page.contains("method=\"post\""));
        assert!(page.contains("https://example.org"));
    }
    #[test]
    fn resets_page_chat_credentials_on_navigation() {
        let ui=start(Arc::new(Mutex::new(crate::tabs::Tabs::default())),Arc::new(std::sync::RwLock::new(crate::shield::Shield::new(&[crate::shield::STARTER_RULES],true)))).unwrap();
        {
            let mut inner=ui.inner.lock().unwrap();
            inner.chat_token="secret".into();
            inner.room_url="https://first.example".into();
        }
        ui.page_changed("https://other.example");
        let inner=ui.inner.lock().unwrap();
        assert!(inner.chat_token.is_empty());
        assert_eq!(inner.chat_cursor,0);
    }
    #[test]
    fn charlotte_sidebar_requires_user_approval_for_mcp() {
        let tabs=Arc::new(Mutex::new(crate::tabs::Tabs::default()));
        let filter=Arc::new(std::sync::RwLock::new(
            crate::shield::Shield::new(&[crate::shield::STARTER_RULES],true)));
        let ui=start(tabs,filter).unwrap();
        {
            let mut inner=ui.inner.lock().unwrap();
            inner.pending_tool=Some(PendingTool{
                name:"mcp__s0__read".into(),server_id:"docs".into(),
                args:serde_json::json!({"path":"/test"}),question:"Read file".into(),
                provider:"ollama".into(),
            });
        }
        let page=sidebar(&ui).0;
        assert!(page.contains("Charlotte requests approval"));
        assert!(page.contains("Approve once"));
        assert!(page.contains(r#"name="command" value="reject""#));
        assert!(!page.contains("<script>"));
    }
    #[test]
    fn browser_automation_requires_explicit_action_approval(){
        let tabs=Arc::new(Mutex::new(crate::tabs::Tabs::default()));
        let shield=Arc::new(std::sync::RwLock::new(
            crate::shield::Shield::new(&[crate::shield::STARTER_RULES],true)));
        let ui=start(tabs,shield).unwrap();
        let context=BrowserContext::new(41,"https://example.org/page").unwrap();
        {
            let mut inner=ui.inner.lock().unwrap();
            inner.pending_browser=Some(PendingBrowser{
                context,action:BrowserAction::Click{
                    selector:"html > body > button".into()},
                goal:"Open settings".into(),provider:"ollama".into(),
                step:0,observed_url:"https://example.org/page".into(),
                target_label:"Settings".into(),visual:false,screenshot_fingerprint:None
            });
        }
        let html=sidebar(&ui).0;
        assert!(html.contains("Automate this Chromium tab"));
        assert!(html.contains("Approve this browser step"));
        assert!(html.contains("Stop automation"));
        assert!(html.contains("Settings"));
    }
    #[test]fn address_search_sanitizes_scheme(){
        assert!(resolve_address("file:///etc/passwd").is_none());
        assert!(resolve_address("javascript:alert(1)").is_none());
        assert_eq!(resolve_address("example.org"),Some("https://example.org".into()));
        assert!(resolve_address("design tools").unwrap().contains("google.com/search"));
    }
}
