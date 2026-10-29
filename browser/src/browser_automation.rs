//! Charlotte's typed Chromium automation protocol.
//! Only fixed, reviewed scripts may be sent to the active CEF page via CDP.
//! Remote websites can never access the host's automation IPC or native OS.
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};

pub const MAX_STEPS:u8=8;
pub const MAX_ELEMENTS:usize=70;
pub const MAX_SELECTOR:usize=512;
pub const MAX_TEXT:usize=2000;

#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(tag="action",rename_all="snake_case",deny_unknown_fields)]
pub enum Action {
    Observe,
    Navigate {url:String},
    Click {selector:String},
    Type {selector:String,text:String},
    Scroll {direction:ScrollDirection},
    ClickAt {x:u32,y:u32},
    Drag {from:String,to:String},
    Dialog {accept:bool,#[serde(default)] prompt_text:Option<String>},
}
#[derive(Debug,Clone,Copy,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ScrollDirection {Up,Down}
impl Action {
    pub fn validate(&self)->Result<(),String>{
        match self {
            Self::Observe|Self::Scroll{..}=>Ok(()),
            Self::ClickAt{x,y} if *x<=10000 && *y<=10000=>Ok(()),
            Self::ClickAt{..}=>Err("Visual click outside allowed viewport".into()),
            Self::Drag{from,to} if from.is_empty()||to.is_empty()||
                from.len()>MAX_SELECTOR||to.len()>MAX_SELECTOR=>Err("Invalid drag selectors".into()),
            Self::Drag{..}=>Ok(()),
            Self::Dialog{prompt_text,..} if prompt_text.as_ref().is_some_and(|v|v.len()>500)=>
                Err("Dialog text too long".into()),
            Self::Dialog{..}=>Ok(()),
            Self::Navigate{url} if crate::session::valid_page_url(url)=>Ok(()),
            Self::Navigate{..}=>Err("Only normal HTTP/HTTPS URLs may be opened".into()),
            Self::Click{selector}|Self::Type{selector,..} if selector.is_empty()||
                selector.len()>MAX_SELECTOR||selector.contains('\0') =>
                Err("Invalid element selector".into()),
            Self::Type{text,..} if text.len()>MAX_TEXT || text.contains('\0')=>
                Err("Text exceeds automation input limit".into()),
            _=>Ok(()),
        }
    }
    pub fn needs_approval(&self)->bool{
        !matches!(self,Self::Observe)
    }
    pub fn description(&self)->String{
        match self {
            Self::Observe=>"Inspect current page".into(),
            Self::Navigate{url}=>format!("Navigate Chromium to {url}"),
            Self::Click{selector}=>format!("Click page element {selector}"),
            Self::Type{selector,text}=>format!("Type {} characters into {selector}",text.chars().count()),
            Self::Scroll{direction}=>format!("Scroll page {:?}",direction),
            Self::ClickAt{x,y}=>format!("Click screenshot position ({x},{y})"),
            Self::Drag{from,to}=>format!("Drag {from} onto {to}"),
            Self::Dialog{accept,..}=>format!("{} JavaScript dialog",if *accept{"Accept"}else{"Dismiss"}),
        }
    }
}
/// Never let a model invent an arbitrary CSS selector to click, or repurpose
/// a text-only element as a form field. Its action must match the last real
/// Chromium DOM observation. Called again immediately before approval.
pub fn ensure_observed_target(action:&Action,snapshot:&Value)->Result<(),String>{
    let (selector,needs_editable)=match action {
        Action::Click{selector}=>(selector.as_str(),false),
        Action::Type{selector,..}=>(selector.as_str(),true),
        _=>return Ok(()),
    };
    let controls=snapshot.get("controls").and_then(Value::as_array)
        .ok_or("Missing page controls in Chromium observation")?;
    let matching=controls.iter().filter(|control|
        control.get("selector").and_then(Value::as_str)==Some(selector) &&
        control.get("password").and_then(Value::as_bool)!=Some(true) &&
        (!needs_editable||control.get("editable").and_then(Value::as_bool)==Some(true))
    ).count();
    if matching!=1 {
        return Err("Charlotte action does not match a unique, permitted visible page control".into())
    }
    if let Action::Click{..}=action {
        if let Some(control)=controls.iter().find(|item|
            item.get("selector").and_then(Value::as_str)==Some(selector)) {
            if let Some(href)=control.get("href").and_then(Value::as_str) {
                if !href.is_empty() {
                    let current=snapshot.get("url").and_then(Value::as_str).ok_or("Missing page URL")?;
                    require_same_origin(current,href)?;
                }
            }
        }
    }
    Ok(())
}
pub fn require_same_origin(current:&str,target:&str)->Result<(),String>{
    let source=crate::permissions::canonical_origin(current)
        .ok_or("Browser automation requires an active HTTPS/loopback page")?;
    let target=crate::permissions::canonical_origin(target)
        .ok_or("Automation target must be HTTPS or loopback HTTP")?;
    if source!=target {return Err("Cross-origin navigation needs a new user-authorized task".into())}
    Ok(())
}

/// Commands are bound to a specific CEF browser and verified on the CEF UI
/// thread. This module never accepts arbitrary Runtime.evaluate expressions.
pub fn runtime_evaluate(expression:String)->(String,Value){
    ("Runtime.evaluate".to_owned(),json!({
        "expression":expression,"returnByValue":true,"awaitPromise":false,
        "userGesture":false
    }))
}
fn serialized(value:&str)->Result<String,String>{
    serde_json::to_string(value).map_err(|e|e.to_string())
}
pub fn observation_command()->(String,Value){
    let script=String::from(r#"(() => {
        const selectable='a[href],button,input:not([type=hidden]),textarea,select,[role=button],[contenteditable=true]';
        const short=s=>String(s||'').replace(/\s+/g,' ').trim().slice(0,180);
        const path=el=>{
            const parts=[];
            for(let node=el;node&&node.nodeType===1&&parts.length<20;node=node.parentElement){
                const tag=node.localName;
                if(!tag)break;
                let ordinal=1;
                for(let sibling=node.previousElementSibling;sibling;sibling=sibling.previousElementSibling)
                    if(sibling.localName===tag)ordinal++;
                parts.unshift(tag+':nth-of-type('+ordinal+')');
                if(tag==='html')break;
            }
            return parts.join(' > ');
        };
        const controls=Array.from(document.querySelectorAll(selectable)).slice(0,300)
            .filter(el=>{
                if(el.matches('input[type=password],input[type=hidden],input[type=file]'))return false;
                const r=el.getBoundingClientRect();
                return r.width>0&&r.height>0&&!el.disabled;
            }).slice(0,70).map(el=>({
                selector:path(el),tag:el.localName,
                role:short(el.getAttribute('role')),
                label:short(el.getAttribute('aria-label')||el.labels?.[0]?.innerText||el.innerText||el.placeholder||el.name),
                type:short(el.getAttribute('type')),href:el.localName==='a'?short(new URL(el.href).origin+new URL(el.href).pathname):'',
                editable:el.matches('input,textarea,[contenteditable=true]'),
                password:el.matches('input[type=password]')
            }));
        return {url:location.origin+location.pathname,title:document.title,ready:document.readyState,
            text:short(document.body?.innerText).slice(0,180),
            excerpt:String(document.body?.innerText||'').slice(0,5000),
            controls};
    })()"#);
    runtime_evaluate(script)
}
pub fn click_point_command(selector:&str)->Result<(String,Value),String>{
    Action::Click{selector:selector.to_owned()}.validate()?;
    let quoted=serialized(selector)?;
    let expr=format!(r#"(() => {{
      const el=document.querySelector({quoted});
      if(!el) return {{error:"Element not found"}};
      if(document.querySelectorAll({quoted}).length!==1)return {{error:"Ambiguous selector"}};
      if(el.matches('input[type=password]')) return {{error:"Password control not actionable"}};
      el.scrollIntoView({{block:"center",inline:"center"}});
      const r=el.getBoundingClientRect();
      if(r.width<1||r.height<1) return {{error:"Element hidden"}};
      const x=Math.round(r.left+r.width/2),y=Math.round(r.top+r.height/2);
      const visible=document.elementFromPoint(x,y);
      if(!visible||(!el.contains(visible)&&!visible.contains(el)))
        return {{error:"Element is covered or not actually clickable"}};
      return {{x,y,tag:el.localName,
        text:String(el.innerText||el.getAttribute("aria-label")||"").slice(0,120)}};
    }})()"#);
    Ok(runtime_evaluate(expr))
}
pub fn type_command(selector:&str,text:&str)->Result<(String,Value),String>{
    Action::Type{selector:selector.into(),text:text.into()}.validate()?;
    let selector=serialized(selector)?;
    let value=serialized(text)?;
    // Native Chromium DOM value setter + input/change events for common SPA
    // frameworks; no arbitrary JS from the model can be injected.
    let expr=format!(r#"(() => {{
        const el=document.querySelector({selector});
        if(!el||document.querySelectorAll({selector}).length!==1||!el.matches("input,textarea,[contenteditable=true]"))
            return {{error:"Editable field not found"}};
        if(el.matches("input[type=password],input[type=file]"))
            return {{error:"Sensitive input requires manual user interaction"}};
        if(el.matches("[contenteditable=true]")) {{
            el.focus();el.textContent={value};
        }} else {{
            const proto=el.localName==="textarea"?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
            const setter=Object.getOwnPropertyDescriptor(proto,"value")?.set;
            if(!setter)return {{error:"Value setter unavailable"}};
            setter.call(el,{value});el.focus();
        }}
        el.dispatchEvent(new Event("input",{{bubbles:true}}));
        el.dispatchEvent(new Event("change",{{bubbles:true}}));
        return {{ok:true}};
    }})()"#);
    Ok(runtime_evaluate(expr))
}
pub fn scroll_command(direction:ScrollDirection)->(String,Value){
    let amount=match direction{ScrollDirection::Down=>610,ScrollDirection::Up=>-610};
    runtime_evaluate(format!("(() => {{ window.scrollBy({{top:{amount},behavior:'auto'}});return {{ok:true}}; }})()"))
}
pub fn input_mouse_command(kind:&str,x:i32,y:i32)->(String,Value){
    ("Input.dispatchMouseEvent".into(),json!({
        "type":kind,"x":x,"y":y,
        // Chrome requires the released button to match the pressed one.
        "button":if kind=="mouseMoved" {"none"} else {"left"},
        "clickCount":if kind=="mouseMoved" {0}else{1}
    }))
}
pub fn extract_result(raw:&Value)->Result<Value,String>{
    if let Some(e)=raw.get("exceptionDetails"){return Err(format!("Chromium JavaScript exception: {e}"))}
    let result=raw.get("result").ok_or("CDP Runtime.evaluate returned no result")?;
    let value=result.get("value").cloned().ok_or("Chromium returned no serializable value")?;
    if let Some(message)=value.get("error").and_then(Value::as_str){return Err(message.to_owned())}
    Ok(value)
}
pub fn click_coordinates(value:&Value)->Result<(i32,i32),String>{
    let x=value.get("x").and_then(Value::as_i64).ok_or("Element has no x position")?;
    let y=value.get("y").and_then(Value::as_i64).ok_or("Element has no y position")?;
    if !(0..=20000).contains(&x)||!(0..=20000).contains(&y) {
        return Err("Element coordinates out of browser bounds".into())
    }
    Ok((x as i32,y as i32))
}
/// Browser-specific capability: the browser ID and exact origin are fixed
/// when the user starts the task. Never use a global remote-debugging port.
#[derive(Debug,Clone)]
pub struct Context {pub browser_id:i32,pub origin:String}
impl Context {
    pub fn new(browser_id:i32,url:&str)->Result<Self,String>{
        if browser_id<=0{return Err("No active browser".into())}
        let origin=crate::permissions::canonical_origin(url)
            .ok_or("Only active HTTPS/loopback pages can be automated")?;
        Ok(Self{browser_id,origin})
    }
    pub fn cdp(&self,method:&str,params:Value)->Result<Value,String>{
        use std::sync::mpsc;
        use std::time::Duration;
        // Every request has an ID known to the worker, allowing cancellation
        // after timeout without leaking a native CEF observer registration.
        static NEXT_ID:std::sync::atomic::AtomicI32=
            std::sync::atomic::AtomicI32::new(100_000_000);
        let request_id=NEXT_ID.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
        if request_id>1_900_000_000{return Err("CDP message IDs exhausted".into())}
        let (reply,receive)=mpsc::channel();
        crate::app::post_ui_command(crate::ui_server::Command::Automation{
            request_id,browser_id:self.browser_id,expected_origin:self.origin.clone(),
            method:method.to_owned(),params,reply,
        });
        match receive.recv_timeout(Duration::from_secs(12)){
            Ok(answer)=>answer,
            Err(_)=>{
                crate::app::post_ui_command(
                    crate::ui_server::Command::AutomationFinished(request_id));
                Err("Chromium automation timed out (12 seconds)".into())
            },
        }
    }
    pub fn observe(&self)->Result<Value,String>{
        let (method,params)=observation_command();
        let result=self.cdp(&method,params)?;
        let data=extract_result(&result)?;
        let url=data.get("url").and_then(Value::as_str).ok_or("No page URL")?;
        if crate::permissions::canonical_origin(url).as_deref()!=Some(self.origin.as_str()){
            return Err("Page left the authorized origin".into())
        }
        Ok(data)
    }
    /// Page transitions are asynchronous; wait for an interactive document
    /// instead of assuming a fixed post-click delay is sufficient.
    pub fn observe_settled(&self)->Result<Value,String>{
        let mut last=String::from("Page did not become ready");
        for _ in 0..8 {
            match self.observe(){
                Ok(snapshot)=>{
                    if snapshot.get("ready").and_then(Value::as_str)!=Some("loading"){
                        return Ok(snapshot)
                    }
                    last="Chromium document is still loading".into();
                },
                Err(error)=>last=error,
            }
            std::thread::sleep(std::time::Duration::from_millis(350));
        }
        Err(last)
    }
    pub fn wait_for_navigation(&self,target:&str)->Result<(),String>{
        let parsed=url::Url::parse(target).map_err(|e|e.to_string())?;
        let expected=format!("{}{}",parsed.origin().ascii_serialization(),parsed.path());
        for _ in 0..12{
            if let Ok(snapshot)=self.observe(){
                if snapshot.get("url").and_then(Value::as_str)==Some(expected.as_str()) &&
                   snapshot.get("ready").and_then(Value::as_str)!=Some("loading"){
                    return Ok(())
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(350));
        }
        Err("Navigation did not complete on the approved origin".into())
    }
    pub fn act(&self,action:&Action)->Result<Value,String>{
        action.validate()?;
        match action {
            Action::Observe=>self.observe(),
            Action::Navigate{url}=>{
                require_same_origin(&self.origin,url)?;
                let result=self.cdp("Page.navigate",json!({"url":url}))?;
                if result.get("errorText").is_some(){return Err(format!("Navigation failed: {result}"))}
                Ok(result)
            },
            Action::Type{selector,text}=>{
                let (method,params)=type_command(selector,text)?;
                extract_result(&self.cdp(&method,params)?)
            },
            Action::Scroll{direction}=>{
                let (method,params)=scroll_command(*direction);
                extract_result(&self.cdp(&method,params)?)
            },
            Action::Click{selector}=>{
                let (method,params)=click_point_command(selector)?;
                let position=extract_result(&self.cdp(&method,params)?)?;
                let (x,y)=click_coordinates(&position)?;
                for kind in ["mouseMoved","mousePressed","mouseReleased"] {
                    let (method,params)=input_mouse_command(kind,x,y);
                    self.cdp(&method,params)?;
                }
                Ok(json!({"clicked":true,"selector":selector}))
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn rejects_invalid_navigation_and_inputs(){
        assert!(Action::Navigate{url:"javascript:alert(1)".into()}.validate().is_err());
        assert!(Action::Type{selector:"input".into(),text:"a".repeat(MAX_TEXT+1)}.validate().is_err());
        assert!(Action::Click{selector:"button".into()}.needs_approval());
        assert!(!Action::Observe.needs_approval());
    }
    #[test]fn origin_scope_rejects_unapproved_new_sites(){
        assert!(require_same_origin("https://site.example/path","https://site.example/next").is_ok());
        assert!(require_same_origin("https://site.example","https://other.example").is_err());
    }
    #[test]fn scripts_encode_untrusted_values_as_json_literals(){
        let (method,params)=type_command("input[title='\";evil()']","hello \"world\"").unwrap();
        assert_eq!(method,"Runtime.evaluate");
        let expr=params["expression"].as_str().unwrap();
        assert!(expr.contains(r#"hello \"world\""#));
        assert!(!expr.contains("eval("));
    }
    #[test]fn disallows_invented_or_noneditable_selectors(){
        let snap=json!({"controls":[
            {"selector":"html > body > button","editable":false,"password":false},
            {"selector":"html > body > input","editable":true,"password":false}
        ]});
        assert!(ensure_observed_target(&Action::Click{
            selector:"html > body > button".into()},&snap).is_ok());
        assert!(ensure_observed_target(&Action::Click{
            selector:"#invented".into()},&snap).is_err());
        let external=json!({"url":"https://site.example/","controls":[
            {"selector":"html > body > a","editable":false,"href":"https://other.example/"}]});
        assert!(ensure_observed_target(&Action::Click{
            selector:"html > body > a".into()},&external).is_err());
        assert!(ensure_observed_target(&Action::Type{
            selector:"html > body > button".into(),text:"x".into()},&snap).is_err());
    }
    #[test]fn mouse_down_and_up_both_keep_left_button(){
        for event in ["mousePressed","mouseReleased"] {
            let (method,args)=input_mouse_command(event,21,34);
            assert_eq!(method,"Input.dispatchMouseEvent");
            assert_eq!(args["button"],"left");
            assert_eq!(args["clickCount"],1);
        }
    }
    #[test]fn browser_snapshot_does_not_read_password_values(){
        let (_,args)=observation_command();
        let source=args["expression"].as_str().unwrap();
        assert!(source.contains("input[type=password]"));
        assert!(!source.contains("||el.value"));
    }
    #[test]fn validates_chromium_reply(){
        assert_eq!(extract_result(&json!({"result":{"value":{"ok":true}}})).unwrap(),json!({"ok":true}));
        assert!(extract_result(&json!({"exceptionDetails":{"text":"bad"}})).is_err());
    }
}
