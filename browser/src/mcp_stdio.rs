//! Opt-in, allowlisted MCP STDIO transport for locally installed servers.
//! No shell invocation, remote code execution, arbitrary command or package
//! installation is performed by this module. The user must install packages
//! separately and explicitly enable each catalog entry.
use std::{
    io::{BufRead,BufReader,Write},
    process::{Child,ChildStdin,Command,Stdio},
    sync::mpsc::{self,Receiver},
    time::{Duration,Instant},
};
use serde_json::{json,Value};
use crate::mcp::Tool;

pub struct LocalConnection {
    child:Child,
    stdin:ChildStdin,
    responses:Receiver<Value>,
    next_id:u64,
    pub tools:Vec<Tool>,
    pub alias:String,
    pub server_id:String,
}
impl LocalConnection {
    pub fn connect(id:&str)->Result<Self,String>{
        let (program,args)=crate::mcp_catalog::local_command(id)
            .ok_or("Unknown or unsupported local MCP preset")?;
        // A direct executable+static arguments, *never* sh -c / cmd /c.
        // npm_config_yes=false prevents silent npx package installation.
        // A local process may see the user's environment; only install
        // trusted packages before enabling a preset.
        let mut child=Command::new(program).args(args)
            .env("npm_config_yes","false")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn().map_err(|e|format!("Cannot run preinstalled local MCP preset {id}: {e}"))?;
        let stdin=child.stdin.take().ok_or("Local MCP stdin unavailable")?;
        let stdout=child.stdout.take().ok_or("Local MCP stdout unavailable")?;
        let (tx,rx)=mpsc::sync_channel(16);
        std::thread::Builder::new().name(format!("charlotte-mcp-{id}"))
            .spawn(move||{
                let mut lines=BufReader::new(stdout);
                let mut line=String::new();
                loop{
                    line.clear();
                    let Ok(n)=lines.read_line(&mut line) else{break};
                    if n==0{break}
                    if n>2*1024*1024{break}
                    if let Ok(message)=serde_json::from_str::<Value>(&line){
                        if message.get("id").is_some()&&tx.send(message).is_err(){break}
                    }
                }
            }).map_err(|e|e.to_string())?;
        let mut conn=Self{
            child,stdin,responses:rx,next_id:0,tools:Vec::new(),
            alias:crate::mcp::alias_for(id),server_id:id.to_owned(),
        };
        conn.call("initialize",json!({
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"charlotte","version":"0.4.0"}
        }))?;
        conn.notify("notifications/initialized",json!({}))?;
        conn.refresh_tools()?;
        Ok(conn)
    }
    fn send(&mut self,value:Value)->Result<(),String>{
        let body=serde_json::to_vec(&value).map_err(|e|e.to_string())?;
        if body.len()>64*1024{return Err("Local MCP message is too large".into())}
        self.stdin.write_all(&body).map_err(|e|e.to_string())?;
        self.stdin.write_all(b"\n").map_err(|e|e.to_string())?;
        self.stdin.flush().map_err(|e|e.to_string())
    }
    pub fn call(&mut self,method:&str,params:Value)->Result<Value,String>{
        self.next_id=self.next_id.saturating_add(1);
        let id=self.next_id;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        let until=Instant::now()+Duration::from_secs(20);
        loop{
            let remaining=until.saturating_duration_since(Instant::now());
            if remaining.is_zero(){return Err("Local MCP timed out".into())}
            match self.responses.recv_timeout(remaining){
                Ok(message) if message["id"].as_u64()==Some(id)=>{
                    if let Some(error)=message.get("error"){
                        return Err(format!("Local MCP error: {}",error.to_string().chars().take(400).collect::<String>()))
                    }
                    return message.get("result").cloned().ok_or("Missing local MCP result".into())
                },
                Ok(_)=>{}, // Out-of-order or unrelated notifications are ignored.
                Err(mpsc::RecvTimeoutError::Timeout)=>return Err("Local MCP timed out".into()),
                Err(_)=>return Err("Local MCP process exited".into()),
            }
        }
    }
    fn notify(&mut self,method:&str,params:Value)->Result<(),String>{
        self.send(json!({"jsonrpc":"2.0","method":method,"params":params}))
    }
    fn refresh_tools(&mut self)->Result<(),String>{
        let response=self.call("tools/list",json!({}))?;
        let entries=response["tools"].as_array().ok_or("Local MCP server has no tools list")?;
        self.tools.clear();
        for item in entries.iter().take(120){
            let Some(name)=item["name"].as_str() else{continue};
            if !crate::mcp::valid_tool_name(name){continue}
            self.tools.push(Tool{
                server_id:self.server_id.clone(),original_name:name.into(),
                name:format!("mcp__{}__{}",self.alias,name),
                description:item["description"].as_str().unwrap_or("").chars().take(500).collect(),
                input_schema:item.get("inputSchema").cloned().unwrap_or(json!({"type":"object"})),
            });
        }
        Ok(())
    }
    pub fn invoke(&mut self,name:&str,args:Value)->Result<Value,String>{
        let allowed=self.tools.iter().any(|t|t.original_name==name);
        if !allowed{return Err("Local MCP tool not advertised".into())}
        self.call("tools/call",json!({"name":name,"arguments":args}))
    }
}
impl Drop for LocalConnection {
    fn drop(&mut self) {
        let _=self.child.kill();
        let _=self.child.wait();
    }
}
#[cfg(test)]
mod tests {
    #[test]fn local_preset_commands_are_vetted(){
        assert!(crate::mcp_catalog::local_command("playwright").is_some());
        assert!(crate::mcp_catalog::local_command("sh -c rm -rf").is_none());
    }
}
