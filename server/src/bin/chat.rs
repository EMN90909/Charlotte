//! Pure Rust interactive Charlotte HTTP chat client.
use std::{collections::HashMap,env,io::{self,BufRead},time::{Duration,SystemTime,UNIX_EPOCH}};
use reqwest::Client;
use serde_json::{json,Value};
#[tokio::main]
async fn main()->Result<(),Box<dyn std::error::Error>>{
    let mut args:HashMap<String,String>=HashMap::new();
    let mut it=env::args().skip(1);
    while let Some(k)=it.next(){let v=it.next().ok_or("expected a value after flag")?;args.insert(k.trim_start_matches('-').to_string(),v);}
    let base=args.get("server").map(String::as_str).unwrap_or("http://localhost:8080").trim_end_matches('/').to_string();
    let room=args.get("url").ok_or("usage: charlotte-chat -url <room-url> -username <name> [-token JWT] [-server URL]")?;
    if !args.contains_key("username")&&!args.contains_key("token"){return Err("set -username or -token".into())}
    let transport=args.get("transport").map(String::as_str).unwrap_or("sse").to_string();
    if !matches!(transport.as_str(),"sse"|"poll"){return Err("-transport must be sse or poll".into())}
    let session=args.get("session").cloned().unwrap_or_else(||{
        let n=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        format!("cli-{}-{n}",std::process::id())
    });
    let req=json!({"url":room,"username":args.get("username"),"token":args.get("token"),
        "browser":args.get("browser").map(String::as_str).unwrap_or("chat-cli"),
        "region":args.get("region").map(String::as_str).unwrap_or("local"),"session_id":session});
    let client=Client::new();
    let response=client.post(format!("{base}/connect")).json(&req).send().await?.error_for_status()?;
    let resp:Value=response.json().await?;
    let token=resp["token"].as_str().ok_or("missing token")?.to_string();
    let cursor=resp["cursor"].as_u64().ok_or("missing cursor")?;
    eprintln!("connected; cursor {cursor}; token for reconnect:\n{token}\n");
    let rxclient=client.clone();let rxbase=base.clone();let rx_token=token.clone();
    tokio::spawn(async move{
        let mut cursor=cursor;
        loop{
            if transport=="poll" {
                match rxclient.get(format!("{rxbase}/poll?cursor={cursor}")).bearer_auth(&rx_token).send().await {
                    Ok(r) if r.status()==reqwest::StatusCode::NOT_MODIFIED=>{},
                    Ok(r) if r.status().is_success()=>{
                        if let Ok(value)=r.json::<Value>().await {
                            if let Some(msgs)=value["messages"].as_array(){
                                for m in msgs{println!("{}: {}",m["sender"].as_str().unwrap_or("?"),m["content"].as_str().unwrap_or(""));}
                            }
                            cursor=value["cursor"].as_u64().unwrap_or(cursor);
                        }
                    },
                    Err(e)=>eprintln!("poll error: {e}"),
                    _=>eprintln!("poll HTTP error"),
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }else{
                match rxclient.get(format!("{rxbase}/events?cursor={cursor}")).bearer_auth(&rx_token).send().await {
                    Ok(mut response)=>{
                        let mut pending=String::new();
                        while let Ok(Some(bytes))=response.chunk().await {
                            pending.push_str(&String::from_utf8_lossy(&bytes));
                            while let Some(pos)=pending.find('\n') {
                                let line=pending[..pos].trim_end_matches('\r').to_string();
                                pending.drain(..=pos);
                                if let Some(json)=line.strip_prefix("data: "){
                                    if let Ok(msg)=serde_json::from_str::<Value>(json){
                                        if let Some(seq)=msg["seq"].as_u64(){if seq<=cursor {continue;}cursor=seq;}
                                        println!("{}: {}",msg["sender"].as_str().unwrap_or("?"),msg["content"].as_str().unwrap_or(""));
                                    }
                                }
                            }
                        }
                    }
                    Err(e)=>eprintln!("SSE disconnected: {e}"),
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    });
    println!("Type a message then press Enter. Ctrl+D quits.");
    let stdin=io::stdin();
    for line in stdin.lock().lines(){
        let line=line?;
        if line.is_empty(){continue;}
        let response=client.post(format!("{base}/send")).bearer_auth(&token).json(&json!({"content":line})).send().await?;
        if !response.status().is_success(){eprintln!("send failed: {}",response.status());}
    }
    Ok(())
}
