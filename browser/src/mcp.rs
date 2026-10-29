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
    if servers.len()>8 {return Err("Too many MCP servers".into())}
    for server in &servers {validate_server(&server.id,&server.url)?}
    Ok(servers)
}
pub fn validate_server(id:&str,raw:&str)->Result<(),String>{
    if id.is_empty()||id.len()>40||
        !id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_'){
        return Err("MCP server ID must be 1-40 letters, digits, hyphens or underscores".into())
    }
    if raw.len()>1024 {return Err("MCP URL too long".into())}
    let endpoint=Url::parse(raw).map_err(|e|e.to_string())?;
    if !endpoint.username().is_empty()||endpoint.password().is_some()||
       endpoint.query().is_some()||endpoint.fragment().is_some() {
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
        if servers.len()>=8{return Err("Maximum eight MCP servers".into())}
        servers.push(ServerConfig{id:id.to_owned(),url:url.to_owned()});
    }
    db.set_setting(CONFIG_KEY,&serde_json::to_string(&servers).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}
pub fn remove_server(db:&crate::storage::Storage,id:&str)->Result<(),String>{
    let mut servers=saved_servers(db)?;
    servers.retain(|s|s.id!=id);
    db.set_setting(CONFIG_KEY,&serde_json::to_string(&servers).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}
/// Call on background workers only. Secrets remain in the OS credential store.
pub fn connect_saved(db:&crate::storage::Storage)->Result<Registry,String>{
    let mut registry=Registry::new();
    for server in saved_servers(db)?{
        let token=crate::credentials::retrieve(&format!("mcp:{}",server.id)).ok();
        if let Err(e)=registry.connect(&server.id,&server.url,token){
            eprintln!("Charlotte MCP server {} not reachable: {}",server.id,e);
        }
    }
    Ok(registry)
}
pub struct Registry {connections:HashMap<String,Connection>,next_alias:usize}
impl Registry {
    pub fn new()->Self{Self{connections:HashMap::new(),next_alias:0}}
    pub fn connect(&mut self,id:&str,raw:&str,token:Option<String>)->Result<&[Tool],String>{
        if self.connections.contains_key(id){
            return Ok(&self.connections.get(id).expect("checked").tools)
        }
        if id.is_empty()||id.len()>128{return Err("Invalid MCP server id".into())}
        let endpoint=Url::parse(raw).map_err(|e|e.to_string())?;
        if endpoint.username()!=""||endpoint.password().is_some()||
           endpoint.query().is_some()||endpoint.fragment().is_some(){return Err("Unsafe MCP endpoint".into())}
        let host=endpoint.host_str().ok_or("MCP endpoint requires a host")?;
        if endpoint.scheme()!="https"&& !(endpoint.scheme()=="http"&&
           matches!(host,"localhost"|"127.0.0.1"|"::1")){
            return Err("MCP must use HTTPS except on loopback".into())
        }
        let alias=format!("s{}",self.next_alias);
        let mut conn=Connection{
            endpoint,client:Client::builder().connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(20))
                .redirect(Policy::none()).build().map_err(|e|e.to_string())?,
            bearer:token,session:None,next_id:0,tools:Vec::new(),alias,server_id:id.to_owned(),
        };
        conn.call("initialize",json!({"protocolVersion":"2025-06-18",
          "capabilities":{},"clientInfo":{"name":"charlotte","version":"0.4.0"}}))?;
        let _=conn.notification("notifications/initialized",json!({}));
        conn.refresh_tools()?;
        self.next_alias+=1;self.connections.insert(id.to_owned(),conn);
        Ok(&self.connections.get(id).expect("inserted").tools)
    }
    pub fn tools(&self)->Vec<Tool>{
        self.connections.values().flat_map(|c|c.tools.iter().cloned()).collect()
    }
    pub fn call(&mut self,alias_name:&str,args:Value)->Result<Value,String>{
        if !args.is_object(){return Err("MCP tool arguments must be a JSON object".into())}
        let (alias,name)=parse_name(alias_name).ok_or("Invalid namespaced MCP tool")?;
        let c=self.connections.values_mut().find(|c|c.alias==alias).ok_or("MCP server not connected")?;
        if !c.tools.iter().any(|t|t.name==alias_name&&t.original_name==name){
            return Err("MCP tool is not advertised by the server".into())
        }
        c.call("tools/call",json!({"name":name,"arguments":args}))
    }
    pub fn disconnect(&mut self,id:&str){self.connections.remove(id);}
}
fn parse_name(name:&str)->Option<(&str,&str)> {
    let rest=name.strip_prefix("mcp__")?;
    let (alias,tool)=rest.split_once("__")?;
    if alias.is_empty()||tool.is_empty()||!alias.starts_with('s')||
       !alias[1..].bytes().all(|b|b.is_ascii_digit()){return None}
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
            if name.is_empty()||name.len()>200||!name.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-'){continue}
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
    #[test]fn namespaced_tools(){assert_eq!(parse_name("mcp__s2__search_files"),Some(("s2","search_files")));assert!(parse_name("mcp__bad__hi").is_none());}
    #[test]fn registry_starts_empty(){assert!(Registry::new().tools().is_empty());}
    #[test]fn mcp_config_validates_remote_endpoints(){
        assert!(validate_server("search","https://example.org/mcp").is_ok());
        assert!(validate_server("local","http://127.0.0.1:8080/mcp").is_ok());
        assert!(validate_server("oops","http://evil.example/mcp").is_err());
        assert!(validate_server("oops","https://username:password@host/mcp").is_err());
        assert!(validate_server("../oops","https://host/mcp").is_err());
    }
    #[test]fn persisted_server_ids_are_bounded(){
        let db=crate::storage::Storage::open(std::path::Path::new(":memory:")).unwrap();
        save_server(&db,"documents","https://docs.example/mcp").unwrap();
        assert_eq!(saved_servers(&db).unwrap()[0].id,"documents");
        remove_server(&db,"documents").unwrap();
        assert!(saved_servers(&db).unwrap().is_empty());
    }
}
