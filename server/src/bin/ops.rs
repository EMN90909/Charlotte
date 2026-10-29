//! Native Rust deploy supervisor, compatible with Charlotte /deploy and /version.
use axum::{extract::State, http::{HeaderMap,StatusCode}, routing::{get,post}, Json, Router};
use jsonwebtoken::{decode,Algorithm,DecodingKey,Validation};
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};
use std::{collections::HashMap,env,path::{Path,PathBuf},sync::Arc,time::Duration};
use tokio::{net::TcpListener,process::{Child,Command},sync::Mutex};

#[derive(Serialize,Deserialize,Clone)]
struct VersionState {
    version:String,
    binary_path:String,
    source:String,
    deployed_at:String,
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct DeployClaims {
    source:String,
    build_version:String,
    builds:HashMap<String,String>,
}
struct Supervisor {
    data_dir:PathBuf,
    current:Option<Child>,
    state:Option<VersionState>,
    client:reqwest::Client,
    health_url:String,
    download_token:Option<String>,
}
#[derive(Clone)]
struct Shared { supervisor:Arc<Mutex<Supervisor>>, secret:Arc<Vec<u8>> }

fn err(msg:impl std::fmt::Display)->std::io::Error {std::io::Error::other(msg.to_string())}
impl Supervisor {
    async fn new()->Result<Self,Box<dyn std::error::Error+Send+Sync>>{
        let data_dir=PathBuf::from(env::var("OPS_DATA_DIR").unwrap_or_else(|_|"ops-data".into()));
        tokio::fs::create_dir_all(data_dir.join("bin")).await?;
        let state=match tokio::fs::read(data_dir.join("state.json")).await {
            Ok(bytes)=>Some(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>None,
            Err(e)=>return Err(e.into()),
        };
        Ok(Self{
            data_dir,current:None,state,
            client:reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?,
            health_url:env::var("OPS_APP_HEALTH_URL").unwrap_or_else(|_|"http://localhost:8080/healthz".into()),
            download_token:env::var("OPS_DOWNLOAD_TOKEN").ok().filter(|s|!s.is_empty()),
        })
    }
    fn spawn(path:&Path)->std::io::Result<Child>{
        let mut cmd=Command::new(path);
        cmd.kill_on_drop(true).stdin(std::process::Stdio::null());
        cmd.spawn()
    }
    async fn healthy(&self)->bool{
        for _ in 0..3 {
            if let Ok(response)=self.client.get(&self.health_url).send().await {
                if response.status().is_success(){return true;}
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        false
    }
    async fn terminate_current(&mut self){
        if let Some(mut child)=self.current.take(){
            #[cfg(unix)]
            if let Some(pid)=child.id(){
                let _=nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32),nix::sys::signal::Signal::SIGTERM);
            }
            if tokio::time::timeout(Duration::from_secs(15),child.wait()).await.is_err(){
                let _=child.kill().await;
            }
        }
    }
    async fn launch_and_check(&self,path:&Path)->Result<Child,String>{
        let mut process=Self::spawn(path).map_err(|e|e.to_string())?;
        tokio::time::sleep(Duration::from_secs(10)).await;
        if !self.healthy(){let _=process.kill().await;return Err("new binary never became healthy".into());}
        Ok(process)
    }
    async fn resume_or_bootstrap(&mut self){
        if let Some(record)=self.state.clone(){
            match self.launch_and_check(Path::new(&record.binary_path)).await {
                Ok(child)=>{self.current=Some(child);tracing::info!(version=%record.version,"resumed");}
                Err(e)=>tracing::error!("resume failed: {e}"),
            }
        } else if let (Ok(src),Ok(version))=(env::var("OPS_INITIAL_BINARY"),env::var("OPS_INITIAL_VERSION")){
            let dest=self.data_dir.join("bin").join(format!("chat-service-server-{version}"));
            if tokio::fs::copy(src,&dest).await.is_ok(){
                match self.launch_and_check(&dest).await{
                    Ok(child)=>{
                        self.current=Some(child);
                        let state=VersionState{version,source:"bootstrap".into(),binary_path:dest.display().to_string(),deployed_at:timestamp()};
                        if self.persist(&state).await.is_ok(){self.state=Some(state);}
                    }
                    Err(e)=>tracing::error!("bootstrap failed: {e}"),
                }
            }
        }
    }
    async fn persist(&self,state:&VersionState)->std::io::Result<()>{
        let path=self.data_dir.join("state.json");
        let temp=self.data_dir.join("state.json.tmp");
        tokio::fs::write(&temp,serde_json::to_vec_pretty(state).map_err(err)?).await?;
        tokio::fs::rename(temp,path).await
    }
    async fn deploy(&mut self,claims:DeployClaims)->Result<String,(String,bool)>{
        let url=claims.builds.get("linux").ok_or(("no linux build URL in deploy request".into(),false))?;
        if claims.build_version.is_empty(){return Err(("buildVersion is required".into(),false));}
        // Forbid traversal in the version-derived filename.
        if !claims.build_version.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'.'|b'-'|b'_')){
            return Err(("invalid buildVersion".into(),false));
        }
        let dest=self.data_dir.join("bin").join(format!("chat-service-server-{}",claims.build_version));
        if self.state.as_ref().is_some_and(|s|s.binary_path==dest.to_string_lossy()){return Err(("this version is already deployed".into(),false));}
        let mut request=self.client.get(url);
        if let Some(token)=&self.download_token {request=request.bearer_auth(token);}
        let response=request.send().await.map_err(|e|(format!("download: {e}"),false))?;
        if !response.status().is_success(){return Err((format!("download: HTTP {}",response.status()),false));}
        if response.content_length().is_some_and(|n|n>200_000_000){return Err(("binary too large".into(),false));}
        let bytes=response.bytes().await.map_err(|e|(format!("download: {e}"),false))?;
        if bytes.len()>200_000_000 {return Err(("binary too large".into(),false));}
        tokio::fs::write(&dest,&bytes).await.map_err(|e|(format!("stage binary: {e}"),false))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e)=tokio::fs::set_permissions(&dest,std::fs::Permissions::from_mode(0o755)).await{
                let _=tokio::fs::remove_file(&dest).await;return Err((format!("permissions: {e}"),false));
            }
        }
        let previous=self.state.clone();
        self.terminate_current().await;
        match self.launch_and_check(&dest).await{
            Ok(child)=>{
                self.current=Some(child);
                let record=VersionState{
                    version:claims.build_version.clone(),binary_path:dest.display().to_string(),
                    source:claims.source,deployed_at:timestamp()
                };
                if let Err(e)=self.persist(&record).await{tracing::error!("persist deployment state: {e}");}
                self.state=Some(record);
                if let Some(prev)=previous {if prev.binary_path!=dest.to_string_lossy(){let _=tokio::fs::remove_file(prev.binary_path).await;}}
                Ok(claims.build_version)
            }
            Err(reason)=>{
                let _=tokio::fs::remove_file(dest).await;
                let mut rolled_back=false;
                if let Some(prev)=previous {
                    if let Ok(child)=self.launch_and_check(Path::new(&prev.binary_path)).await {
                        self.current=Some(child);rolled_back=true;
                    }
                }
                Err((reason,rolled_back))
            }
        }
    }
}
fn timestamp()->String{
    chrono::Utc::now().to_rfc3339()
}
async fn version(State(shared):State<Shared>)->Json<Value>{
    let guard=shared.supervisor.lock().await;
    match &guard.state{
        Some(s)=>Json(json!({"version":s.version,"running":guard.current.is_some(),"source":s.source,"deployed_at":s.deployed_at})),
        None=>Json(json!({"version":null,"running":false}))
    }
}
async fn deploy(State(shared):State<Shared>,headers:HeaderMap)->(StatusCode,Json<Value>){
    let token=headers.get("authorization").and_then(|v|v.to_str().ok())
        .and_then(|v|v.strip_prefix("Bearer ")).filter(|t|!t.trim().is_empty());
    let Some(token)=token else{return (StatusCode::UNAUTHORIZED,Json(json!({"error":"Bearer token required"})));};
    let mut validation=Validation::new(Algorithm::HS256);
    validation.required_spec_claims.clear();
    let Ok(claims)=decode::<DeployClaims>(token,&DecodingKey::from_secret(&shared.secret),&validation)
        else{return (StatusCode::UNAUTHORIZED,Json(json!({"error":"invalid token"})));};
    let supervisor=shared.supervisor.clone();
    match tokio::time::timeout(Duration::from_secs(90),async move {supervisor.lock().await.deploy(claims.claims).await}).await {
        Ok(Ok(version))=>(StatusCode::OK,Json(json!({"status":"ok","version":version}))),
        Ok(Err((message,rolled_back)))=>(StatusCode::BAD_GATEWAY,Json(json!({"status":"failed","error":message,"rolled_back":rolled_back}))),
        Err(_)=>(StatusCode::GATEWAY_TIMEOUT,Json(json!({"status":"failed","error":"deployment timed out"})))
    }
}
#[tokio::main]
async fn main()->Result<(),Box<dyn std::error::Error+Send+Sync>>{
    tracing_subscriber::fmt::init();
    let secret=env::var("OPS_JWT_SECRET").map_err(|_|err("OPS_JWT_SECRET is required"))?;
    if secret.is_empty(){return Err(err("OPS_JWT_SECRET is required").into());}
    let mut supervisor=Supervisor::new().await?;
    supervisor.resume_or_bootstrap().await;
    let shared=Shared{supervisor:Arc::new(Mutex::new(supervisor)),secret:Arc::new(secret.into_bytes())};
    let app=Router::new().route("/version",get(version)).route("/deploy",post(deploy)).with_state(shared);
    let addr=env::var("OPS_ADDR").unwrap_or_else(|_|"0.0.0.0:9090".into());
    let addr=if let Some(port)=addr.strip_prefix(':'){format!("0.0.0.0:{port}")}else{addr};
    let socket=TcpListener::bind(addr).await?;
    axum::serve(socket,app).await?;
    Ok(())
}
