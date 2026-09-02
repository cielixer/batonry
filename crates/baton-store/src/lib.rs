//! One SQLite database on the user's machine, plus text export/import.
//!
//! Nothing is ever written to a remote host.
//!
//! Stage 1 stores `app_pref` only; the schema grows in stage 4.

use std::path::Path;

use baton_core::Store;

/// The SQLite-backed persistence adapter for Baton preferences.
pub struct SqliteStore {
    conn: rusqlite::Connection,
}

impl SqliteStore {
    /// Opens the database at `path`, creating its parent directory and schema.
    pub fn open_at(path: &Path) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = path.parent() {
            // If directory creation fails, opening the database reports the
            // failure with the path and preserves this constructor's error.
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = rusqlite::Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_pref (\
                key TEXT PRIMARY KEY, \
                value TEXT NOT NULL\
            )",
            [],
        )?;
        Ok(Self { conn })
    }
}

impl Store for SqliteStore {
    fn app_pref(&self, key: &str) -> Option<String> {
        // A broken preference store degrades to defaults. Never log the
        // value: root rule 7 forbids writing anything but hostnames and users.
        self.conn
            .query_row(
                "SELECT value FROM app_pref WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .ok()
    }

    fn set_app_pref(&mut self, key: &str, value: &str) {
        // Convenience state is allowed to disappear; a database error must
        // not panic or make the application unusable.
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO app_pref (key, value) VALUES (?1, ?2)",
            [key, value],
        );
    }
}
