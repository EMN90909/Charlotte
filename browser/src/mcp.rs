//! Native Rust MCP Streamable HTTP client. Tool schemas and results are
//! transported as JSON-RPC values. No JavaScript, Node, or shell execution.
use reqwest::blocking::Client;
use reqwest::{header,redirect::Policy};
use serde_json::{json,Value};
use std::{collections::HashMap,io::Read,time::Duration};
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
            endpoint,client:Client::builder().timeout(Duration::from_secs(30))
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
        let mut bytes=Vec::new();
        response.take(2*1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        if bytes.len()>2*1024*1024{return Err("MCP response exceeds 2 MB".into())}
        if bytes.is_empty(){return Ok(None)}
        if mime.starts_with("text/event-stream"){
            let text=String::from_utf8(bytes).map_err(|_|"Malformed SSE response")?;
            let mut data=String::new();
            for line in text.lines(){
                if let Some(part)=line.strip_prefix("data:") {
                    data.push_str(part.trim_start());
                } else if line.is_empty()&&!data.is_empty(){
                    let result=serde_json::from_str(&data).map_err(|e|e.to_string())?;
                    return Ok(Some(result));
                }
            }
            if !data.is_empty(){return serde_json::from_str(&data).map(Some).map_err(|e|e.to_string())}
            return Err("No JSON-RPC response in SSE stream".into())
        }
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
}
