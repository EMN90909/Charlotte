//! Network services used by Charlotte. Credentials stay in the host process.
//! Opt-in browser screenshot vision requests are supported; general media uploads and streaming are not.
//! User-approved MCP execution lives in the native agent.rs and mcp.rs modules.
use reqwest::blocking::{Client, Response};
use reqwest::{redirect::Policy, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{time::Duration, io::{BufRead, BufReader}, sync::atomic::{AtomicBool, Ordering}};
use url::Url;

#[derive(Debug)]
pub enum ServiceError {
    InvalidEndpoint,
    MissingCredential,
    Http(reqwest::Error),
    Server { status: u16, retry_after: Option<u64> },
    InvalidResponse,
}
impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEndpoint => write!(f, "Invalid service endpoint"),
            Self::MissingCredential => write!(f, "No API key configured"),
            Self::Http(e) => write!(f, "Network error: {e}"),
            Self::Server { status, .. } => write!(f, "Service returned HTTP {status}"),
            Self::InvalidResponse => write!(f, "Service returned an unexpected response"),
        }
    }
}
impl std::error::Error for ServiceError {}
impl From<reqwest::Error> for ServiceError {
    fn from(e: reqwest::Error) -> Self { Self::Http(e) }
}

fn client() -> Result<Client, ServiceError> {
    Ok(Client::builder().timeout(Duration::from_secs(60)).redirect(Policy::none()).build()?)
}

/// Append a fixed API route without allowing a configured base to inject query,
/// credentials or a fragment. Avoid `Url::join` dropping a deployment prefix.
fn endpoint(base: &str, route: &str) -> Result<Url, ServiceError> {
    let mut url = Url::parse(base).map_err(|_| ServiceError::InvalidEndpoint)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
        || !url.username().is_empty() || url.password().is_some()
        || url.query().is_some() || url.fragment().is_some() {
        return Err(ServiceError::InvalidEndpoint);
    }
    // Credentials and chat messages may use cleartext only for loopback services.
    if url.scheme() == "http" && !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")) {
        return Err(ServiceError::InvalidEndpoint);
    }
    let path = format!("{}/{}", url.path().trim_end_matches('/'), route.trim_start_matches('/'));
    url.set_path(&path);
    Ok(url)
}

fn checked(response: Response) -> Result<Response, ServiceError> {
    if response.status().is_success() { return Ok(response); }
    let retry_after = response.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
    Err(ServiceError::Server { status: response.status().as_u16(), retry_after })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Turn { pub role: String, pub text: String }

#[derive(Clone, Copy, Debug)]
pub enum Provider { OpenAi, Claude, Gemini, Ollama }

/// Sends a plain text turn. The UI must execute this on a worker thread.
/// API keys must be obtained from the operating system credential store.
pub fn send_agent(provider: Provider, base: Option<&str>, key: Option<&str>, model: &str, system: &str, history: &[Turn], text: &str) -> Result<String, ServiceError> {
    let http = client()?;
    let messages: Vec<Value> = history.iter().map(|turn| json!({"role":turn.role,"content":turn.text})).chain(std::iter::once(json!({"role":"user","content":text}))).collect();
    let key = match provider { Provider::Ollama => key.unwrap_or(""), _ => key.filter(|k| !k.is_empty()).ok_or(ServiceError::MissingCredential)? };
    let (request, pointer) = match provider {
        Provider::OpenAi => {
            let mut all = Vec::new();
            if !system.is_empty() { all.push(json!({"role":"system","content":system})); }
            all.extend(messages);
            (http.post(endpoint(base.unwrap_or("https://api.openai.com"), "/v1/chat/completions")?).bearer_auth(key).json(&json!({"model":model,"messages":all,"stream":false})), "/choices/0/message/content")
        },
        Provider::Claude => (http.post(endpoint(base.unwrap_or("https://api.anthropic.com"), "/v1/messages")?).header("x-api-key", key).header("anthropic-version", "2023-06-01").json(&json!({"model":model,"max_tokens":4096,"system":system,"messages":messages,"stream":false})), "/content/0/text"),
        Provider::Gemini => {
            let mut url = endpoint(base.unwrap_or("https://generativelanguage.googleapis.com"), "/v1beta/models")?;
            url.path_segments_mut().map_err(|_| ServiceError::InvalidEndpoint)?.push(&format!("{model}:generateContent"));
            let contents: Vec<Value> = history.iter().map(|t| json!({"role":if t.role == "assistant" {"model"} else {"user"},"parts":[{"text":t.text}]})).chain(std::iter::once(json!({"role":"user","parts":[{"text":text}]}))).collect();
            (http.post(url).header("x-goog-api-key", key).json(&json!({"contents":contents,"systemInstruction":{"parts":[{"text":system}]}})), "/candidates/0/content/parts/0/text")
        },
        Provider::Ollama => {
            let mut all = Vec::new();
            if !system.is_empty() { all.push(json!({"role":"system","content":system})); }
            all.extend(messages);
            (http.post(endpoint(base.unwrap_or("http://localhost:11434"), "/api/chat")?).json(&json!({"model":model,"messages":all,"stream":false})), "/message/content")
        },
    };
    let body: Value = checked(request.send()?)?.json()?;
    body.pointer(pointer).and_then(Value::as_str).map(str::to_owned).ok_or(ServiceError::InvalidResponse)
}

/// Multimodal browser-page inspection, called only when the user explicitly
/// selects "Visual automation". JPEG bytes stay in memory and are not logged.
/// The image is sent to the chosen AI provider, with a 1.6 MB encoded cap.
pub fn send_visual_agent(
    provider:Provider,key:Option<&str>,model:&str,system:&str,
    history:&[Turn],text:&str,jpeg:&str
)->Result<String,ServiceError>{
    if jpeg.len()>1_600_000||jpeg.is_empty() {
        return Err(ServiceError::InvalidResponse)
    }
    let http=client()?;
    let key=match provider{
        Provider::Ollama=>key.unwrap_or(""),
        _=>key.filter(|v|!v.is_empty()).ok_or(ServiceError::MissingCredential)?,
    };
    let (request,pointer)=match provider{
        Provider::OpenAi=>{
            let mut messages=vec![json!({"role":"system","content":system})];
            messages.extend(history.iter().rev().take(10).rev().map(|turn|
                json!({"role":turn.role,"content":turn.text})));
            messages.push(json!({"role":"user","content":[
                {"type":"text","text":text},
                {"type":"image_url","image_url":{
                    "url":format!("data:image/jpeg;base64,{jpeg}"),"detail":"low"
                }}
            ]}));
            (http.post(endpoint("https://api.openai.com","/v1/chat/completions")?)
                .bearer_auth(key).json(&json!({"model":model,"messages":messages,
                "max_tokens":1200,"stream":false})), "/choices/0/message/content")
        },
        Provider::Claude=>{
            let mut messages:Vec<Value>=history.iter().rev().take(10).rev().map(|turn|
                json!({"role":turn.role,"content":turn.text})).collect();
            messages.push(json!({"role":"user","content":[
                {"type":"text","text":text},
                {"type":"image","source":{"type":"base64","media_type":"image/jpeg","data":jpeg}}
            ]}));
            (http.post(endpoint("https://api.anthropic.com","/v1/messages")?)
                .header("x-api-key",key).header("anthropic-version","2023-06-01")
                .json(&json!({"model":model,"max_tokens":1200,"system":system,
                    "messages":messages,"stream":false})), "/content/0/text")
        },
        Provider::Gemini=>{
            let contents=vec![json!({"role":"user","parts":[
                {"text":text},
                {"inline_data":{"mime_type":"image/jpeg","data":jpeg}}
            ]})];
            let mut url=endpoint("https://generativelanguage.googleapis.com","/v1beta/models")?;
            url.path_segments_mut().map_err(|_|ServiceError::InvalidEndpoint)?
                .push(&format!("{model}:generateContent"));
            (http.post(url).header("x-goog-api-key",key)
                .json(&json!({"systemInstruction":{"parts":[{"text":system}]},
                    "contents":contents})), "/candidates/0/content/parts/0/text")
        },
        Provider::Ollama=>{
            let messages=vec![
                json!({"role":"system","content":system}),
                json!({"role":"user","content":text,"images":[jpeg]})
            ];
            (http.post(endpoint("http://localhost:11434","/api/chat")?)
                .json(&json!({"model":model,"messages":messages,"stream":false})),
                "/message/content")
        },
    };
    let response:Value=checked(request.send()?)?.json()?;
    response.pointer(pointer).and_then(Value::as_str)
        .map(str::to_owned).ok_or(ServiceError::InvalidResponse)
}

#[derive(Debug, Serialize)]
pub struct ChatIdentity<'a> { pub url: &'a str, pub browser: &'a str, pub session_id: &'a str, pub region: &'a str, #[serde(skip_serializing_if = "Option::is_none")] pub username: Option<&'a str>, #[serde(skip_serializing_if = "Option::is_none")] pub token: Option<&'a str> }
#[derive(Debug, Deserialize)]
pub struct ChatConnection { pub token: String, pub cursor: u64 }
#[derive(Debug, Deserialize)]
pub struct ChatMessage { pub id: String, pub seq: u64, pub room: String, pub sender: String, pub content: String, pub timestamp: i64 }
#[derive(Debug, Deserialize)]
pub struct PollResult { pub messages: Vec<ChatMessage>, pub cursor: u64, #[serde(default)] pub next_poll_ms: u64 }

pub fn chat_connect(base: &str, identity: &ChatIdentity<'_>) -> Result<ChatConnection, ServiceError> {
    if identity.token.is_some() == identity.username.is_some() { return Err(ServiceError::InvalidResponse); }
    Ok(checked(client()?.post(endpoint(base, "/connect")?).json(identity).send()?)?.json()?)
}
pub fn chat_send(base: &str, token: &str, content: &str) -> Result<String, ServiceError> {
    let result: Value = checked(client()?.post(endpoint(base, "/send")?).bearer_auth(token).json(&json!({"content":content})).send()?)?.json()?;
    result.get("id").and_then(Value::as_str).map(str::to_owned).ok_or(ServiceError::InvalidResponse)
}
pub fn chat_poll(base: &str, token: &str, cursor: u64) -> Result<PollResult, ServiceError> {
    let mut url = endpoint(base, "/poll")?;
    url.query_pairs_mut().append_pair("cursor", &cursor.to_string());
    let response = client()?.get(url).bearer_auth(token).send()?;
    if response.status() == StatusCode::NOT_MODIFIED {
        let delay = response.headers().get("x-next-poll-ms").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(1000);
        return Ok(PollResult { messages: vec![], cursor, next_poll_ms: delay });
    }
    Ok(checked(response)?.json()?)
}


/// Continuously consumes Charlotte SSE events on a background thread.
/// This function is synchronous and MUST NOT be called on the CEF UI thread.
/// At most one buffered event is delivered per server data line, with cursor
/// deduplication for reconnecting clients. Closing the connection ends the call.
pub fn chat_events<F>(
    base: &str, token: &str, cursor: u64, stopped: &AtomicBool, mut receive: F,
) -> Result<u64, ServiceError>
where F: FnMut(ChatMessage) {
    let mut url = endpoint(base, "/events")?;
    url.query_pairs_mut().append_pair("cursor", &cursor.to_string());
    let response = checked(client()?.get(url).bearer_auth(token).send()?)?;
    let mut last = cursor;
    for line in BufReader::new(response).lines() {
        if stopped.load(Ordering::Relaxed) { break; }
        let line = line.map_err(|_| ServiceError::InvalidResponse)?;
        let Some(payload) = line.strip_prefix("data: ") else { continue };
        if let Ok(message) = serde_json::from_str::<ChatMessage>(payload) {
            if message.seq > last {
                last = message.seq;
                receive(message);
            }
        }
    }
    Ok(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoints_preserve_prefix_and_reject_credentials() {
        assert_eq!(endpoint("https://example.com/service/", "/poll").unwrap().as_str(), "https://example.com/service/poll");
        assert!(endpoint("https://user:password@example.com", "/send").is_err());
        assert!(endpoint("file:///tmp/a", "/send").is_err());
        assert!(endpoint("http://example.com", "/send").is_err());
        assert!(endpoint("https://example.com?redirect=bad", "/send").is_err());
    }
}
