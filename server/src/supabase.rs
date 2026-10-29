use super::AppState;
use axum::{extract::State,http::{HeaderMap,StatusCode,header,HeaderValue},response::{IntoResponse,Response},Json};
use serde_json::{Value,json};
pub async fn me(State(_state):State<AppState>,headers:HeaderMap)->Response {
 let result=check(headers).await;
 let mut res=result.into_response();
 res.headers_mut().insert(header::CACHE_CONTROL,HeaderValue::from_static("no-store"));
 res
}
async fn check(headers:HeaderMap)->Result<Json<Value>,(StatusCode,&'static str)>{
 let base=std::env::var("SUPABASE_URL").map_err(|_|(StatusCode::SERVICE_UNAVAILABLE,"Supabase Auth is not configured"))?;
 let key=std::env::var("SUPABASE_PUBLISHABLE_KEY").map_err(|_|(StatusCode::SERVICE_UNAVAILABLE,"Supabase Auth is not configured"))?;
 let parsed=reqwest::Url::parse(&base).map_err(|_|(StatusCode::SERVICE_UNAVAILABLE,"Supabase Auth is not configured"))?;
 let local=matches!(parsed.host_str(),Some("localhost"|"127.0.0.1"));
 if !(parsed.scheme()=="https"||(parsed.scheme()=="http"&&local))||!key.starts_with("sb_publishable_"){return Err((StatusCode::SERVICE_UNAVAILABLE,"Supabase Auth is not configured"))}
 let tok=headers.get(header::AUTHORIZATION).and_then(|h|h.to_str().ok()).and_then(|s|s.strip_prefix("Bearer ")).filter(|s|!s.trim().is_empty()).ok_or((StatusCode::UNAUTHORIZED,"Bearer access token required"))?;
 let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(7)).build().map_err(|_|(StatusCode::BAD_GATEWAY,"Identity provider unavailable"))?;
 let response=client.get(format!("{}/auth/v1/user",base.trim_end_matches('/'))).header("apikey",key).bearer_auth(tok).send().await.map_err(|_|(StatusCode::BAD_GATEWAY,"Identity provider unavailable"))?;
 if response.status()!=reqwest::StatusCode::OK{return Err((StatusCode::UNAUTHORIZED,"Invalid or expired access token"))}
 let bytes=response.bytes().await.map_err(|_|(StatusCode::BAD_GATEWAY,"Invalid identity response"))?;
 if bytes.len()>16384{return Err((StatusCode::BAD_GATEWAY,"Invalid identity response"))}
 let body:Value=serde_json::from_slice(&bytes).map_err(|_|(StatusCode::BAD_GATEWAY,"Invalid identity response"))?;
 let id=body.get("id").and_then(Value::as_str).filter(|s|!s.is_empty()).ok_or((StatusCode::BAD_GATEWAY,"Invalid identity response"))?;
 Ok(Json(json!({"id":id,"email":body.get("email").and_then(Value::as_str).unwrap_or("")})))
}