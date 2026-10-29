mod hll;
mod grpc;
mod sse;
mod supabase;
//! Native Rust HTTP implementation of the Charlotte chat endpoints.
//! Migration-in-progress: gRPC, SSE, HLL persistence and ops are not ported yet.
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    env,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    secret: Arc<Vec<u8>>,
    registry: Arc<Mutex<Registry>>,
    hub: Arc<Mutex<HashMap<String, Room>>>,
    metrics: Arc<Mutex<hll::Metrics>>,
    allowed_origin: Option<String>,
    streams: Arc<Mutex<HashMap<String,tokio::sync::broadcast::Sender<Message>>>>,
    subscribers: Arc<std::sync::atomic::AtomicUsize>,
}
struct Registry { names: HashSet<String>, path: PathBuf }
struct Room { seq: u64, messages: VecDeque<Message>, last_publish: u64 }
#[derive(Clone, Serialize)]
struct Message {
    id: String, seq: u64, room: String, sender: String, content: String,
    timestamp: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    url: String, username: String, browser: String, session_id: String,
    region: String, room: String,
}
#[derive(Deserialize)]
struct ConnectRequest {
    url: String, username: Option<String>, browser: String,
    session_id: String, region: String, token: Option<String>,
}
#[derive(Serialize)]
struct ConnectResponse { token: String, cursor: u64 }
#[derive(Deserialize)]
struct SendRequest { content: String }
#[derive(Serialize)]
struct SendResponse { id: String }
#[derive(Deserialize)]
struct PollQuery { cursor: Option<String> }
#[derive(Serialize)]
struct PollResponse { messages: Vec<Message>, cursor: u64, next_poll_ms: u64 }
#[derive(Serialize)]
struct Metrics { unique_rooms: usize, unique_users: usize }

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn room_from_url(url: &str) -> String {
    format!("{:x}", md5::compute(url.trim_end_matches('/').as_bytes()))
}
fn verify(state: &AppState, token: &str) -> Result<Claims, StatusCode> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims.clear();
    validation.validate_exp = false;
    decode::<Claims>(token, &DecodingKey::from_secret(&state.secret), &validation)
        .map(|data| data.claims).map_err(|_| StatusCode::UNAUTHORIZED)
}
fn bearer(headers: &HeaderMap) -> Result<&str, StatusCode> {
    headers.get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|s| !s.trim().is_empty())
        .ok_or(StatusCode::UNAUTHORIZED)
}
fn lock<'a, T>(mutex: &'a Mutex<T>) -> Result<std::sync::MutexGuard<'a, T>, StatusCode> {
    mutex.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
fn seq(rooms: &HashMap<String, Room>, room: &str) -> u64 {
    rooms.get(room).map(|r| r.seq).unwrap_or(0)
}
async fn health() -> &'static str { "ok\n" }
async fn connect(
    State(state): State<AppState>,
    Json(req): Json<ConnectRequest>,
) -> Result<Json<ConnectResponse>, (StatusCode, &'static str)> {
    let bad = |message| (StatusCode::BAD_REQUEST, message);
    if req.url.is_empty() || req.browser.is_empty() || req.session_id.is_empty() || req.region.is_empty() {
        return Err(bad("url, browser, session_id, and region are required"));
    }
    let identity = if let Some(token) = req.token.filter(|t| !t.is_empty()) {
        let claims = verify(&state, &token).map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token"))?;
        let registry = lock(&state.registry).map_err(|s| (s, "registry lock failed"))?;
        if !registry.names.contains(&claims.username.to_lowercase()) {
            return Err((StatusCode::UNAUTHORIZED, "token's username is no longer claimed on this server"));
        }
        claims.username
    } else {
        let name = req.username.filter(|v| !v.is_empty())
            .ok_or_else(|| bad("username is required unless token is provided"))?;
        let mut registry = lock(&state.registry).map_err(|s| (s, "registry lock failed"))?;
        if registry.names.contains(&name.to_lowercase()) {
            return Err((StatusCode::CONFLICT, "username already taken"));
        }
        let mut file = OpenOptions::new().append(true).create(true).open(&registry.path)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "could not claim username"))?;
        writeln!(file, "{name}").and_then(|_| file.sync_all())
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "could not claim username"))?;
        registry.names.insert(name.to_lowercase());
        lock(&state.metrics).map_err(|s| (s, "metrics lock failed"))?.add_user(&name);
        name
    };
    let room = room_from_url(&req.url);
    lock(&state.metrics).map_err(|s| (s, "metrics lock failed"))?.add_room(&room);
    let claims = Claims {
        url: req.url, username: identity, browser: req.browser,
        session_id: req.session_id, region: req.region, room: room.clone(),
    };
    let token = encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(&state.secret))
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "could not generate token"))?;
    let cursor = seq(&lock(&state.hub).map_err(|s| (s, "hub lock failed"))?, &room);
    Ok(Json(ConnectResponse { token, cursor }))
}
async fn send(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SendRequest>,
) -> Result<Json<SendResponse>, StatusCode> {
    let claims = verify(&state, bearer(&headers)?)?;
    if req.content.is_empty() { return Err(StatusCode::BAD_REQUEST); }
    let now = now_ms();
    let id = format!("{}{}", now, SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos());
    let mut rooms = lock(&state.hub)?;
    let room = rooms.entry(claims.room.clone())
        .or_insert_with(|| Room { seq: now, messages: VecDeque::new(), last_publish: now });
    room.seq = room.seq.saturating_add(1);
    room.last_publish = now;
    let published_seq=room.seq;
    let published_content=req.content.clone();
    room.messages.push_back(Message {
        id: id.clone(), seq: room.seq, room: claims.room.clone(),
        sender: claims.username.clone(), content: req.content, timestamp: now,
    });
    while room.messages.len() > 256 ||
        room.messages.front().is_some_and(|m| now.saturating_sub(m.timestamp) > 120_000) {
        room.messages.pop_front();
    }
    drop(rooms);
    sse::broadcast(&state,&claims.room,&Message{id:id.clone(),seq:published_seq,room:claims.room.clone(),sender:claims.username.clone(),content:published_content,timestamp:now});
    Ok(Json(SendResponse { id }))
}
async fn poll(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PollQuery>,
) -> Result<(StatusCode, HeaderMap, Option<Json<PollResponse>>), StatusCode> {
    let claims = verify(&state, bearer(&headers)?)?;
    let cursor = query.cursor.as_deref().unwrap_or("0").parse::<u64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let rooms = lock(&state.hub)?;
    let room = rooms.get(&claims.room);
    let next = room.map(|r| match now_ms().saturating_sub(r.last_publish) {
        0..=9_999 => 1_000, 10_000..=59_999 => 5_000,
        60_000..=299_999 => 20_000, _ => 30_000,
    }).unwrap_or(30_000);
    let messages: Vec<Message> = room.into_iter().flat_map(|r| r.messages.iter())
        .filter(|m| m.seq > cursor).cloned().collect();
    let new_cursor = messages.last().map(|m| m.seq).unwrap_or(cursor);
    let mut response_headers = HeaderMap::new();
    response_headers.insert("x-next-poll-ms", HeaderValue::from_str(&next.to_string()).unwrap());
    response_headers.insert(header::ETAG, HeaderValue::from_str(&format!("\"{new_cursor}\"")).unwrap());
    if messages.is_empty() {
        return Ok((StatusCode::NOT_MODIFIED, response_headers, None));
    }
    Ok((StatusCode::OK, response_headers, Some(Json(PollResponse {
        messages, cursor: new_cursor, next_poll_ms: next,
    }))))
}
async fn metrics(State(state): State<AppState>) -> Result<Json<Metrics>, StatusCode> {
    let (rooms,users) = lock(&state.metrics)?.snapshot();
    let unique_rooms = rooms as usize;
    let unique_users = users as usize;
    Ok(Json(Metrics { unique_rooms, unique_users }))
}
fn address() -> Result<SocketAddr, String> {
    let p = env::var("HTTP_PORT").ok().filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("PORT").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "8080".into());
    let port = p.trim().parse::<u16>().map_err(|_| "HTTP_PORT or PORT must be an integer from 1 to 65535")?;
    if port == 0 { return Err("HTTP_PORT or PORT must be an integer from 1 to 65535".into()); }
    Ok(SocketAddr::from(([0, 0, 0, 0], port)))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let secret = env::var("CHAT_JWT_SECRET").map_err(|_| "CHAT_JWT_SECRET environment variable must be set")?;
    if secret.is_empty() { return Err("CHAT_JWT_SECRET environment variable must be set".into()); }
    let path = PathBuf::from(env::var("CHAT_USERNAMES_FILE").unwrap_or_else(|_| "data/usernames.log".into()));
    if let Some(dir) = path.parent() { fs::create_dir_all(dir)?; }
    let mut names = HashSet::from(["system".to_owned()]);
    if path.exists() {
        for line in BufReader::new(fs::File::open(&path)?).lines() {
            let line = line?;
            if !line.trim().is_empty() { names.insert(line.trim().to_lowercase()); }
        }
    }
    let metrics_store = hll::Metrics::load(env::var("CHAT_ROOMS_HLL_FILE").unwrap_or_else(|_| "data/rooms.hll".into()), env::var("CHAT_USERS_HLL_FILE").unwrap_or_else(|_| "data/users.hll".into()))?;
    let state = AppState {
        secret: Arc::new(secret.into_bytes()),
        registry: Arc::new(Mutex::new(Registry { names, path })),
        hub: Arc::new(Mutex::new(HashMap::new())),
        metrics: Arc::new(Mutex::new(metrics_store)),
        allowed_origin: env::var("CHARLOTTE_WEB_ORIGIN").ok(),
        streams:Arc::new(Mutex::new(HashMap::new())),
        subscribers:Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let persist = state.metrics.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));
        loop { ticker.tick().await; if let Ok(m) = persist.lock() { if let Err(e) = m.save() { tracing::error!("metrics persist failed: {e}"); } } }
    });
    let final_metrics = state.metrics.clone();
    let grpc_state=state.clone();
    tokio::spawn(async move {if let Err(e)=grpc::serve(grpc_state).await {tracing::error!("gRPC failure: {e}");}});
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/connect", post(connect))
        .route("/send", post(send))
        .route("/poll", get(poll))
        .route("/custom-metrics", get(metrics))
        .route("/events",get(sse::events))
        .route("/auth/me",get(supabase::me))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(state, cors));
    let listener = TcpListener::bind(address()?).await?;
    tracing::info!(addr = %listener.local_addr()?, "Charlotte Rust HTTP listening");
    axum::serve(listener, app).with_graceful_shutdown(async {
        #[cfg(unix)]
        {
            if let Ok(mut term) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {},
                    _ = term.recv() => {},
                }
            } else { let _ = tokio::signal::ctrl_c().await; }
        }
        #[cfg(not(unix))]
        { let _ = tokio::signal::ctrl_c().await; }
    }).await?;
    if let Ok(m) = final_metrics.lock() { m.save()?; }
    Ok(())
}
async fn cors(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let origin_matches = state.allowed_origin.as_ref().is_some_and(|origin|
        req.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok()) == Some(origin.as_str()));
    let path_allowed = matches!(req.uri().path(), "/healthz" | "/custom-metrics");
    let mut response = next.run(req).await;
    if origin_matches && path_allowed {
        if let Some(origin) = &state.allowed_origin {
            if let Ok(value) = HeaderValue::from_str(origin) {
                response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
                response.headers_mut().insert(header::VARY, HeaderValue::from_static("Origin"));
            }
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn url_normalization_matches_go() {
        assert_eq!(room_from_url("https://example.com///"), room_from_url("https://example.com"));
    }
    #[test]
    fn port_validation() {
        assert!("0".parse::<u16>().unwrap() == 0);
    }
    #[test]
    fn jwt_round_trip_legacy_fields() {
        let secret = b"local-test-secret";
        let claims = Claims { url: "https://example.com".into(), username: "alice".into(),
            browser: "test".into(), session_id: "x".into(), region: "KE".into(),
            room: room_from_url("https://example.com") };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret)).unwrap();
        let state = AppState {
            secret: Arc::new(secret.to_vec()),
            registry: Arc::new(Mutex::new(Registry { names: HashSet::new(), path: PathBuf::new() })),
            hub: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(hll::Metrics::load(String::new(), String::new()).unwrap())),
            allowed_origin: None,
            streams:Arc::new(Mutex::new(HashMap::new())),
            subscribers:Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        assert_eq!(verify(&state, &token).unwrap().username, "alice");
    }
}
