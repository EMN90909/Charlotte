//! Completed CEF download history. Browser downloads are saved only through
//! Chromium's native Save dialog; this module never opens a remote URL or
//! writes a downloaded payload. Never trust a suggested remote filename.
use crate::storage::Storage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub id: u32,
    pub source_url: String,
    pub save_path: String,
    pub received_bytes: i64,
    pub total_bytes: i64,
    pub status: Status,
    pub finished_at: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status { InProgress, Paused, Completed, Canceled, Interrupted }
impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Canceled => "canceled",
            Self::Interrupted => "interrupted",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "in_progress" => Some(Self::InProgress),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "canceled" => Some(Self::Canceled),
            "interrupted" => Some(Self::Interrupted),
            _ => None,
        }
    }
}
impl Download {
    pub fn finished(id: u32, source_url: &str, save_path: &str, received_bytes: i64,
        total_bytes: i64, status: Status) -> Option<Self> {
        if !crate::session::valid_page_url(source_url) || source_url.len() > 8192 ||
            save_path.len() > 8192 || id == 0 {
            return None;
        }
        let finished_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
        Some(Self {
            id, source_url: source_url.to_owned(), save_path: save_path.to_owned(),
            received_bytes: received_bytes.max(0), total_bytes: total_bytes.max(-1), status,
            finished_at
        })
    }
}
pub fn record(db: &Storage, download: &Download) -> rusqlite::Result<()> {
    db.record_download(
        download.id, &download.source_url, &download.save_path,
        download.received_bytes, download.total_bytes,
        download.status.as_str(), download.finished_at,
    )
}
pub fn recent(db: &Storage, max: usize) -> rusqlite::Result<Vec<Download>> {
    db.recent_downloads(max.min(1000) as u32).map(|rows| {
        rows.into_iter().filter_map(|(id, source_url, save_path, received_bytes, total_bytes, status, finished_at)| {
            Some(Download {
                id: u32::try_from(id).ok()?, source_url, save_path,
                received_bytes, total_bytes, status: Status::parse(&status)?, finished_at
            })
        }).collect()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn completed_download_is_durable() {
        let db=Storage::open(Path::new(":memory:")).unwrap();
        let d=Download::finished(1,"https://example.com/file.txt","/home/user/file.txt",1024,1024,Status::Completed).unwrap();
        record(&db,&d).unwrap();
        assert_eq!(recent(&db,10).unwrap(),vec![d]);
    }
    #[test]
    fn invalid_scheme_is_not_recorded() {
        assert!(Download::finished(1,"file:///etc/passwd","/tmp/x",3,3,Status::Completed).is_none());
        assert!(Download::finished(1,"javascript:evil()","/tmp/x",3,3,Status::Completed).is_none());
    }
    #[test]
    fn negative_unknown_total_is_preserved() {
        let d=Download::finished(2,"https://a.example/download","",0,-1,Status::Interrupted).unwrap();
        assert_eq!(d.total_bytes,-1);
    }
}
