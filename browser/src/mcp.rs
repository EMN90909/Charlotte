//! Native Rust MCP Streamable HTTP client. Tool schemas and results are
//! transported as JSON-RPC values. No JavaScript, Node, or shell execution.
use reqwest::blocking::Client;
use reqwest::{header,redirect::Policy};
use serde_json::{json,Value};
use std::{collections::HashMap,io::{BufRead,BufReader,Read},time::Duration};
use url::Url;
#[derive(Clone,Debug)]
pub struct Tool {
    pub server_id:String,pub name:String,pub original_name:String,
    pub description:String,pub input_schema:Value,
}
pub struct Connection {
    endpoint:Url,
    client:Client,
    bearer:Option<String>,
    session:Option<String>,
    next_id:u64,
    tools:Vec<Tool>,
    alias:String,
    server_id:String,
}
#[derive(Clone,Debug,serde::Serialize,serde::Deserialize,PartialEq,Eq)]
pub struct ServerConfig { pub id:String, pub url:String }
const CONFIG_KEY:&str="charlotte:mcp:servers:v1";
pub fn saved_servers(db:&crate::storage::Storage)->Result<Vec<ServerConfig>,String>{
    let raw=db.setting(CONFIG_KEY).map_err(|e|e.to_string())?;
    let servers:Vec<ServerConfig>=match raw {
        None=>Vec::new(),
        Some(text)=>serde_json::from_str(&text).map_err(|e|e.to_string())?,
    };
    if servers.len()>48 {return Err("Too many MCP servers (maximum 48)".into())}
    for server in &servers {validate_server(&server.id,&server.url)?}
    Ok(servers)
}
pub fn validate_server(id:&str,raw:&str)->Result<(),String>{
    if id.is_empty()||id.len()>40||
        !id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_'){
        return Err("MCP server ID must be 1-40 letters, digits, hyphens or underscores".into())
    }
    if raw.len()>1024 {return Err("MCP URL too long".into())}
    if let Some(local)=raw.strip_prefix("local:"){
        if local!=id || (crate::mcp_catalog::local_command(local).is_none()&&local!="sqlite"){
            return Err("Unapproved local MCP executable; choose a catalog preset".into())
        }
        return Ok(())
    }
    let endpoint=Url::parse(raw).map_err(|e|e.to_string())?;
    let permitted_query=endpoint.host_str()==Some("mcp.supabase.com") &&
        endpoint.path()=="/mcp" &&
        endpoint.query_pairs().collect::<Vec<_>>()==vec![("read_only".into(),"true".into())];
    if !endpoint.username().is_empty()||endpoint.password().is_some()||
       (endpoint.query().is_some()&&!permitted_query)||endpoint.fragment().is_some() {
        return Err("MCP endpoint cannot contain credentials, query or fragment".into())
    }
    let host=endpoint.host_str().ok_or("MCP endpoint requires host")?;
    if endpoint.scheme()!="https" &&
       !(endpoint.scheme()=="http"&&matches!(host,"localhost"|"127.0.0.1"|"::1")){
        return Err("MCP requires HTTPS except local loopback".into())
    }
    Ok(())
}
pub fn save_server(db:&crate::storage::Storage,id:&str,url:&str)->Result<(),String>{
    validate_server(id,url)?;
    let mut servers=saved_servers(db)?;
    if let Some(existing)=servers.iter_mut().find(|s|s.id==id){
        existing.url=url.to_owned();
    } else {
        if servers.len()>=48{return Err("Maximum 48 configured MCP servers".into())}
        servers.push(ServerConfig{id:id.to_owned(),url:url.to_owned()});
    }
    db.set_setting(CONFIG_KEY,&serde_json::to_string(&servers).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}
/// Register a catalog entry without connecting, authorizing OAuth, downloading
/// local packages, or opening any third-party account.
pub fn save_preset(db:&crate::storage::Storage,id:&str)->Result<(),String>{
    let endpoint=crate::mcp_catalog::selected_url(id)?;
    save_server(db,id,endpoint)
}
pub fn remove_server(db:&crate::storage::Storage,id:&str)->Result<(),String>{
    let mut servers=saved_servers(db)?;
    servers.retain(|s|s.id!=id);
    db.set_setting(CONFIG_KEY,&serde_json::to_string(&servers).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}
/// Each task connects only the most relevant saved, explicitly opted-in
/// profiles. Vendors still require the user's own OAuth or API credentials.
pub fn connect_saved_for(
    db:&crate::storage::Storage,goal:&str,max:usize
)->Result<Registry,String>{
    let servers=saved_servers(db)?;
    let words:Vec<String>=goal.split(|c:char|!c.is_ascii_alphanumeric())
        .filter(|w|w.len()>2).map(|w|w.to_ascii_lowercase()).collect();
    let mut ranked:Vec<(usize,i32)>=servers.iter().enumerate().map(|(i,server)|{
        let description=crate::mcp_catalog::get(&server.id).map(|preset|
            format!("{} {} {}",preset.name,preset.category,preset.keywords)).unwrap_or_default();
        let haystack=format!("{} {}",server.id,description).to_ascii_lowercase();
        let score=words.iter().map(|word|{
            if server.id.contains(word.as_str()){12}
            else if haystack.split_whitespace().any(|term|term==word){5}
            else if haystack.contains(word){2}
            else{0}
        }).sum::<i32>();
        (i,score)
    }).collect();
    ranked.sort_by(|a,b|b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut registry=Registry::new();
    let limit=max.clamp(1,5);
    for (index,_) in ranked.into_iter().take(limit){
        let server=&servers[index];
        let token=crate::credentials::retrieve(&format!("mcp:{}",server.id)).ok();
        if let Err(error)=registry.connect_config(&server.id,&server.url,token){
            eprintln!("Charlotte MCP profile {} unavailable: {}",server.id,error);
        }
    }
    Ok(registry)
}
pub fn connect_saved(db:&crate::storage::Storage)->Result<Registry,String>{
    connect_saved_for(db,"",5)
}
pub fn connect_selected(db:&crate::storage::Storage,id:&str)->Result<Registry,String>{
    let server=saved_servers(db)?.into_iter().find(|s|s.id==id)
        .ok_or("MCP server not enabled")?;
    let token=crate::credentials::retrieve(&format!("mcp:{}",id)).ok();
    let mut registry=Registry::new();
    registry.connect_config(&server.id,&server.url,token)?;
    Ok(registry)
}
/// Tool namespace identity is derived from the saved server ID, NOT connection
/// order. This prevents attaching an approved request to a different server
/// when on-demand selection changes between planning and execution.
pub(crate) fn alias_for(id:&str)->String{
    let mut value=String::from("s");
    for byte in id.bytes(){value.push_str(&format!("{byte:02x}"))}
    value
}
pub(crate) fn valid_tool_name(name:&str)->bool{
    !name.is_empty()&&name.len()<=200&&
        name.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-')
}
pub struct Registry {
    connections:HashMap<String,Connection>,
    local:HashMap<String,crate::mcp_stdio::LocalConnection>,
    sqlite:Option<crate::mcp_sqlite::SqliteConnection>,
}
impl Registry {
    pub fn new()->Self{Self{connections:HashMap::new(),local:HashMap::new(),sqlite:None}}
    pub fn connect_config(&mut self,id:&str,endpoint:&str,token:Option<String>)
        ->Result<&[Tool],String>{
        validate_server(id,endpoint)?;
        if endpoint=="local:sqlite"&&id=="sqlite"{
            if self.sqlite.is_none(){
                self.sqlite=Some(crate::mcp_sqlite::SqliteConnection::open()?);
            }
            return Ok(&self.sqlite.as_ref().ok_or("SQLite MCP not connected")?.tools)
        }
        if endpoint==format!("local:{id}"){
            if !self.local.contains_key(id){
                let conn=crate::mcp_stdio::LocalConnection::connect(id)?;
                self.local.insert(id.to_owned(),conn);
            }
            return Ok(&self.local.get(id).ok_or("Local MCP not connected")?.tools)
        }
        self.connect(id,endpoint,token)
    }
    pub fn connect(&mut self,id:&str,raw:&str,token:Option<String>)->Result<&[Tool],String>{
        if self.connections.contains_key(id){
            return Ok(&self.connections.get(id).expect("checked").tools)
        }
        if id.is_empty()||id.len()>128{return Err("Invalid MCP server id".into())}
        let endpoint=Url::parse(raw).map_err(|e|e.to_string())?;
        validate_server(id,raw)?;
        if raw.starts_with("local:"){return Err("Use connect_config for STDIO".into())}
        let host=endpoint.host_str().ok_or("MCP endpoint requires a host")?;
        if endpoint.scheme()!="https"&& !(endpoint.scheme()=="http"&&
           matches!(host,"localhost"|"127.0.0.1"|"::1")){
            return Err("MCP must use HTTPS except on loopback".into())
        }
        let alias=alias_for(id);
        let mut conn=Connection{
            endpoint,client:Client::builder().connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(20))
                .redirect(Policy::none()).build().map_err(|e|e.to_string())?,
            bearer:token,session:None,next_id:0,tools:Vec::new(),alias,server_id:id.to_owned(),
        };
        conn.call("initialize",json!({"protocolVersion":"2025-06-18",
          "capabilities":{},"clientInfo":{"name":"charlotte","version":"0.4.0"}}))?;
        let _=conn.notification("notifications/initialized",json!({}));
        conn.refresh_tools()?;
        self.connections.insert(id.to_owned(),conn);
        Ok(&self.connections.get(id).expect("inserted").tools)
    }
    pub fn tools(&self)->Vec<Tool>{
        let mut list:Vec<Tool>=self.connections.values().flat_map(|c|c.tools.iter().cloned())
            .chain(self.local.values().flat_map(|c|c.tools.iter().cloned()))
            .chain(self.sqlite.iter().flat_map(|c|c.tools.iter().cloned())).collect();
        list.sort_by(|a,b|a.server_id.cmp(&b.server_id).then(a.name.cmp(&b.name)));
        list
    }
    pub fn call(&mut self,alias_name:&str,args:Value)->Result<Value,String>{
        if !args.is_object(){return Err("MCP tool arguments must be a JSON object".into())}
        let (alias,name)=parse_name(alias_name).ok_or("Invalid namespaced MCP tool")?;
        if let Some(c)=self.connections.values_mut().find(|c|c.alias==alias){
            if !c.tools.iter().any(|t|t.name==alias_name&&t.original_name==name){
                return Err("MCP tool is not advertised by the server".into())
            }
            return c.call("tools/call",json!({"name":name,"arguments":args}))
        }
        if let Some(c)=self.local.values_mut().find(|c|c.alias==alias){
            if !c.tools.iter().any(|t|t.name==alias_name&&t.original_name==name){
                return Err("Local MCP tool is not advertised by the server".into())
            }
            return c.invoke(name,args)
        }
        if let Some(c)=self.sqlite.as_mut().filter(|c|c.alias==alias){
            if !c.tools.iter().any(|t|t.name==alias_name&&t.original_name==name){
                return Err("SQLite MCP tool is not advertised".into())
            }
            return c.invoke(name,args)
        }
        Err("MCP server is not connected".into())
    }
    pub fn disconnect(&mut self,id:&str){self.connections.remove(id);self.local.remove(id);if id=="sqlite"{self.sqlite=None;}}
}
fn parse_name(name:&str)->Option<(&str,&str)> {
    let rest=name.strip_prefix("mcp__")?;
    let (alias,tool)=rest.split_once("__")?;
    if alias.len()<3||tool.is_empty()||!alias.starts_with('s')||
        alias[1..].len()%2!=0 ||
        !alias[1..].bytes().all(|b|b.is_ascii_hexdigit()){return None}
    Some((alias,tool))
}
impl Connection {
    fn request(&mut self,body:Value)->Result<Option<Value>,String>{
        let mut request=self.client.post(self.endpoint.clone())
            .header(header::ACCEPT,"application/json, text/event-stream")
            .header("mcp-protocol-version","2025-06-18")
            .json(&body);
        if let Some(token)=&self.bearer{request=request.bearer_auth(token)}
        if let Some(session)=&self.session{request=request.header("mcp-session-id",session)}
        let mut response=request.send().map_err(|e|e.to_string())?;
        if !response.status().is_success(){return Err(format!("MCP HTTP {}",response.status()))}
        if let Some(value)=response.headers().get("mcp-session-id"){
            let session=value.to_str().map_err(|_|"Invalid MCP session header")?.to_owned();
            if session.len()>256{return Err("Invalid MCP session length".into())}
            self.session=Some(session);
        }
        let mime=response.headers().get(header::CONTENT_TYPE).and_then(|v|v.to_str().ok()).unwrap_or("").to_owned();
        const LIMIT:usize=2*1024*1024;
        if mime.starts_with("text/event-stream"){
            // Streamable HTTP MCP may keep the SSE connection alive after
            // sending its JSON-RPC reply; read until *our* reply arrives,
            // rather than blocking on read_to_end() until server shutdown.
            let expected=body.get("id").and_then(Value::as_u64);
            let mut reader=BufReader::new(response.take((LIMIT+1) as u64));
            let mut line=String::new();
            let mut data=String::new();
            let mut used=0usize;
            loop {
                line.clear();
                let size=reader.read_line(&mut line).map_err(|e|e.to_string())?;
                used=used.saturating_add(size);
                if used>LIMIT{return Err("MCP SSE stream exceeded 2 MB".into())}
                if size==0 {break}
                if let Some(fragment)=line.strip_prefix("data:"){
                    data.push_str(fragment.trim());
                } else if line.trim().is_empty() && !data.is_empty(){
                    if let Ok(value)=serde_json::from_str::<Value>(&data){
                        if expected.is_none() ||
                           value.get("id").and_then(Value::as_u64)==expected {
                            return Ok(Some(value))
                        }
                    }
                    data.clear();
                }
            }
            if !data.is_empty(){
                let value:Value=serde_json::from_str(&data).map_err(|e|e.to_string())?;
                if expected.is_none()||value.get("id").and_then(Value::as_u64)==expected{
                    return Ok(Some(value))
                }
            }
            return Err("MCP SSE stream ended without matching JSON-RPC response".into())
        }
        let mut bytes=Vec::new();
        response.take((LIMIT+1) as u64).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        if bytes.len()>LIMIT{return Err("MCP response exceeds 2 MB".into())}
        if bytes.is_empty(){return Ok(None)}
        serde_json::from_slice(&bytes).map(Some).map_err(|e|e.to_string())
    }
    fn call(&mut self,method:&str,params:Value)->Result<Value,String>{
        self.next_id=self.next_id.saturating_add(1);
        let id=self.next_id;
        let response=self.request(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?.ok_or("Missing MCP response")?;
        if response.get("id").and_then(Value::as_u64)!=Some(id){
            return Err("MCP response id mismatch".into())
        }
        if let Some(error)=response.get("error"){return Err(format!("MCP error: {error}"))}
        response.get("result").cloned().ok_or("MCP response has no result".into())
    }
    fn notification(&mut self,method:&str,params:Value)->Result<(),String>{
        self.request(json!({"jsonrpc":"2.0","method":method,"params":params}))?;Ok(())
    }
    pub fn refresh_tools(&mut self)->Result<(),String>{
        let result=self.call("tools/list",json!({}))?;
        let list=result.get("tools").and_then(Value::as_array).ok_or("MCP server returned no tools array")?;
        let mut tools=Vec::new();
        for entry in list.iter().take(300){
            let Some(name)=entry.get("name").and_then(Value::as_str) else {continue};
            if !valid_tool_name(name){continue}
            tools.push(Tool{
                server_id:self.server_id.clone(),original_name:name.to_owned(),
                name:format!("mcp__{}__{name}",self.alias),
                description:entry.get("description").and_then(Value::as_str).unwrap_or("").to_owned(),
                input_schema:entry.get("inputSchema").cloned().unwrap_or(json!({"type":"object","properties":{}})),
            });
        }
        self.tools=tools;
        Ok(())
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn namespaced_tools(){assert_eq!(parse_name("mcp__s616263__search_files"),Some(("s616263","search_files")));assert!(parse_name("mcp__bad__hi").is_none());}
    #[test]fn registry_starts_empty(){assert!(Registry::new().tools().is_empty());}
    #[test]fn mcp_config_validates_remote_endpoints(){
        assert!(validate_server("search","https://example.org/mcp").is_ok());
        assert!(validate_server("local","http://127.0.0.1:8080/mcp").is_ok());
        assert!(validate_server("oops","http://evil.example/mcp").is_err());
        assert!(validate_server("oops","https://username:password@host/mcp").is_err());
        assert!(validate_server("../oops","https://host/mcp").is_err());
    }
    #[test]fn stable_server_namespace_is_independent_of_connection_order(){
        assert_eq!(alias_for("github"),alias_for("github"));
        assert_ne!(alias_for("github"),alias_for("gitlab"));
        let name=format!("mcp__{}__list_issues",alias_for("github"));
        assert_eq!(parse_name(&name).unwrap().1,"list_issues");
    }
    #[test]fn read_only_url_and_local_allowlist_are_strict(){
        assert!(validate_server("supabase","https://mcp.supabase.com/mcp?read_only=true").is_ok());
        assert!(validate_server("supabase","https://mcp.supabase.com/mcp?read_only=false").is_err());
        assert!(validate_server("playwright","local:playwright").is_ok());
        assert!(validate_server("sqlite","local:sqlite").is_ok());
        assert!(validate_server("unknown","local:unknown").is_err());
    }
    #[test]fn persisted_server_ids_are_bounded(){
        let db=crate::storage::Storage::open(std::path::Path::new(":memory:")).unwrap();
        save_server(&db,"documents","https://docs.example/mcp").unwrap();
        assert_eq!(saved_servers(&db).unwrap()[0].id,"documents");
        remove_server(&db,"documents").unwrap();
        assert!(saved_servers(&db).unwrap().is_empty());
    }
}
