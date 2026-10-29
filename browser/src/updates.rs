//! Opt-in Charlotte release checks. The hosted service distributes
//! metadata; only a trusted compile-time Ed25519 public key can authorize an
//! update. Downloaded assets are staged and SHA-256 verified, NEVER executed.
use std::{fs::{self,OpenOptions},io::{Read,Write},time::Duration};
use sha2::{Digest,Sha256};
use ed25519_dalek::{Signature,Verifier,VerifyingKey};
use base64::{engine::general_purpose::STANDARD,Engine as _};
use serde::Deserialize;
use reqwest::{blocking::Client,redirect::Policy};
use url::Url;
#[derive(Debug,Clone,Deserialize)]
pub struct Manifest {
    pub available:bool,
    pub version:Option<String>,pub channel:Option<String>,
    pub platform:Option<String>,pub arch:Option<String>,
    pub url:Option<String>,pub sha256:Option<String>,
    pub size:Option<u64>,pub signature:Option<String>,
    pub algorithm:Option<String>,
}
fn public_key()->Result<VerifyingKey,String>{
    // Embed release public key during the trusted, signed app build.
    // Never download the verification key from the same update server.
    let hex=option_env!("CHARLOTTE_RELEASE_PUBKEY_HEX")
        .ok_or("Release verification key is not embedded in this build; updates disabled")?;
    if hex.len()!=64||!hex.bytes().all(|c|c.is_ascii_hexdigit()){
        return Err("Invalid embedded Ed25519 key".into())
    }
    let mut bytes=[0u8;32];
    for i in 0..32{
        bytes[i]=u8::from_str_radix(&hex[i*2..i*2+2],16)
            .map_err(|_|"Invalid release key hex")?;
    }
    VerifyingKey::from_bytes(&bytes).map_err(|e|e.to_string())
}
fn client()->Result<Client,String>{
    Client::builder().timeout(Duration::from_secs(35))
        .redirect(Policy::none()).build().map_err(|e|e.to_string())
}
fn location(raw:&str)->Result<Url,String>{
    crate::cloud::validate_cloud_url(raw).map_err(|e|e.to_string())
}
fn platform()->Result<(&'static str,&'static str),String>{
    let os=match std::env::consts::OS{
        "windows"=>"windows","macos"=>"macos","linux"=>"linux",
        _=>return Err("Update platform unsupported".into()),
    };
    let arch=match std::env::consts::ARCH{
        "x86_64"=>"x64","aarch64"=>"arm64",
        _=>return Err("Update CPU architecture unsupported".into()),
    };
    Ok((os,arch))
}
fn canonical(m:&Manifest)->Result<Vec<u8>,String>{
    let fields=[
        m.version.as_deref().ok_or("Missing update version")?,
        m.channel.as_deref().ok_or("Missing update channel")?,
        m.platform.as_deref().ok_or("Missing update platform")?,
        m.arch.as_deref().ok_or("Missing update arch")?,
        m.url.as_deref().ok_or("Missing update URL")?,
        m.sha256.as_deref().ok_or("Missing update digest")?,
    ];
    let size=m.size.ok_or("Missing update size")?;
    Ok(format!("{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
        fields[0],fields[1],fields[2],fields[3],fields[4],
        fields[5],size).into_bytes())
}
fn newer(candidate:&str,current:&str)->bool{
    fn nums(v:&str)->Option<[u32;3]>{
        let parts:Vec<&str>=v.split('-').next()?.split('.').collect();
        if parts.len()!=3||parts.iter().any(|x|x.len()>4||x.is_empty()){return None}
        Some([parts[0].parse().ok()?,parts[1].parse().ok()?,
            parts[2].parse().ok()?])
    }
    nums(candidate).zip(nums(current)).is_some_and(|(a,b)|a>b)
}
pub fn verify(m:&Manifest,channel:&str)->Result<(),String>{
    if !m.available{return Err("No release available".into())}
    let (os,arch)=platform()?;
    if m.channel.as_deref()!=Some(channel) ||
        m.platform.as_deref()!=Some(os)||m.arch.as_deref()!=Some(arch) {
        return Err("Release manifest targets another channel/platform".into())
    }
    let version=m.version.as_deref().ok_or("Release version absent")?;
    if !newer(version,env!("CARGO_PKG_VERSION")){
        return Err("Manifest is not a newer release".into())
    }
    let url=m.url.as_deref().ok_or("Release URL absent")?;
    let parsed=Url::parse(url).map_err(|e|e.to_string())?;
    if parsed.scheme()!="https"||parsed.host_str().is_none()||
        !parsed.username().is_empty()||parsed.password().is_some()||
        parsed.query().is_some()||parsed.fragment().is_some(){
        return Err("Release URL must be HTTPS without credentials/query".into())
    }
    let hash=m.sha256.as_deref().ok_or("Missing SHA256 digest")?;
    if hash.len()!=64||!hash.bytes().all(|c|c.is_ascii_hexdigit()) ||
        hash!=hash.to_ascii_lowercase(){return Err("Invalid SHA256 digest".into())}
    let size=m.size.ok_or("Missing release size")?;
    if size==0||size>2_500_000_000{return Err("Invalid release size".into())}
    if m.algorithm.as_deref()!=Some("Ed25519"){return Err("Unsupported update signature".into())}
    let encoded=m.signature.as_deref().ok_or("Missing update signature")?;
    let signature=STANDARD.decode(encoded).map_err(|e|e.to_string())?;
    let signature=Signature::from_slice(&signature).map_err(|e|e.to_string())?;
    public_key()?.verify(&canonical(m)?,&signature)
        .map_err(|_|"Untrusted update signature".into())
}
pub fn check(channel:&str)->Result<Option<Manifest>,String>{
    if !["stable","beta"].contains(&channel){return Err("Unknown update channel".into())}
    // Verify the trust anchor exists before contacting the remote service.
    public_key()?;
    let (os,arch)=platform()?;
    let mut url=location(&crate::cloud::saved_endpoint()
        .map_err(|e|e.to_string())?.to_string())?
        .join("v1/updates/manifest").map_err(|e|e.to_string())?;
    url.query_pairs_mut().append_pair("channel",channel)
        .append_pair("platform",os).append_pair("arch",arch)
        .append_pair("version",env!("CARGO_PKG_VERSION"));
    let mut response=client()?.get(url).send().map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err(format!(
        "Update service HTTP {}",response.status()))}
    let mut bytes=Vec::new();
    response.take(65_537).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>65_536{return Err("Update manifest too large".into())}
    let manifest:Manifest=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    if !manifest.available{return Ok(None)}
    verify(&manifest,channel)?;
    Ok(Some(manifest))
}
pub fn stage(manifest:&Manifest,channel:&str)->Result<std::path::PathBuf,String>{
    verify(manifest,channel)?;
    let url=manifest.url.as_deref().ok_or("Update URL absent")?;
    let expected=manifest.size.ok_or("Size absent")?;
    let mut response=client()?.get(url).send().map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err("Release asset unavailable".into())}
    let root=crate::profile::profile_root().join("downloads").join("updates");
    fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    use rand::RngCore;
    let mut rng=rand::rngs::OsRng;
    let path=root.join(format!("Charlotte-update-{:016x}.pending",rng.next_u64()));
    let mut file=OpenOptions::new().create_new(true).write(true).open(&path)
        .map_err(|e|e.to_string())?;
    let result=(||->Result<(),String>{
        let mut hasher=Sha256::new();
        let mut total=0u64;let mut buffer=[0u8;65536];
        loop {
            let amount=response.read(&mut buffer).map_err(|e|e.to_string())?;
            if amount==0{break}
            total=total.checked_add(amount as u64).ok_or("Update size overflow")?;
            if total>expected{return Err("Downloaded update exceeds signed size".into())}
            hasher.update(&buffer[..amount]);
            file.write_all(&buffer[..amount]).map_err(|e|e.to_string())?;
        }
        if total!=expected{return Err("Update size mismatch".into())}
        let digest=format!("{:x}",hasher.finalize());
        if Some(digest.as_str())!=manifest.sha256.as_deref(){
            return Err("Update SHA256 mismatch".into())
        }
        file.sync_all().map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(error)=result{drop(file);let _=fs::remove_file(&path);return Err(error)}
    Ok(path)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn compares_semver_components(){
        assert!(newer("0.4.1","0.4.0"));
        assert!(!newer("0.3.9","0.4.0"));
        assert!(!newer("1.0.0","2.0.0"));
    }
    #[test]fn update_verification_is_disabled_without_a_trusted_build_key(){
        if option_env!("CHARLOTTE_RELEASE_PUBKEY_HEX").is_none(){
            assert!(public_key().is_err());
        }
    }
}
