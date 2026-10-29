//! Explicit import of user-chosen Electron userData bookmarks and history.
//! Never imports API credentials, browser cookies, or executable state.
use std::{fs,path::Path};
use serde::Deserialize;
use crate::storage::Storage;

#[derive(Deserialize)] struct BookmarksFile {
  #[serde(default)] entries: Vec<LegacyBookmark>,
  #[serde(default)] folders: Vec<LegacyFolder>,
}
#[derive(Deserialize)] struct LegacyBookmark {
  url:String, title:String,
  #[serde(rename="folderId")] folder_id:Option<String>,
}
#[derive(Deserialize)] struct LegacyFolder {
  id:String, name:String,
  #[serde(rename="parentId")] parent_id:Option<String>,
}
#[derive(Deserialize)] struct HistoryFile {#[serde(default)] entries:Vec<LegacyHistory>}
#[derive(Deserialize)] struct LegacyHistory {url:String,title:String,timestamp:i64}
#[derive(Default,Debug,PartialEq,Eq)]
pub struct ImportReport {pub bookmarks:usize,pub folders:usize,pub visits:usize}

fn read<T:for<'de> Deserialize<'de>>(path:&Path)->Result<Option<T>,String>{
  if !path.exists(){return Ok(None)}
  let size=fs::metadata(path).map_err(|e|e.to_string())?.len();
  if size>32*1024*1024 {return Err(format!("Legacy JSON too large: {}",path.display()))}
  let bytes=fs::read(path).map_err(|e|e.to_string())?;
  serde_json::from_slice(&bytes).map(Some).map_err(|e|format!("{}: {e}",path.display()))
}
fn safe_url(raw:&str)->bool {
  url::Url::parse(raw).ok().is_some_and(|url|matches!(url.scheme(),"https"|"http") && url.host_str().is_some())
}

/// Import is opt-in and idempotent after successful completion.
pub fn import_dir(db:&Storage,source:&Path)->Result<ImportReport,String>{
  if !source.is_dir(){return Err(format!("Legacy directory not found: {}",source.display()))}
  if db.setting("legacy_imported").map_err(|e|e.to_string())?.as_deref()==Some("v1"){
    return Ok(ImportReport::default())
  }
  let bookmarks:Option<BookmarksFile>=read(&source.join("bookmarks.json"))?;
  let history:Option<HistoryFile>=read(&source.join("history.json"))?;
  let mut report=ImportReport::default();
  if let Some(file)=bookmarks {
    for folder in file.folders {
      if folder.id.len()>256 || folder.name.len()>4096 {continue}
      db.save_folder(&folder.id,&folder.name,folder.parent_id.as_deref()).map_err(|e|e.to_string())?;
      report.folders+=1;
    }
    for bookmark in file.entries {
      if !safe_url(&bookmark.url)||bookmark.url.len()>8192 {continue}
      db.save_bookmark_folder(&bookmark.url,&bookmark.title,bookmark.folder_id.as_deref()).map_err(|e|e.to_string())?;
      report.bookmarks+=1;
    }
  }
  if let Some(file)=history {
    let entries=file.entries;
    let skip=entries.len().saturating_sub(5000);
    for visit in entries.into_iter().skip(skip) {
      if !safe_url(&visit.url)||visit.url.len()>8192 {continue}
      db.record_visit(&visit.url,&visit.title,visit.timestamp).map_err(|e|e.to_string())?;
      report.visits+=1;
    }
  }
  db.set_setting("legacy_imported","v1").map_err(|e|e.to_string())?;
  Ok(report)
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]fn safe_urls(){assert!(safe_url("https://example.com"));assert!(!safe_url("javascript:alert(1)"));assert!(!safe_url("file:///etc/passwd"));}
}
