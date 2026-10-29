//! Bounded inspection of open shadow roots and same-origin frames.
//! Cross-origin/OOPIF DOM is never bypassed: these frames are only listed,
//! and can be visually inspected via explicitly permitted screenshots.
use serde_json::{Value,json};

pub const LOOKUP:&str=r#"
 const resolvePart=(root,route)=>{
   const stages=route.split(' >>> ');
   let node=null,scope=root;
   for(let i=0;i<stages.length;i++){
     const matches=scope.querySelectorAll(stages[i]);
     if(matches.length!==1)return null;
     node=matches[0];
     if(i+1<stages.length){
       if(!node.shadowRoot)return null;
       scope=node.shadowRoot;
     }
   }
   return node;
 };
 const resolve=(selector)=>{
   const frames=selector.split(' ::frame ');
   let scope=document,offsetX=0,offsetY=0,frame=null,firstFrame=null;
   for(let i=0;i<frames.length-1;i++){
     frame=resolvePart(scope,frames[i]);
     if(!frame||frame.localName!=='iframe')return null;
     if(!firstFrame)firstFrame=frame;
     try{scope=frame.contentDocument;}catch(e){return null;}
     if(!scope)return null;
     const r=frame.getBoundingClientRect();
     offsetX+=r.left;offsetY+=r.top;
   }
   const el=resolvePart(scope,frames[frames.length-1]);
   return el?{el,scope,offsetX,offsetY,firstFrame}:null;
 };
"#;

pub fn observation_command()->(String,Value) {
    let script=r#"(() => {
      const pick='a[href],button,input:not([type=hidden]),textarea,select,[role=button],[contenteditable=true],[draggable=true]';
      const short=s=>String(s||'').replace(/\s+/g,' ').trim().slice(0,140);
      const path=el=>{
        const parts=[];
        for(let node=el;node&&node.nodeType===1&&parts.length<20;node=node.parentElement){
          let nth=1;
          for(let p=node.previousElementSibling;p;p=p.previousElementSibling)
            if(p.localName===node.localName)nth++;
          parts.unshift(node.localName+':nth-of-type('+nth+')');
          if(node.localName==='html')break;
        }
        return parts.join(' > ');
      };
      const bounds=el=>{
        const r=el.getBoundingClientRect();
        return {x:Math.round(r.left),y:Math.round(r.top),
          width:Math.round(r.width),height:Math.round(r.height)};
      };
      const safeUrl=url=>{
        try{
          const p=new URL(url,location.href);
          if(p.protocol!=='https:' && !(p.protocol==='http:'&&
             ['localhost','127.0.0.1','[::1]'].includes(p.hostname)))return '';
          return p.origin+p.pathname;
        }catch(e){return ''}
      };
      const controls=[],frames=[],queue=[{root:document,prefix:'',ox:0,oy:0}];
      let examined=0;
      while(queue.length && examined<30 && controls.length<70){
        const {root,prefix,ox,oy}=queue.shift();examined++;
        const choices=Array.from(root.querySelectorAll(pick)).slice(0,250);
        let collected=0;
        for(const el of choices){
          // Reserve observation capacity for open shadow roots and iframes.
          if(controls.length>=70||collected>=35)break;
          if(el.matches('input[type=password],input[type=file],input[type=hidden]'))continue;
          const r=bounds(el);
          if(r.width<1||r.height<1||el.disabled)continue;
          const selector=prefix+path(el);
          collected++;
          controls.push({selector,tag:el.localName,
            label:short(el.getAttribute('aria-label')||el.labels?.[0]?.innerText||
              el.innerText||el.placeholder||el.name),
            type:short(el.getAttribute('type')),role:short(el.getAttribute('role')),
            href:el.localName==='a'?safeUrl(el.href):'',
            editable:el.matches('input,textarea,[contenteditable=true]'),
            draggable:el.draggable===true,
            password:false,
            rect:{x:r.x+ox,y:r.y+oy,width:r.width,height:r.height}});
        }
        const hosts=Array.from(root.querySelectorAll('*')).slice(0,800);
        for(const host of hosts){
          if(queue.length>=25)break;
          if(host.shadowRoot){
            queue.push({root:host.shadowRoot,prefix:prefix+path(host)+' >>> ',ox,oy});
          }
          if(host.localName==='iframe'){
            const r=bounds(host), selector=prefix+path(host), src=safeUrl(host.src);
            let nested=null;
            try{nested=host.contentDocument;}catch(e){}
            frames.push({selector,url:src,sameOrigin:!!nested,
                rect:{x:r.x+ox,y:r.y+oy,width:r.width,height:r.height}});
            if(nested&&queue.length<25)queue.push({
              root:nested,prefix:selector+' ::frame ',ox:ox+r.x,oy:oy+r.y
            });
          }
        }
      }
      return {url:location.origin+location.pathname,title:short(document.title),
        ready:document.readyState,
        viewport:{width:innerWidth,height:innerHeight},
        excerpt:String(document.body?.innerText||'').slice(0,4800),
        controls,frames:frames.slice(0,30),shadowAndFrameRoots:examined};
    })()"#;
    ("Runtime.evaluate".into(),json!({
       "expression":script,"returnByValue":true,"awaitPromise":false,"userGesture":false
    }))
}
/// Resolve a unique element in the live DOM, traversing open shadow roots
/// and explicitly accessible same-origin frames.
pub fn element_command(selector:&str,kind:&str,value:Option<&str>)->Result<(String,Value),String>{
    if selector.is_empty()||selector.len()>512||selector.contains('\0'){
        return Err("Invalid selector".into())
    }
    let quoted=serde_json::to_string(selector).map_err(|e|e.to_string())?;
    let action=match kind {
        "position"=>r#"
          const r=el.getBoundingClientRect();
          if(r.width<1||r.height<1)return {error:'Element hidden'};
          el.scrollIntoView({block:'center',inline:'center'});
          const updated=el.getBoundingClientRect();
          const x=Math.round(updated.left+updated.width/2+hit.offsetX);
          const y=Math.round(updated.top+updated.height/2+hit.offsetY);
          const visible=document.elementFromPoint(x,y);
          if(!visible)return {error:'Element offscreen'};
          const shadow=el.getRootNode() instanceof ShadowRoot?el.getRootNode():null;
          const top=hit.firstFrame||(shadow?shadow.host:el);
          if(!(visible===top||visible.contains(top)||top.contains(visible)))
            return {error:'Target covered by another element'};
          if(shadow&&typeof shadow.elementFromPoint==='function'&&!hit.firstFrame){
            const inner=shadow.elementFromPoint(x,y);
            if(!inner||!(inner===el||el.contains(inner)||inner.contains(el)))
              return {error:'Shadow DOM target covered'};
          }
          if(hit.firstFrame){
            const local=hit.scope.elementFromPoint(x-hit.offsetX,y-hit.offsetY);
            const expected=shadow?shadow.host:el;
            if(!local||!(local===expected||local.contains(expected)||expected.contains(local)))
              return {error:'Iframe target covered'};
          }
          return {x,y,tag:el.localName};
        "#,
        "fill"=>r#"
          if(!el.matches('input,textarea,[contenteditable=true]'))
            return {error:'Not editable'};
          if(el.matches('input[type=password],input[type=file]'))
            return {error:'Sensitive input blocked'};
          const value=__VALUE__;
          if(el.matches('[contenteditable=true]')){el.focus();el.textContent=value;}
          else{
            const proto=el.localName==='textarea'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
            const setter=Object.getOwnPropertyDescriptor(proto,'value')?.set;
            if(!setter)return {error:'Input setter unavailable'};
            setter.call(el,value);el.focus();
          }
          el.dispatchEvent(new Event('input',{bubbles:true}));
          el.dispatchEvent(new Event('change',{bubbles:true}));
          return {ok:true};
        "#,
        _=>return Err("Unknown DOM operation".into())
    };
    let value=serde_json::to_string(value.unwrap_or("")).map_err(|e|e.to_string())?;
    let js=["(() => {",LOOKUP,
        " const hit=resolve(__SELECTOR__); if(!hit)return {error:'Selector missing or ambiguous'}; const el=hit.el;",
        action," })()"].concat();
    let js=js.replace("__SELECTOR__",&quoted).replace("__VALUE__",&value);
    Ok(("Runtime.evaluate".into(),json!({
        "expression":js,"returnByValue":true,"awaitPromise":false,"userGesture":false
    })))
}
pub fn screenshot_command()->(String,Value){
    ("Page.captureScreenshot".into(),json!({
      "format":"jpeg","quality":48,"fromSurface":true,
      "captureBeyondViewport":false,"optimizeForSpeed":true
    }))
}
pub fn frame_tree_command()->(String,Value){
    ("Page.getFrameTree".into(),json!({}))
}
pub fn dialog_command(accept:bool,prompt_text:Option<&str>)->Result<(String,Value),String>{
    if prompt_text.is_some_and(|s|s.len()>500||s.contains('\0')){
        return Err("Dialog prompt text too long".into())
    }
    let mut params=json!({"accept":accept});
    if let Some(value)=prompt_text{params["promptText"]=json!(value)}
    Ok(("Page.handleJavaScriptDialog".into(),params))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn screenshot_is_viewport_bound_and_jpeg(){
        let (m,v)=screenshot_command();
        assert_eq!(m,"Page.captureScreenshot");
        assert_eq!(v["captureBeyondViewport"],false);
    }
    #[test]fn shadow_and_frame_selectors_are_data_not_code(){
        let (m,v)=element_command("html > body > x-card >>> button","fill",Some("test\";alert(1)")).unwrap();
        assert_eq!(m,"Runtime.evaluate");
        let expr=v["expression"].as_str().unwrap();
        assert!(expr.contains("resolvePart"));
        assert!(expr.contains("test\\\";alert(1)"));
    }
    #[test]fn javascript_dialog_always_has_explicit_response(){
        assert_eq!(dialog_command(false,None).unwrap().1["accept"],false);
        assert!(dialog_command(true,Some(&"a".repeat(501))).is_err());
    }
}
