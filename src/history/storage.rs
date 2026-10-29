//! Transcription history storage and retrieval using SQLite.
//!
//! Manages persistent storage of all transcriptions with timestamps,
//! and provides querying capabilities for the history view.

use anyhow::Result;
use chrono::{DateTime, Local};
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// A single transcription entry in the history.
#[derive(Debug, Clone)]
pub struct TranscriptionEntry {
    /// Unique identifier for this transcription
    pub id: i64,
    /// The transcribed text content
    pub text: String,
    /// When this transcription was created
    pub created_at: DateTime<Local>,
}

/// Manages the transcription history database.
pub struct HistoryManager {
    /// Path to the SQLite database file
    database_path: PathBuf,
    /// Connection to the database (lazy-loaded)
    connection: Option<Connection>,
}

/// Makes the database readable only by this user before it is opened: its
/// directory private, and the database, with any journal SQLite left, mode
/// 0600. SQLite gives the journals it creates the database's mode.
fn protect(database_path: &Path) -> Result<()> {
    if let Some(dir) = database_path.parent() {
        crate::app_dirs::private_dir(dir)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .mode(0o600)
            .open(database_path)?;
        for suffix in ["", "-journal", "-wal", "-shm"] {
            let mut path = database_path.as_os_str().to_owned();
            path.push(suffix);
            match std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)) {
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
                _ => {}
            }
        }
    }
    Ok(())
}

impl HistoryManager {
    /// Creates a new history manager for the given data directory.
    ///
    /// # Arguments
    /// * `data_dir` - Directory where the database file will be stored
    ///
    /// # Errors
    /// - If the data directory cannot be accessed
    pub fn new(data_dir: &Path) -> Result<Self> {
        let database_path = data_dir.join("transcription_history.db");

        Ok(Self {
            database_path,
            connection: None,
        })
    }

    /// Initializes database connection and creates tables if necessary.
    ///
    /// # Errors
    /// - If the database file cannot be opened
    /// - If table creation fails
    fn get_connection(&mut self) -> Result<&Connection> {
        if self.connection.is_none() {
            protect(&self.database_path)?;
            let connection = Connection::open(&self.database_path)?;

            connection.execute("PRAGMA foreign_keys = ON", [])?;

            connection.execute(
                "CREATE TABLE IF NOT EXISTS transcriptions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    text TEXT NOT NULL,
                    created_at TEXT NOT NULL
                )",
                [],
            )?;

            self.connection = Some(connection);
        }

        Ok(self.connection.as_ref().unwrap())
    }

    /// Saves a new transcription to the history database.
    ///
    /// # Arguments
    /// * `text` - The transcribed text to save
    ///
    /// # Errors
    /// - If database connection fails
    /// - If insertion fails
    pub fn save_transcription(&mut self, text: &str) -> Result<()> {
        let connection = self.get_connection()?;
        let now = Local::now();
        let timestamp = now.to_rfc3339();

        connection.execute(
            "INSERT INTO transcriptions (text, created_at) VALUES (?1, ?2)",
            params![text, timestamp],
        )?;

        tracing::debug!("Transcription saved to history");
        Ok(())
    }

    /// Retrieves all transcriptions ordered by most recent first.
    ///
    /// # Errors
    /// - If database connection fails
    /// - If query execution fails
    /// - If timestamp parsing fails
    pub fn get_all_transcriptions(&mut self) -> Result<Vec<TranscriptionEntry>> {
        let connection = self.get_connection()?;

        let mut statement = connection
            .prepare("SELECT id, text, created_at FROM transcriptions ORDER BY created_at DESC")?;

        let entries = statement
            .query_map([], |row| {
                let id = row.get::<_, i64>(0)?;
                let text = row.get::<_, String>(1)?;
                let timestamp_str = row.get::<_, String>(2)?;

                let created_at = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Local))
                    .map_err(|_| {
                        rusqlite::Error::InvalidParameterName(
                            "Invalid timestamp format".to_string(),
                        )
                    })?;

                Ok(TranscriptionEntry {
                    id,
                    text,
                    created_at,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Gets the Nth most recent transcription (1-indexed).
    ///
    /// # Arguments
    /// * `index` - 1-based index (1 = most recent, 2 = second most recent, etc.)
    ///
    /// # Errors
    /// - If database connection fails
    /// - If query execution fails
    pub fn get_transcription_by_index(
        &mut self,
        index: usize,
    ) -> Result<Option<TranscriptionEntry>> {
        let connection = self.get_connection()?;
        let offset = index.saturating_sub(1);

        let mut statement = connection.prepare(
            "SELECT id, text, created_at FROM transcriptions ORDER BY created_at DESC LIMIT 1 OFFSET ?1",
        )?;

        let entry = statement
            .query_row(params![offset], |row| {
                let id = row.get::<_, i64>(0)?;
                let text = row.get::<_, String>(1)?;
                let timestamp_str = row.get::<_, String>(2)?;

                let created_at = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Local))
                    .map_err(|_| {
                        rusqlite::Error::InvalidParameterName(
                            "Invalid timestamp format".to_string(),
                        )
                    })?;

                Ok(TranscriptionEntry {
                    id,
                    text,
                    created_at,
                })
            })
            .optional()?;

        Ok(entry)
    }

    /// Retrieves a single transcription by ID.
    ///
    /// # Arguments
    /// * `id` - The ID of the transcription to retrieve
    ///
    /// # Errors
    /// - If database connection fails
    /// - If query execution fails
    /// - If timestamp parsing fails
    pub fn get_transcription(&mut self, id: i64) -> Result<Option<TranscriptionEntry>> {
        let connection = self.get_connection()?;

        let mut statement =
            connection.prepare("SELECT id, text, created_at FROM transcriptions WHERE id = ?1")?;

        let entry = statement
            .query_row(params![id], |row| {
                let entry_id = row.get::<_, i64>(0)?;
                let text = row.get::<_, String>(1)?;
                let timestamp_str = row.get::<_, String>(2)?;

                let created_at = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Local))
                    .map_err(|_| {
                        rusqlite::Error::InvalidParameterName(
                            "Invalid timestamp format".to_string(),
                        )
                    })?;

                Ok(TranscriptionEntry {
                    id: entry_id,
                    text,
                    created_at,
                })
            })
            .optional()?;

        Ok(entry)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    // Dictations are private: another local account must not be able to read
    // the history, even when the umask would make new files readable.
    #[test]
    fn new_history_is_private_to_its_owner() {
        let home = tempfile::tempdir().unwrap();
        let data = home.path().join("ostt");
        // The usual umask, under which new files are readable by others.
        // SAFETY: umask has no preconditions.
        let previous = unsafe { libc::umask(0o022) };
        let mut history = HistoryManager::new(&data).unwrap();
        history.save_transcription("my private words").unwrap();
        unsafe { libc::umask(previous) };

        assert_eq!(mode(&data), 0o700);
        assert_eq!(mode(&data.join("transcription_history.db")), 0o600);
    }

    // History written by an older ostt is tightened the next time it is used.
    #[test]
    fn existing_history_is_tightened() {
        let home = tempfile::tempdir().unwrap();
        let data = home.path().join("ostt");
        HistoryManager::new(&data)
            .unwrap()
            .save_transcription("old words")
            .unwrap();
        let database = data.join("transcription_history.db");
        let journal = data.join("transcription_history.db-journal");
        std::fs::write(&journal, b"").unwrap();
        for path in [&data, &database, &journal] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let entries = HistoryManager::new(&data)
            .unwrap()
            .get_all_transcriptions()
            .unwrap();
        assert_eq!(entries[0].text, "old words");
        assert_eq!(mode(&data), 0o700);
        assert_eq!(mode(&database), 0o600);
        assert_eq!(mode(&journal), 0o600);
    }
}
