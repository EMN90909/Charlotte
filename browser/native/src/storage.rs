//! Local application data. Keep secrets out of this database; use the OS keychain.
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Storage(Connection);

impl Storage {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS bookmarks (url TEXT PRIMARY KEY, title TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS history (id INTEGER PRIMARY KEY, url TEXT NOT NULL, title TEXT NOT NULL, visited_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS history_visited_at ON history(visited_at DESC);
             CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY, room TEXT NOT NULL, sender TEXT NOT NULL, body TEXT NOT NULL, created_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS messages_room_time ON messages(room, created_at);",
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
}
