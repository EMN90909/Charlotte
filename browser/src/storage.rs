//! Local application data. Keep secrets out of this database; use the OS keychain.
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub struct Bookmark { pub url: String, pub title: String }

#[derive(Debug, PartialEq, Eq)]
pub struct Visit { pub id: i64, pub url: String, pub title: String, pub visited_at: i64 }

#[derive(Debug, PartialEq, Eq)]
pub struct Message { pub id: i64, pub room: String, pub sender: String, pub body: String, pub created_at: i64 }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PinnedSite {
    pub url: String,
    pub title: String,
    pub favicon_url: String,
    pub position: i64,
    pub pinned_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkMessage {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentWorkSession {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub mode: String, // "local" | "cloud_e2b"
    pub cloud_sandbox_id: String,
    pub status: String, // "running" | "completed" | "awaiting_approval" | "error"
    pub messages: Vec<WorkMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A durable automation progress marker. No screenshots, cookies, field
/// values or approval tokens are ever written to the run log.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct AutomationRun{
    pub id:String,pub goal:String,pub origin:String,pub provider:String,
    pub visual:bool,pub step:u8,pub status:String,pub summary:String,
    pub updated_at:i64,
}
pub struct Storage(Connection);

impl Storage {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 2 { return Err(rusqlite::Error::InvalidQuery); }
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             BEGIN IMMEDIATE;
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS bookmarks (url TEXT PRIMARY KEY, title TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS history (id INTEGER PRIMARY KEY, url TEXT NOT NULL, title TEXT NOT NULL, visited_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS history_visited_at ON history(visited_at DESC);
             CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY, room TEXT NOT NULL, sender TEXT NOT NULL, body TEXT NOT NULL, created_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS messages_room_time ON messages(room, created_at);
             CREATE TABLE IF NOT EXISTS bookmark_folders (id TEXT PRIMARY KEY, name TEXT NOT NULL, parent_id TEXT);
             CREATE TABLE IF NOT EXISTS downloads (
                id INTEGER NOT NULL, source_url TEXT NOT NULL, save_path TEXT NOT NULL,
                received_bytes INTEGER NOT NULL, total_bytes INTEGER NOT NULL,
                status TEXT NOT NULL, finished_at INTEGER NOT NULL,
                PRIMARY KEY(id, finished_at)
             );
             CREATE INDEX IF NOT EXISTS downloads_recent ON downloads(finished_at DESC);
             CREATE TABLE IF NOT EXISTS automation_runs (
                id TEXT PRIMARY KEY,
                goal TEXT NOT NULL,
                origin TEXT NOT NULL,
                provider TEXT NOT NULL,
                visual INTEGER NOT NULL,
                step INTEGER NOT NULL,
                status TEXT NOT NULL,
                summary TEXT NOT NULL,
                updated_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS automation_runs_recent
                ON automation_runs(updated_at DESC);
             CREATE TABLE IF NOT EXISTS pinned_sites (
                url TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                favicon_url TEXT NOT NULL DEFAULT '',
                position INTEGER NOT NULL DEFAULT 0,
                pinned_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS pinned_sites_pos ON pinned_sites(position ASC, pinned_at ASC);
             CREATE TABLE IF NOT EXISTS agent_work_sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                prompt TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'local',
                cloud_sandbox_id TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'running',
                messages_json TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS agent_work_recent ON agent_work_sessions(updated_at DESC);
             COMMIT;",
        )?;
        if version < 2 {
            let has_folder = {
                let mut statement = connection.prepare("PRAGMA table_info(bookmarks)")?;
                let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
                rows.filter_map(Result::ok).any(|col| col == "folder_id")
            };
            if !has_folder { connection.execute_batch("ALTER TABLE bookmarks ADD COLUMN folder_id TEXT;")?; }
            connection.pragma_update(None, "user_version", 2)?;
        }
        Ok(Self(connection))
    }

    /// A task is written before execution, never after a step's side effect.
    /// Actions, typed field contents and screenshots are purposely NOT saved.
    pub fn save_automation_run(&self,run:&AutomationRun)->rusqlite::Result<()>{
        if run.goal.len()>4000||run.summary.len()>500||run.step>100{
            return Err(rusqlite::Error::InvalidQuery)
        }
        self.0.execute("INSERT INTO automation_runs
            (id,goal,origin,provider,visual,step,status,summary,updated_at)
            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
            ON CONFLICT(id) DO UPDATE SET
            step=excluded.step,status=excluded.status,summary=excluded.summary,
            updated_at=excluded.updated_at",
            params![run.id,run.goal,run.origin,run.provider,
                i64::from(run.visual),i64::from(run.step),run.status,run.summary,
                run.updated_at])?;
        // Keep a small local audit log; no indefinite collection.
        self.0.execute("DELETE FROM automation_runs WHERE id NOT IN
            (SELECT id FROM automation_runs ORDER BY updated_at DESC LIMIT 25)",[])?;
        Ok(())
    }
    pub fn latest_paused_automation_run(&self)->rusqlite::Result<Option<AutomationRun>>{
        self.0.query_row("SELECT id,goal,origin,provider,visual,step,status,summary,updated_at
            FROM automation_runs WHERE status IN ('observing','planning',
             'awaiting_approval','executing','verifying','paused','interrupted')
            ORDER BY updated_at DESC LIMIT 1",[],|row|{
                let step:i64=row.get(5)?;
                Ok(AutomationRun{
                    id:row.get(0)?,goal:row.get(1)?,origin:row.get(2)?,
                    provider:row.get(3)?,visual:row.get::<_,i64>(4)?!=0,
                    step:step.clamp(0,255) as u8,status:row.get(6)?,
                    summary:row.get(7)?,updated_at:row.get(8)?
                })
            }).optional()
    }
    pub fn set_automation_status(&self,id:&str,status:&str,summary:&str,step:u8)
        ->rusqlite::Result<bool>{
        let now=std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
        Ok(self.0.execute("UPDATE automation_runs SET status=?2,summary=?3,
            step=?4,updated_at=?5 WHERE id=?1",params![
                id,status,summary.chars().take(500).collect::<String>(),i64::from(step),now
            ])?!=0)
    }

    pub fn setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.0.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0)).optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.0.execute("INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value])?;
        Ok(())
    }

    pub fn remove_setting(&self, key: &str) -> rusqlite::Result<bool> {
        Ok(self.0.execute("DELETE FROM settings WHERE key = ?1", [key])? != 0)
    }

    pub fn save_bookmark(&self, url: &str, title: &str) -> rusqlite::Result<()> {
        self.0.execute("INSERT INTO bookmarks(url, title) VALUES (?1, ?2) ON CONFLICT(url) DO UPDATE SET title=excluded.title", params![url, title])?;
        Ok(())
    }

    pub fn save_folder(&self, id: &str, name: &str, parent: Option<&str>) -> rusqlite::Result<()> {
        self.0.execute("INSERT INTO bookmark_folders(id,name,parent_id) VALUES (?1,?2,?3)
            ON CONFLICT(id) DO UPDATE SET name=excluded.name,parent_id=excluded.parent_id",
            params![id,name,parent])?;
        Ok(())
    }
    pub fn save_bookmark_folder(&self, url: &str, title: &str, folder: Option<&str>) -> rusqlite::Result<()> {
        self.0.execute("INSERT INTO bookmarks(url,title,folder_id) VALUES (?1,?2,?3)
            ON CONFLICT(url) DO UPDATE SET title=excluded.title,folder_id=excluded.folder_id",
            params![url,title,folder])?;
        Ok(())
    }
    /// Toggle a bookmarked page from native Ctrl/Cmd+D. The browser does
    /// not store script/file/opaque URLs as bookmarks.
    pub fn toggle_bookmark(&self, url: &str, title: &str) -> rusqlite::Result<bool> {
        let exists = self.0.query_row(
            "SELECT EXISTS(SELECT 1 FROM bookmarks WHERE url = ?1)",
            [url], |row| row.get::<_, i64>(0),
        )? != 0;
        if exists {
            self.remove_bookmark(url)?;
            Ok(false)
        } else {
            self.save_bookmark(url, title)?;
            Ok(true)
        }
    }
    pub fn remove_bookmark(&self, url: &str) -> rusqlite::Result<bool> {
        Ok(self.0.execute("DELETE FROM bookmarks WHERE url = ?1", [url])? != 0)
    }

    pub fn bookmarks(&self) -> rusqlite::Result<Vec<Bookmark>> {
        let mut statement = self.0.prepare("SELECT url, title FROM bookmarks ORDER BY title COLLATE NOCASE, url")?;
        let rows = statement.query_map([], |row| Ok(Bookmark { url: row.get(0)?, title: row.get(1)? }))?;
        rows.collect()
    }

    /// Timestamps are Unix milliseconds, supplied by the caller.
    pub fn record_visit(&self, url: &str, title: &str, visited_at: i64) -> rusqlite::Result<i64> {
        self.0.execute("INSERT INTO history(url, title, visited_at) VALUES (?1, ?2, ?3)", params![url, title, visited_at])?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn recent_history(&self, limit: u32) -> rusqlite::Result<Vec<Visit>> {
        let mut statement = self.0.prepare("SELECT id, url, title, visited_at FROM history ORDER BY visited_at DESC, id DESC LIMIT ?1")?;
        let rows = statement.query_map([i64::from(limit.min(10_000))], |row| Ok(Visit { id: row.get(0)?, url: row.get(1)?, title: row.get(2)?, visited_at: row.get(3)? }))?;
        rows.collect()
    }

    pub fn clear_history(&self) -> rusqlite::Result<usize> { self.0.execute("DELETE FROM history", []) }

    pub fn add_message(&self, room: &str, sender: &str, body: &str, created_at: i64) -> rusqlite::Result<i64> {
        self.0.execute("INSERT INTO messages(room, sender, body, created_at) VALUES (?1, ?2, ?3, ?4)", params![room, sender, body, created_at])?;
        Ok(self.0.last_insert_rowid())
    }

    /// Returns the most recent messages in chronological order.
    pub fn messages(&self, room: &str, limit: u32) -> rusqlite::Result<Vec<Message>> {
        let mut statement = self.0.prepare("SELECT id, room, sender, body, created_at FROM (SELECT id, room, sender, body, created_at FROM messages WHERE room = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2) ORDER BY created_at, id")?;
        let rows = statement.query_map(params![room, i64::from(limit.min(10_000))], |row| Ok(Message { id: row.get(0)?, room: row.get(1)?, sender: row.get(2)?, body: row.get(3)?, created_at: row.get(4)? }))?;
        rows.collect()
    }

    pub fn delete_message(&self, room: &str, id: i64) -> rusqlite::Result<bool> {
        Ok(self.0.execute("DELETE FROM messages WHERE room = ?1 AND id = ?2", params![room, id])? != 0)
    }

    /// Persist only terminal CEF downloads. A download's browser session
    /// identifier can be reused on restart, so timestamp forms the key.
    pub fn record_download(&self, id: u32, url: &str, path: &str, received: i64,
        total: i64, status: &str, finished_at: i64) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT INTO downloads(id,source_url,save_path,received_bytes,total_bytes,status,finished_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id,finished_at) DO UPDATE
             SET received_bytes=excluded.received_bytes,
                 total_bytes=excluded.total_bytes,status=excluded.status",
            params![i64::from(id),url,path,received,total,status,finished_at]
        )?;
        Ok(())
    }
    pub fn recent_downloads(&self, limit: u32) -> rusqlite::Result<Vec<(i64,String,String,i64,i64,String,i64)>> {
        let mut stmt = self.0.prepare(
            "SELECT id,source_url,save_path,received_bytes,total_bytes,status,finished_at
             FROM downloads ORDER BY finished_at DESC,id DESC LIMIT ?1"
        )?;
        stmt.query_map([i64::from(limit.min(1000))], |row| Ok((
            row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,
            row.get(4)?,row.get(5)?,row.get(6)?
        )))?.collect()
    }
    pub fn clear_downloads(&self) -> rusqlite::Result<usize> {
        self.0.execute("DELETE FROM downloads", [])
    }

    // --- Pinned Websites (only populated when user explicitly pins a site) ---

    pub fn pin_site(&self, url: &str, title: &str, favicon_url: &str) -> rusqlite::Result<()> {
        if !crate::session::valid_page_url(url) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let next_pos: i64 = self.0.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM pinned_sites",
            [],
            |row| row.get(0),
        )?;
        self.0.execute(
            "INSERT INTO pinned_sites(url, title, favicon_url, position, pinned_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(url) DO UPDATE SET title=excluded.title, favicon_url=excluded.favicon_url",
            params![url, title, favicon_url, next_pos, now],
        )?;
        Ok(())
    }

    pub fn unpin_site(&self, url: &str) -> rusqlite::Result<bool> {
        Ok(self.0.execute("DELETE FROM pinned_sites WHERE url = ?1", [url])? != 0)
    }

    pub fn is_pinned(&self, url: &str) -> rusqlite::Result<bool> {
        Ok(self.0.query_row(
            "SELECT EXISTS(SELECT 1 FROM pinned_sites WHERE url = ?1)",
            [url],
            |row| row.get::<_, i64>(0),
        )? != 0)
    }

    pub fn pinned_sites(&self) -> rusqlite::Result<Vec<PinnedSite>> {
        let mut stmt = self.0.prepare(
            "SELECT url, title, favicon_url, position, pinned_at
             FROM pinned_sites ORDER BY position ASC, pinned_at ASC LIMIT 24",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PinnedSite {
                url: row.get(0)?,
                title: row.get(1)?,
                favicon_url: row.get(2)?,
                position: row.get(3)?,
                pinned_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    // --- History helpers for Last Pages Visited & charlotte://history ---

    pub fn recent_unique_history(&self, limit: u32) -> rusqlite::Result<Vec<Visit>> {
        let mut stmt = self.0.prepare(
            "SELECT h.id, h.url, h.title, h.visited_at
             FROM history h
             INNER JOIN (
                 SELECT url, MAX(visited_at) AS max_ts
                 FROM history
                 GROUP BY url
             ) latest ON h.url = latest.url AND h.visited_at = latest.max_ts
             GROUP BY h.url
             ORDER BY h.visited_at DESC, h.id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([i64::from(limit.min(500))], |row| {
            Ok(Visit {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                visited_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn search_history(&self, query: &str, limit: u32) -> rusqlite::Result<Vec<Visit>> {
        let pattern = format!("%{}%", query.trim());
        let mut stmt = self.0.prepare(
            "SELECT id, url, title, visited_at
             FROM history
             WHERE url LIKE ?1 OR title LIKE ?1
             ORDER BY visited_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, i64::from(limit.min(1000))], |row| {
            Ok(Visit {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                visited_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_visit(&self, id: i64) -> rusqlite::Result<bool> {
        Ok(self.0.execute("DELETE FROM history WHERE id = ?1", [id])? != 0)
    }

    // --- Agent Work Sessions ("Previous work" & Active Work Thread) ---

    pub fn save_work_session(&self, session: &AgentWorkSession) -> rusqlite::Result<()> {
        let messages_json = serde_json::to_string(&session.messages)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        self.0.execute(
            "INSERT INTO agent_work_sessions
             (id, title, prompt, mode, cloud_sandbox_id, status, messages_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
             title=excluded.title, prompt=excluded.prompt, mode=excluded.mode,
             cloud_sandbox_id=excluded.cloud_sandbox_id, status=excluded.status,
             messages_json=excluded.messages_json, updated_at=excluded.updated_at",
            params![
                session.id,
                session.title,
                session.prompt,
                session.mode,
                session.cloud_sandbox_id,
                session.status,
                messages_json,
                session.created_at,
                session.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_work_sessions(&self, limit: u32) -> rusqlite::Result<Vec<AgentWorkSession>> {
        let mut stmt = self.0.prepare(
            "SELECT id, title, prompt, mode, cloud_sandbox_id, status, messages_json, created_at, updated_at
             FROM agent_work_sessions
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([i64::from(limit.min(200))], |row| {
            let msg_raw: String = row.get(6)?;
            let messages: Vec<WorkMessage> = serde_json::from_str(&msg_raw).unwrap_or_default();
            Ok(AgentWorkSession {
                id: row.get(0)?,
                title: row.get(1)?,
                prompt: row.get(2)?,
                mode: row.get(3)?,
                cloud_sandbox_id: row.get(4)?,
                status: row.get(5)?,
                messages,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_work_session(&self, id: &str) -> rusqlite::Result<Option<AgentWorkSession>> {
        self.0
            .query_row(
                "SELECT id, title, prompt, mode, cloud_sandbox_id, status, messages_json, created_at, updated_at
                 FROM agent_work_sessions WHERE id = ?1",
                [id],
                |row| {
                    let msg_raw: String = row.get(6)?;
                    let messages: Vec<WorkMessage> = serde_json::from_str(&msg_raw).unwrap_or_default();
                    Ok(AgentWorkSession {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        prompt: row.get(2)?,
                        mode: row.get(3)?,
                        cloud_sandbox_id: row.get(4)?,
                        status: row.get(5)?,
                        messages,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .optional()
    }

    pub fn append_work_message(&self, id: &str, msg: WorkMessage, status: Option<&str>) -> rusqlite::Result<bool> {
        let Some(mut session) = self.get_work_session(id)? else {
            return Ok(false);
        };
        session.updated_at = msg.timestamp;
        session.messages.push(msg);
        if let Some(st) = status {
            session.status = st.to_owned();
        }
        self.save_work_session(&session)?;
        Ok(true)
    }

    pub fn delete_work_session(&self, id: &str) -> rusqlite::Result<bool> {
        Ok(self.0.execute("DELETE FROM agent_work_sessions WHERE id = ?1", [id])? != 0)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_checkpoints_survive_reopening_without_secrets(){
        let path=std::env::temp_dir().join(format!("charlotte-automation-{}.sqlite",std::process::id()));
        let checkpoint=AutomationRun{id:"test".into(),goal:"Review product plans".into(),
            origin:"https://example.org".into(),provider:"ollama".into(),visual:false,
            step:1,status:"awaiting_approval".into(),summary:"Click plan details".into(),
            updated_at:24};
        {let db=Storage::open(&path).unwrap();
         db.save_automation_run(&checkpoint).unwrap();}
        let db=Storage::open(&path).unwrap();
        assert_eq!(db.latest_paused_automation_run().unwrap(),Some(checkpoint));
        assert!(db.set_automation_status("test","done","Completed",2).unwrap());
        assert!(db.latest_paused_automation_run().unwrap().is_none());
        drop(db);let _=std::fs::remove_file(path);
    }
    #[test]
    fn settings_survive_reopening() {
        let path = std::env::temp_dir().join(format!("charlotte-settings-{}-{}.sqlite", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        {
            let db = Storage::open(&path).unwrap();
            assert_eq!(db.setting("shields").unwrap(), None);
            db.set_setting("shields", "on").unwrap();
            db.set_setting("shields", "off").unwrap();
        }
        let db = Storage::open(&path).unwrap();
        assert_eq!(db.setting("shields").unwrap().as_deref(), Some("off"));
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bookmarks_history_and_messages_are_isolated() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        db.save_bookmark("https://example.com", "Old").unwrap();
        db.save_bookmark("https://example.com", "New").unwrap();
        assert_eq!(db.bookmarks().unwrap()[0].title, "New");
        db.record_visit("https://example.com", "Example", 20).unwrap();
        db.record_visit("https://example.org", "Other", 10).unwrap();
        assert_eq!(db.recent_history(1).unwrap()[0].visited_at, 20);
        db.add_message("one", "Ada", "first", 1).unwrap();
        db.add_message("two", "Ada", "other", 2).unwrap();
        let id = db.add_message("one", "Ada", "last", 3).unwrap();
        assert_eq!(db.messages("one", 1).unwrap()[0].body, "last");
        assert!(!db.delete_message("two", id).unwrap());
        assert!(db.delete_message("one", id).unwrap());
        assert_eq!(db.clear_history().unwrap(), 2);
    }

    #[test]
    fn native_bookmark_toggle() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        assert!(db.toggle_bookmark("https://example.org", "Example").unwrap());
        assert_eq!(db.bookmarks().unwrap().len(), 1);
        assert!(!db.toggle_bookmark("https://example.org", "Example").unwrap());
        assert!(db.bookmarks().unwrap().is_empty());
    }

    #[test]
    fn rejects_future_database_versions() {
        let path = std::env::temp_dir().join(format!("charlotte-future-{}-{}.sqlite", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "user_version", 3).unwrap();
        drop(db);
        assert!(Storage::open(&path).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pinned_sites_and_work_sessions_start_empty_and_persist() {
        let db = Storage::open(Path::new(":memory:")).unwrap();
        assert!(db.pinned_sites().unwrap().is_empty());
        assert!(db.list_work_sessions(10).unwrap().is_empty());

        db.pin_site("https://github.com", "GitHub", "https://github.com/favicon.ico").unwrap();
        assert_eq!(db.pinned_sites().unwrap().len(), 1);
        assert!(db.is_pinned("https://github.com").unwrap());
        assert!(db.unpin_site("https://github.com").unwrap());
        assert!(db.pinned_sites().unwrap().is_empty());

        db.record_visit("https://rust-lang.org", "Rust", 100).unwrap();
        db.record_visit("https://rust-lang.org", "Rust Lang", 200).unwrap();
        db.record_visit("https://github.com", "GitHub", 150).unwrap();
        let uniq = db.recent_unique_history(5).unwrap();
        assert_eq!(uniq.len(), 2);
        assert_eq!(uniq[0].url, "https://rust-lang.org");

        let sess = AgentWorkSession {
            id: "work-1".into(),
            title: "Cloud research".into(),
            prompt: "Research solar providers".into(),
            mode: "cloud_e2b".into(),
            cloud_sandbox_id: "sb-123".into(),
            status: "running".into(),
            messages: vec![WorkMessage {
                role: "user".into(),
                content: "Research solar providers".into(),
                timestamp: 1000,
            }],
            created_at: 1000,
            updated_at: 1000,
        };
        db.save_work_session(&sess).unwrap();
        assert_eq!(db.list_work_sessions(10).unwrap().len(), 1);
        db.append_work_message(
            "work-1",
            WorkMessage {
                role: "assistant".into(),
                content: "Provisioned E2B sandbox".into(),
                timestamp: 1005,
            },
            Some("completed"),
        )
        .unwrap();
        let loaded = db.get_work_session("work-1").unwrap().unwrap();
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.status, "completed");
    }
}
