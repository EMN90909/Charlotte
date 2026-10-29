//! Native Rust filter-list subscriptions for Charlotte's built-in Shield.
//! Inspired by OpenShield-js list management, but uses Brave's adblock-rust
//! engine directly. No JavaScript or Node dependency is shipped.
//!
//! Downloads are performed only on a background thread or in the settings CLI,
//! never in CEF resource callbacks. Lists are capped, cached, and optional.
//! The offline starter rules always remain available.
use crate::{shield::{Shield, STARTER_RULES}, storage::Storage};
use std::{fs, io::Read, path::{Path,PathBuf}, time::{Duration,SystemTime}};

pub const MAX_LIST_BYTES: u64 = 8 * 1024 * 1024;
const UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Copy)]
pub struct Subscription {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub default_enabled: bool,
}

/// Known community-maintained lists from the OpenShield-js subscription model.
/// URL strings are not user-controlled; a manually supplied HTTPS URL is never
/// fetched on behalf of a web page. Add local rules to custom.txt instead.
pub const LISTS: &[Subscription] = &[
    Subscription { id:"easylist", name:"EasyList", url:"https://easylist.to/easylist/easylist.txt", default_enabled:true },
    Subscription { id:"easyprivacy", name:"EasyPrivacy", url:"https://easylist.to/easylist/easyprivacy.txt", default_enabled:true },
    Subscription { id:"peter-lowe", name:"Peter Lowe's List", url:"https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext", default_enabled:false },
    Subscription { id:"fanboy-annoyance", name:"Fanboy Annoyance", url:"https://secure.fanboy.co.nz/fanboy-annoyance.txt", default_enabled:false },
    Subscription { id:"fanboy-social", name:"Fanboy Social", url:"https://easylist.to/easylist/fanboy-social.txt", default_enabled:false },
    Subscription { id:"ublock-filters", name:"uBlock Origin Filters", url:"https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt", default_enabled:false },
    Subscription { id:"ublock-badware", name:"uBlock Origin Badware", url:"https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/badware.txt", default_enabled:false },
    Subscription { id:"ublock-privacy", name:"uBlock Origin Privacy", url:"https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/privacy.txt", default_enabled:false },
];

pub fn list_dir(profile: &Path) -> PathBuf { profile.join("shield-lists") }
pub fn list_file(profile: &Path, id: &str) -> Result<PathBuf,String> {
    if id != "custom" && !LISTS.iter().any(|s|s.id == id) {
        return Err(format!("Unknown shield list: {id}"));
    }
    Ok(list_dir(profile).join(format!("{id}.txt")))
}
pub fn enabled(db: &Storage, list: &Subscription) -> bool {
    match db.setting(&format!("shields:list:{}", list.id)).ok().flatten().as_deref() {
        Some("on") => true,
        Some("off") => false,
        _ => list.default_enabled,
    }
}
pub fn set_enabled(db: &Storage, id: &str, enabled: bool) -> Result<(),String> {
    if !LISTS.iter().any(|s|s.id == id) {return Err("Unknown shield list".into())}
    db.set_setting(&format!("shields:list:{id}"),if enabled {"on"}else{"off"})
        .map_err(|e|e.to_string())
}
pub fn load_cached(db: &Storage, profile: &Path) -> Shield {
    let mut texts=vec![STARTER_RULES.to_owned()];
    for subscription in LISTS.iter().filter(|s| enabled(db,s)) {
        if let Ok(path)=list_file(profile,subscription.id) {
            if let Ok(text)=fs::read_to_string(path) {
                if text.len() as u64 <= MAX_LIST_BYTES {texts.push(text)}
            }
        }
    }
    // Users may add local ABP/uBO rules; never interpret local lists as JS.
    if let Ok(path)=list_file(profile,"custom") {
        if let Ok(text)=fs::read_to_string(path) {
            if text.len() as u64 <= MAX_LIST_BYTES {texts.push(text)}
        }
    }
    let refs:Vec<&str>=texts.iter().map(String::as_str).collect();
    let on=db.setting("shields").ok().flatten().as_deref()!=Some("off");
    Shield::new(&refs,on)
}
fn fresh(path:&Path)->bool {
    fs::metadata(path).and_then(|m|m.modified()).ok()
        .and_then(|date|SystemTime::now().duration_since(date).ok())
        .is_some_and(|age| age < UPDATE_INTERVAL)
}
fn save_list(path:&Path,content:&str)->Result<(),String>{
    let temp=path.with_extension(format!("{}.tmp",std::process::id()));
    fs::write(&temp,content).map_err(|e|e.to_string())?;
    if let Err(err)=fs::rename(&temp,path) {
        // Windows rename does not replace an existing destination.
        if cfg!(windows) && path.exists() {
            fs::remove_file(path).map_err(|e|e.to_string())?;
            fs::rename(&temp,path).map_err(|e|e.to_string())?;
        } else {
            let _=fs::remove_file(&temp);
            return Err(err.to_string());
        }
    }
    Ok(())
}
/// Refresh selected subscriptions over HTTPS; preserve cached rules on errors.
/// The response has an 8 MiB hard cap and cannot redirect off HTTPS.
pub fn update(db:&Storage,profile:&Path,force:bool)->Result<usize,String>{
    fs::create_dir_all(list_dir(profile)).map_err(|e|e.to_string())?;
    let client=reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(|attempt|{
            if attempt.previous().len()>=3 || attempt.url().scheme()!="https" {
                attempt.stop()
            } else { attempt.follow() }
        }))
        .build().map_err(|e|e.to_string())?;
    let mut changed=0;
    for subscription in LISTS.iter().filter(|s|enabled(db,s)){
        let path=list_file(profile,subscription.id)?;
        if !force && fresh(&path){continue}
        let download=(||->Result<String,String>{
            let response=client.get(subscription.url).send()
                .and_then(reqwest::blocking::Response::error_for_status)
                .map_err(|e|e.to_string())?;
            if response.url().scheme()!="https" {return Err("Insecure filter list redirect".into())}
            let mut bytes=Vec::new();
            response.take(MAX_LIST_BYTES+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
            if bytes.len() as u64 >MAX_LIST_BYTES {return Err("List exceeds size limit".into())}
            let content=String::from_utf8(bytes).map_err(|e|e.to_string())?;
            // Reject empty/error pages; don't overwrite a known-good cache.
            if !content.lines().any(|line|{
                let line=line.trim();
                line.starts_with("||") || line.starts_with("@@") || line.contains("##")
                    || line.starts_with('!') || line.starts_with('[')
            }) {return Err("Not a recognizable filter list".into())}
            Ok(content)
        })();
        match download.and_then(|contents|save_list(&path,&contents)){
            Ok(())=>changed+=1,
            Err(err)=>eprintln!("Shield list {}: {err} (using cached rules)",subscription.id),
        }
    }
    Ok(changed)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn list_paths_reject_untrusted_filenames(){
        let root=Path::new("/tmp/charlotte-shield");
        assert!(list_file(root,"../../escape").is_err());
        assert!(list_file(root,"unknown").is_err());
        assert!(list_file(root,"easylist").unwrap().starts_with(root));
        assert!(list_file(root,"custom").is_ok());
    }
    #[test]
    fn local_filters_and_disabled_state_are_respected(){
        let root=std::env::temp_dir().join(format!("charlotte-shield-test-{}",std::process::id()));
        fs::create_dir_all(list_dir(&root)).unwrap();
        let db=Storage::open(&root.join("test.sqlite")).unwrap();
        let rules=list_file(&root,"custom").unwrap();
        fs::write(&rules,"||ads.example.test^\nexample.test##.ad-banner\n").unwrap();
        let shield=load_cached(&db,&root);
        assert_eq!(shield.check("https://ads.example.test/banner.js","https://example.test","script","GET"),
            crate::shield::Decision::Block);
        assert!(shield.cosmetics("https://example.test").hide_selectors.contains(".ad-banner"));
        db.set_setting("shields","off").unwrap();
        let shield=load_cached(&db,&root);
        assert_eq!(shield.check("https://ads.example.test/banner.js","https://example.test","script","GET"),
            crate::shield::Decision::Allow);
        let _=fs::remove_dir_all(root);
    }
    #[test]
    fn subscriptions_are_https_and_unique(){
        for (index,list) in LISTS.iter().enumerate(){
            assert!(list.url.starts_with("https://"));
            assert!(LISTS[..index].iter().all(|old|old.id != list.id));
        }
        assert!(LISTS.iter().any(|s|s.id=="easylist" && s.default_enabled));
        assert!(LISTS.iter().any(|s|s.id=="easyprivacy" && s.default_enabled));
    }
}
