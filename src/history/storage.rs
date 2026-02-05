//! Transcription history storage and retrieval using SQLite.
//!
//! Manages persistent storage of all transcriptions with timestamps,
//! and provides querying capabilities for the history viewer.

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

        let mut statement = connection.prepare(
            "SELECT id, text, created_at FROM transcriptions ORDER BY created_at DESC",
        )?;

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

        let mut statement = connection
            .prepare("SELECT id, text, created_at FROM transcriptions WHERE id = ?1")?;

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
