//! Rust Supabase account authentication. Refresh and access tokens never
//! enter CEF webpages or SQLite; persist only through the OS credential store.
use reqwest::blocking::{Client,RequestBuilder};
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use std::time::{Duration,SystemTime,UNIX_EPOCH};
use url::Url;
use crate::credentials;
const SERVICE_KEY:&str="account:supabase:session";
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct AccountStatus{
    pub configured:bool,pub authenticated:bool,pub email:Option<String>,
    pub persistent:bool,pub message:Option<String>,
}
#[derive(Debug,Clone,Serialize,Deserialize)]
struct Session {
    access_token:String,refresh_token:String,expires_at:u64,email:Option<String>,
}
pub struct SupabaseAuth {
    client:Client,url:Url,publishable:String,
}
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()}
fn error(msg:impl std::fmt::Display)->String{msg.to_string()}
impl SupabaseAuth {
    pub fn configured()->Result<Self,String>{
        let base=std::env::var("CHARLOTTE_SUPABASE_URL").map_err(|_|error("Missing CHARLOTTE_SUPABASE_URL"))?;
        let url=Url::parse(&base).map_err(error)?;
        let host=url.host_str().ok_or("Supabase URL needs a host")?;
        if url.scheme()!="https" && !(url.scheme()=="http" && matches!(host,"localhost"|"127.0.0.1")) {
            return Err("Supabase requires HTTPS (or loopback HTTP for development)".into());
        }
        if url.username()!=""||url.password().is_some()||url.query().is_some()||url.fragment().is_some(){
            return Err("Supabase URL must not contain credentials, queries, or fragments".into());
        }
        let publishable=std::env::var("CHARLOTTE_SUPABASE_PUBLISHABLE_KEY").map_err(|_|error("Missing Supabase publishable key"))?;
        if !publishable.starts_with("sb_publishable_"){
            return Err("Only a Supabase publishable key is supported".into());
        }
        Ok(Self{client:Client::builder().timeout(Duration::from_secs(10)).redirect(reqwest::redirect::Policy::none()).build().map_err(error)?,url,publishable})
    }
    fn request(&self,path:&str)->Result<RequestBuilder,String>{
        let u=self.url.join(path).map_err(error)?;
        if u.origin()!=self.url.origin(){return Err("Supabase endpoint outside configured origin".into())}
        Ok(self.client.post(u).header("apikey",&self.publishable).header("content-type","application/json"))
    }
    fn json(&self,request:RequestBuilder)->Result<Value,String>{
        let response=request.send().map_err(error)?;
        let status=response.status();
        let content=response.bytes().map_err(error)?;
        if content.len()>128*1024 {return Err("Supabase response too large".into())}
        let data:Value=serde_json::from_slice(&content).map_err(error)?;
        if !status.is_success(){
            return Err(data.get("error_description").or_else(||data.get("msg")).or_else(||data.get("message"))
                .and_then(Value::as_str).unwrap_or("Authentication request failed").to_owned());
        }
        Ok(data)
    }
    fn decode_session(&self,data:Value)->Result<Session,String>{
        let access=data.get("access_token").and_then(Value::as_str).ok_or("Missing access token; check email confirmation")?;
        let refresh=data.get("refresh_token").and_then(Value::as_str).ok_or("Missing refresh token")?;
        let expires=data.get("expires_in").and_then(Value::as_u64).unwrap_or(3600).max(1);
        let email=data.pointer("/user/email").and_then(Value::as_str).map(str::to_owned);
        Ok(Session{access_token:access.into(),refresh_token:refresh.into(),expires_at:now().saturating_add(expires),email})
    }
    fn save(&self,s:&Session)->Result<(),String>{
        let data=serde_json::to_string(s).map_err(error)?;
        credentials::store(SERVICE_KEY,&data).map_err(error)
    }
    fn load(&self)->Option<Session>{
        credentials::retrieve(SERVICE_KEY).ok().and_then(|data|serde_json::from_str(&data).ok())
    }
    fn fresh(&self)->Result<Option<Session>,String>{
        let Some(mut session)=self.load() else {return Ok(None)};
        if session.expires_at <= now().saturating_add(60){
            let data=self.json(self.request("/auth/v1/token?grant_type=refresh_token")?
                .json(&json!({"refresh_token":session.refresh_token})));
            session=match data.and_then(|d|self.decode_session(d)){
                Ok(session)=>session,
                Err(_)=>{let _=credentials::remove(SERVICE_KEY);return Ok(None)}
            };
            self.save(&session)?;
        }
        Ok(Some(session))
    }
    pub fn status(&self)->AccountStatus{
        let base=AccountStatus{configured:true,authenticated:false,email:None,
            persistent:true,message:None};
        let session=match self.fresh(){Ok(Some(s))=>s,Ok(None)=>return base,Err(e)=>return AccountStatus{message:Some(e),..base}};
        let url=match self.url.join("/auth/v1/user"){Ok(u)=>u,Err(_)=>return base};
        let response=self.client.get(url).header("apikey",&self.publishable)
            .bearer_auth(&session.access_token).send();
        let Ok(response)=response else{return AccountStatus{message:Some("Identity provider unavailable".into()),..base}};
        if !response.status().is_success(){
            let _=credentials::remove(SERVICE_KEY);return base
        }
        let email=response.json::<Value>().ok().and_then(|v|v.get("email").and_then(Value::as_str).map(str::to_owned)).or(session.email);
        AccountStatus{authenticated:true,email,..base}
    }
    pub fn sign_in(&self,email:&str,password:&str)->Result<AccountStatus,String>{
        if !email.contains('@')||password.is_empty(){return Err("Email and password required".into())}
        let data=self.json(self.request("/auth/v1/token?grant_type=password")?
            .json(&json!({"email":email.trim(),"password":password})))?;
        self.save(&self.decode_session(data)?)?;
        Ok(self.status())
    }
    pub fn sign_up(&self,email:&str,password:&str)->Result<AccountStatus,String>{
        if !email.contains('@')||password.len()<8 {return Err("A valid email and password of 8+ characters required".into())}
        let data=self.json(self.request("/auth/v1/signup")?.json(&json!({"email":email.trim(),"password":password})))?;
        if data.get("access_token").is_some(){self.save(&self.decode_session(data)?)?;return Ok(self.status())}
        Ok(AccountStatus{configured:true,authenticated:false,email:None,persistent:true,
            message:Some("Check your email for confirmation".into())})
    }
    pub fn sign_out(&self)->Result<AccountStatus,String>{
        let old=self.load();
        let _=credentials::remove(SERVICE_KEY);
        if let Some(old)=old{
            let _=self.json(self.request("/auth/v1/logout")?.bearer_auth(old.access_token));
        }
        Ok(AccountStatus{configured:true,authenticated:false,email:None,persistent:true,message:None})
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn default_status_serializes(){
        let s=AccountStatus{configured:false,authenticated:false,email:None,persistent:false,message:None};
        assert_eq!(serde_json::to_value(s).unwrap()["authenticated"],false);
    }
}
