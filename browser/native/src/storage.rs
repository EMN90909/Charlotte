//! Local application data. Keep secrets out of this database; use the OS keychain.
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub struct Bookmark { pub url: String, pub title: String }

#[derive(Debug, PartialEq, Eq)]
pub struct Visit { pub id: i64, pub url: String, pub title: String, pub visited_at: i64 }

#[derive(Debug, PartialEq, Eq)]
pub struct Message { pub id: i64, pub room: String, pub sender: String, pub body: String, pub created_at: i64 }

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

}

#[cfg(test)]
mod tests {
    use super::*;

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
}
