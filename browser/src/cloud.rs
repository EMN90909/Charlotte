//! Charlotte cloud AI requests. The app passes only a fresh Supabase user
//! access token; cloud AI vendor keys remain server-side in services/.env.
use crate::services::{ServiceError,Turn};
use serde_json::{json,Value};
use reqwest::{blocking::Client,redirect::Policy};
use std::time::Duration;
use url::Url;

pub fn validate_cloud_url(raw:&str)->Result<Url,ServiceError>{
    let parsed=Url::parse(raw).map_err(|_|ServiceError::InvalidEndpoint)?;
    let host=parsed.host_str().ok_or(ServiceError::InvalidEndpoint)?;
    let local=matches!(host,"127.0.0.1"|"localhost"|"::1");
    if (parsed.scheme()!="https" && !(local&&parsed.scheme()=="http")) ||
        !parsed.username().is_empty()||parsed.password().is_some()||
        parsed.query().is_some()||parsed.fragment().is_some() ||
        parsed.path()!="/" || parsed.port()==Some(0) {
        return Err(ServiceError::InvalidEndpoint)
    }
    Ok(parsed)
}
pub fn saved_endpoint()->Result<Url,ServiceError>{
    let db=crate::storage::Storage::open(
        &crate::profile::profile_root().join("charlotte.sqlite"))
        .map_err(|_|ServiceError::InvalidEndpoint)?;
    let endpoint=db.setting("ai:cloud_url")
        .map_err(|_|ServiceError::InvalidEndpoint)?
        .ok_or(ServiceError::InvalidEndpoint)?;
    validate_cloud_url(&endpoint)
}
pub fn send(model:&str,system:&str,history:&[Turn],text:&str,image:Option<&str>)
    ->Result<String,ServiceError>{
    if text.len()>12_000||system.len()>8_000||history.len()>16{
        return Err(ServiceError::InvalidResponse)
    }
    if image.is_some_and(|v|v.len()>1_600_000){return Err(ServiceError::InvalidResponse)}
    let base=saved_endpoint()?;
    let target=base.join("v1/ai/respond").map_err(|_|ServiceError::InvalidEndpoint)?;
    let auth=crate::auth::SupabaseAuth::configured()
        .map_err(|_|ServiceError::MissingCredential)?;
    let token=auth.access_token().map_err(|_|ServiceError::MissingCredential)?;
    let client=Client::builder().timeout(Duration::from_secs(60))
        .redirect(Policy::none()).build()?;
    let limited:Vec<Value>=history.iter().rev().take(12).rev().map(|h|
        json!({"role":h.role,"text":h.text.chars().take(8000).collect::<String>()})
    ).collect();
    let body=json!({
        "model":model,"text":text,"system":system,"history":limited,
        "image_jpeg_base64":image
    });
    let response=client.post(target).bearer_auth(token).json(&body).send()?;
    if !response.status().is_success(){
        let status=response.status().as_u16();
        return Err(ServiceError::Server{status,retry_after:None})
    }
    if response.content_length().is_some_and(|len|len>200_000){
        return Err(ServiceError::InvalidResponse)
    }
    let bytes=response.bytes()?;
    if bytes.len()>200_000{return Err(ServiceError::InvalidResponse)}
    let result:Value=serde_json::from_slice(&bytes)
        .map_err(|_|ServiceError::InvalidResponse)?;
    result["text"].as_str().filter(|v|!v.is_empty()&&v.len()<=100_000)
        .map(str::to_owned).ok_or(ServiceError::InvalidResponse)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn cloud_endpoint_requires_https_or_loopback(){
        assert!(validate_cloud_url("https://ai.example.org/").is_ok());
        assert!(validate_cloud_url("http://localhost:8087").is_ok());
        assert!(validate_cloud_url("http://ai.example.org").is_err());
        assert!(validate_cloud_url("https://user:pass@ai.example.org").is_err());
        assert!(validate_cloud_url("https://ai.example.org/?token=abc").is_err());
        assert!(validate_cloud_url("https://ai.example.org/redirect").is_err());
    }
}
