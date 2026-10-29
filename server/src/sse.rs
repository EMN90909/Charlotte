use super::{AppState,Message,PollQuery,bearer,verify};
use axum::{extract::{State,Query},http::{HeaderMap,StatusCode},response::sse::{Sse,Event,KeepAlive}};
use std::{sync::{Arc,atomic::{AtomicUsize,Ordering}},time::Duration,convert::Infallible};
use tokio::sync::broadcast;
pub fn broadcast(state:&AppState,room:&str,msg:&Message){
 if let Ok(streams)=state.streams.lock(){if let Some(tx)=streams.get(room){let _=tx.send(msg.clone());}}
}
struct Guard(Arc<AtomicUsize>);
impl Drop for Guard{fn drop(&mut self){self.0.fetch_sub(1,Ordering::SeqCst);}}
pub async fn events(State(state):State<AppState>,headers:HeaderMap,Query(query):Query<PollQuery>)->Result<Sse<impl futures_util::Stream<Item=Result<Event,Infallible>>>,StatusCode>{
 let claims=verify(&state,bearer(&headers)?)?;
 let cursor=query.cursor.as_deref().unwrap_or("0").parse::<u64>().map_err(|_|StatusCode::BAD_REQUEST)?;
 if state.subscribers.fetch_add(1,Ordering::SeqCst)>=2000{state.subscribers.fetch_sub(1,Ordering::SeqCst);return Err(StatusCode::SERVICE_UNAVAILABLE);}
 let count=state.subscribers.clone();
 let rx=match state.streams.lock(){Ok(mut m)=>m.entry(claims.room.clone()).or_insert_with(||broadcast::channel(32).0).subscribe(),Err(_)=>{count.fetch_sub(1,Ordering::SeqCst);return Err(StatusCode::INTERNAL_SERVER_ERROR)}};
 let backlog:Vec<Message>=match state.hub.lock(){Ok(m)=>m.get(&claims.room).into_iter().flat_map(|r|r.messages.iter()).filter(|m|m.seq>cursor).cloned().collect(),Err(_)=>{count.fetch_sub(1,Ordering::SeqCst);return Err(StatusCode::INTERNAL_SERVER_ERROR)}};
 let last=backlog.last().map(|m|m.seq).unwrap_or(cursor);
 let stream=async_stream::stream!{
 let _guard=Guard(count);
 for msg in backlog{if let Ok(body)=serde_json::to_string(&msg){yield Ok::<Event,Infallible>(Event::default().data(body));}}
 let mut receiver=rx;let mut last_seq=last;
 loop{match receiver.recv().await{
 Ok(msg) if msg.seq>last_seq=>{last_seq=msg.seq;if let Ok(body)=serde_json::to_string(&msg){yield Ok(Event::default().data(body));}}
 Ok(_)|Err(broadcast::error::RecvError::Lagged(_))=>{}
 Err(broadcast::error::RecvError::Closed)=>break
 }}
 };
 Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(25)).text("heartbeat")))
}