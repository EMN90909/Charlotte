//! Explicit, interactive MCP OAuth authorization. Based on OAuth 2.1,
//! PKCE S256 and MCP discovery metadata. No client secrets stored in SQLite.
//! Vendor support depends on dynamic client registration (DCR) or a user-
//! supplied public client ID. This does not silently grant access to any app.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD,Engine as _};
use rand::RngCore;
use reqwest::{blocking::Client,redirect::Policy};
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use std::{
    io::{Read,Write},
    net::TcpListener,
    time::{Duration,Instant},
};
use url::Url;

fn https_url(raw:&str)->Result<Url,String>{
    let url=Url::parse(raw).map_err(|e|e.to_string())?;
    if url.scheme()!="https"||url.host_str().is_none()||
       !url.username().is_empty()||url.password().is_some()||
       url.fragment().is_some(){
        return Err("OAuth metadata endpoints must be authenticated HTTPS URLs".into())
    }
    Ok(url)
}
fn client()->Result<Client,String>{
    Client::builder().timeout(Duration::from_secs(15))
        .redirect(Policy::none()).build().map_err(|e|e.to_string())
}
fn get_json(client:&Client,link:&Url)->Result<Value,String>{
    let response=client.get(link.clone()).send().map_err(|e|e.to_string())?;
    if !response.status().is_success(){
        return Err(format!("OAuth metadata HTTP {}",response.status()))
    }
    if response.content_length().is_some_and(|n|n>128*1024){
        return Err("OAuth metadata too large".into())
    }
    let mut bytes=Vec::new();
    response.take(128*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>128*1024{return Err("OAuth metadata too large".into())}
    serde_json::from_slice(&bytes).map_err(|e|e.to_string())
}
fn metadata_candidates(url:&Url,resource:bool)->Vec<Url>{
    let mut urls=Vec::new();
    let root=if resource{"oauth-protected-resource"}else{"oauth-authorization-server"};
    // First try the endpoint-specific protected-resource location, then
    // origin-wide metadata. Never guess third-party authorization endpoints.
    if resource {
        let suffix=url.path().trim_start_matches('/');
        if !suffix.is_empty(){
            if let Ok(location)=url.join(&format!("/.well-known/{root}/{suffix}")){
                urls.push(location)
            }
        }
    }
    if let Ok(location)=url.join(&format!("/.well-known/{root}")){
        urls.push(location)
    }
    urls
}
fn discover(client:&Client,url:&Url,resource:bool)->Result<Value,String>{
    let mut last=String::from("No OAuth metadata");
    for candidate in metadata_candidates(url,resource){
        match get_json(client,&candidate){
            Ok(value)=>return Ok(value),
            Err(error)=>last=error,
        }
    }
    Err(last)
}
fn nonce(bytes:usize)->String{
    let mut random=vec![0u8;bytes];
    rand::rngs::OsRng.fill_bytes(&mut random);
    URL_SAFE_NO_PAD.encode(&random)
}
pub fn authorization_url(
    auth_endpoint:&Url,client_id:&str,redirect:&str,challenge:&str,state:&str,resource:&str
)->Result<Url,String>{
    if client_id.trim().is_empty(){return Err("OAuth client ID missing".into())}
    let mut auth=auth_endpoint.clone();
    auth.query_pairs_mut()
        .append_pair("response_type","code")
        .append_pair("client_id",client_id)
        .append_pair("redirect_uri",redirect)
        .append_pair("code_challenge",challenge)
        .append_pair("code_challenge_method","S256")
        .append_pair("state",state)
        .append_pair("resource",resource);
    Ok(auth)
}
fn register_public_client(
    http:&Client,metadata:&Value,redirect:&str
)->Result<String,String>{
    let url=metadata["registration_endpoint"].as_str().ok_or(
        "OAuth server does not advertise dynamic client registration; configure a public client ID instead"
    )?;
    let registration=https_url(url)?;
    let result=http.post(registration)
        .json(&json!({
            "client_name":"Charlotte native browser",
            "redirect_uris":[redirect],
            "grant_types":["authorization_code"],
            "response_types":["code"],
            "token_endpoint_auth_method":"none"
        })).send().map_err(|e|e.to_string())?;
    if !result.status().is_success(){
        return Err(format!("OAuth client registration HTTP {}",result.status()))
    }
    let value:Value=result.json().map_err(|e|e.to_string())?;
    let id=value["client_id"].as_str().filter(|id|!id.is_empty()&&id.len()<1024)
        .ok_or("OAuth registration did not yield a public client ID")?;
    Ok(id.to_owned())
}
fn await_callback(listener:&TcpListener,port:u16,expected_state:&str)->Result<String,String>{
    listener.set_nonblocking(true).map_err(|e|e.to_string())?;
    let deadline=Instant::now()+Duration::from_secs(180);
    while Instant::now()<deadline {
        match listener.accept(){
            Ok((mut stream,addr))=>{
                if !addr.ip().is_loopback(){continue}
                stream.set_read_timeout(Some(Duration::from_secs(4))).map_err(|e|e.to_string())?;
                let mut buffer=[0u8;4096];
                let mut n=0usize;
                loop{
                    if n==buffer.len(){return Err("OAuth callback headers too large".into())}
                    let read=stream.read(&mut buffer[n..]).map_err(|e|e.to_string())?;
                    if read==0 {break}
                    n+=read;
                    if buffer[..n].windows(4).any(|w|w==b"\r\n\r\n"){break}
                }
                let request=std::str::from_utf8(&buffer[..n])
                    .map_err(|_|"Invalid OAuth callback request")?;
                let Some(start)=request.lines().next() else{continue};
                let Some(path)=start.strip_prefix("GET ").and_then(|s|s.split_once(" HTTP/"))
                    .map(|(path,_)|path) else{continue};
                if !path.starts_with("/mcp/callback?"){continue}
                let url=Url::parse(&format!("http://127.0.0.1:{port}{path}"))
                    .map_err(|e|e.to_string())?;
                let mut code=None;
                let mut state=None;
                let mut denied=false;
                for (k,v) in url.query_pairs(){
                    match k.as_ref(){
                        "code"=>code=Some(v.into_owned()),
                        "state"=>state=Some(v.into_owned()),
                        "error"=>denied=true,
                        _=>{},
                    }
                }
                let ok=state.as_deref()==Some(expected_state)&&!denied&&code.is_some();
                let body=if ok{"Authorization received. Return to Charlotte settings."}
                    else{"MCP authorization declined or state mismatch. Return to Charlotte settings."};
                let status=if ok{"200 OK"}else{"400 Bad Request"};
                let response=format!("HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                let _=stream.write_all(response.as_bytes());
                if !ok{return Err("OAuth callback was denied or failed state verification".into())}
                return code.ok_or("OAuth callback missing authorization code".into());
            },
            Err(error) if error.kind()==std::io::ErrorKind::WouldBlock=>{
                std::thread::sleep(Duration::from_millis(120))
            },
            Err(error)=>return Err(error.to_string()),
        }
    }
    Err("OAuth authorization expired after three minutes".into())
}
/// This command is synchronous and must run only after an explicit user
/// invocation. It prints a URL for the user to open in Charlotte/any browser.
pub fn authorize(
    id:&str,endpoint:&str,configured_client_id:Option<&str>
)->Result<(),String>{
    crate::mcp::validate_server(id,endpoint)?;
    if endpoint.starts_with("local:"){
        return Err("Local STDIO MCP does not use HTTP OAuth".into())
    }
    let resource=https_url(endpoint)?;
    let http=client()?;
    let resource_meta=discover(&http,&resource,true)?;
    let issuer=resource_meta["authorization_servers"].as_array()
        .and_then(|arr|arr.first()).and_then(Value::as_str)
        .ok_or("MCP resource does not advertise an OAuth authorization server")?;
    let issuer=https_url(issuer)?;
    let auth_meta=discover(&http,&issuer,false)?;
    let metadata_issuer=auth_meta["issuer"].as_str()
        .ok_or("OAuth authorization server metadata has no issuer")?;
    if https_url(metadata_issuer)?.as_str().trim_end_matches('/')!=
        issuer.as_str().trim_end_matches('/'){
        return Err("OAuth issuer mismatch; refusing authorization-server switch".into())
    }
    let supported=auth_meta["code_challenge_methods_supported"].as_array()
        .is_some_and(|values|values.iter().any(|v|v.as_str()==Some("S256")));
    if !supported{return Err("OAuth server did not advertise required PKCE S256".into())}
    let auth_endpoint=https_url(auth_meta["authorization_endpoint"].as_str()
        .ok_or("OAuth authorization endpoint missing")?)?;
    let token_endpoint=https_url(auth_meta["token_endpoint"].as_str()
        .ok_or("OAuth token endpoint missing")?)?;
    let listener=TcpListener::bind(("127.0.0.1",0)).map_err(|e|e.to_string())?;
    let port=listener.local_addr().map_err(|e|e.to_string())?.port();
    let redirect=format!("http://127.0.0.1:{port}/mcp/callback");
    let client_id=match configured_client_id.filter(|s|!s.is_empty()){
        Some(id)=>id.to_owned(),
        None=>register_public_client(&http,&auth_meta,&redirect)?,
    };
    let verifier=nonce(32);
    let challenge=URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state=nonce(24);
    let link=authorization_url(&auth_endpoint,&client_id,&redirect,
        &challenge,&state,resource.as_str())?;
    println!("Authorize Charlotte MCP profile '{id}' by opening this URL in your browser:");
    println!("{link}");
    println!("Charlotte will accept a single local callback for up to three minutes.");
    let code=await_callback(&listener,port,&state)?;
    let response=http.post(token_endpoint).form(&[
        ("grant_type","authorization_code"),
        ("client_id",client_id.as_str()),
        ("code",code.as_str()),
        ("code_verifier",verifier.as_str()),
        ("redirect_uri",redirect.as_str()),
        ("resource",resource.as_str()),
    ]).send().map_err(|e|e.to_string())?;
    if !response.status().is_success(){
        return Err(format!("MCP OAuth token exchange HTTP {}",response.status()))
    }
    let body:Value=response.json().map_err(|e|e.to_string())?;
    if body["token_type"].as_str().is_some_and(|kind|!kind.eq_ignore_ascii_case("bearer")){
        return Err("OAuth token type is not Bearer".into())
    }
    let token=body["access_token"].as_str().filter(|value|
        !value.is_empty()&&value.len()<=16_000)
        .ok_or("OAuth response did not contain an access token")?;
    crate::credentials::store(&format!("mcp:{id}"),token)
        .map_err(|e|e.to_string())?;
    if let Some(refresh)=body["refresh_token"].as_str().filter(|v|v.len()<=16_000){
        crate::credentials::store(&format!("mcp-refresh:{id}"),refresh)
            .map_err(|e|e.to_string())?;
    }
    println!("MCP OAuth account authorization saved in the OS credential store for '{id}'.");
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn pkce_authorization_url_cannot_skip_state(){
        let endpoint=https_url("https://authorize.example/oauth").unwrap();
        let link=authorization_url(&endpoint,"public-id","http://127.0.0.1:8888/mcp/callback",
            "challenge","state-value","https://mcp.example/mcp").unwrap();
        assert_eq!(link.query_pairs().find(|(k,_)|k=="code_challenge_method").unwrap().1,"S256");
        assert_eq!(link.query_pairs().find(|(k,_)|k=="state").unwrap().1,"state-value");
    }
    #[test]fn metadata_endpoints_never_include_resource_query(){
        let url=https_url("https://mcp.example/mcp?read_only=true").unwrap();
        assert!(metadata_candidates(&url,true).iter().all(|v|v.query().is_none()));
    }
}
