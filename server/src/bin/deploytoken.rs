use std::{collections::HashMap,env};
use serde_json::json;
use jsonwebtoken::{encode,EncodingKey,Header};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let mut args=HashMap::new();
    let mut it=env::args().skip(1);
    while let Some(key)=it.next(){let value=it.next().ok_or("missing flag value")?;args.insert(key.trim_start_matches('-').to_string(),value);}
    let version=args.get("version").ok_or("required: -version <version>")?;
    let linux=args.get("linux").ok_or("required: -linux <download-url>")?;
    let secret=env::var("OPS_JWT_SECRET")?;
    if secret.is_empty(){return Err("OPS_JWT_SECRET is empty".into());}
    let source=args.get("source").map(String::as_str).unwrap_or("github");
    let claims=json!({"source":source,"buildVersion":version,"builds":{"linux":linux}});
    let jwt=encode(&Header::default(),&claims,&EncodingKey::from_secret(secret.as_bytes()))?;
    print!("{jwt}");
    Ok(())
}
