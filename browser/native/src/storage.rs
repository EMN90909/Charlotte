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
        if version > 1 { return Err(rusqlite::Error::InvalidQuery); }
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
             PRAGMA user_version=1;
             COMMIT;",
        )?;
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
    fn rejects_future_database_versions() {
        let path = std::env::temp_dir().join(format!("charlotte-future-{}-{}.sqlite", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "user_version", 2).unwrap();
        drop(db);
        assert!(Storage::open(&path).is_err());
        let _ = std::fs::remove_file(path);
    }
}
