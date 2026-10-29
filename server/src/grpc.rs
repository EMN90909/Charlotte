use super::{AppState,Message,now_ms,lock,verify,Room};
use std::collections::VecDeque;
use tonic::{Request,Response,Status};
pub mod proto {tonic::include_proto!("chat");}
use proto::chat_server::{Chat,ChatServer};
struct ChatService(AppState);
#[tonic::async_trait]
impl Chat for ChatService {
 async fn send_message(&self,request:Request<proto::SendMessageRequest>)->Result<Response<proto::SendMessageResponse>,Status>{
 let req=request.into_inner();
 let claims=verify(&self.0,&req.token).map_err(|_|Status::unauthenticated("invalid token"))?;
 if req.content.is_empty(){return Err(Status::invalid_argument("content is required"));}
 let now=now_ms();
 let id=format!("{}-{}",now,std::process::id());
 let mut rooms=lock(&self.0.hub).map_err(|_|Status::internal("hub lock failed"))?;
 let r=rooms.entry(claims.room.clone()).or_insert_with(||Room{seq:now,messages:VecDeque::new(),last_publish:now});
 r.seq+=1;r.last_publish=now;
 let msg=Message{id:id.clone(),seq:r.seq,room:claims.room.clone(),sender:claims.username,content:req.content,timestamp:now};
 r.messages.push_back(msg.clone());
 while r.messages.len()>256||r.messages.front().is_some_and(|m|now.saturating_sub(m.timestamp)>120000){r.messages.pop_front();}
 drop(rooms);
 crate::sse::broadcast(&self.0,&claims.room,&msg);
 Ok(Response::new(proto::SendMessageResponse{id}))
 }
}
pub async fn serve(state:AppState)->Result<(),Box<dyn std::error::Error+Send+Sync>>{
 let addr="0.0.0.0:50051".parse()?;
 tonic::transport::Server::builder().add_service(ChatServer::new(ChatService(state))).serve(addr).await?;
 Ok(())
}