use super::{LibraryDatabase, Mutex, State};
use rusqlite::OptionalExtension;
use std::collections::HashSet;

impl LibraryDatabase {
    pub(super) fn ignored_track_ids(&self) -> Result<HashSet<String>, String> {
        let mut query = self
            .connection
            .prepare("SELECT track_id FROM ignored_tracks")
            .map_err(|error| error.to_string())?;
        let rows = query
            .query_map([], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<HashSet<String>>>()
            .map_err(|error| error.to_string())
    }

    pub(super) fn is_track_ignored(&self, id: &str) -> Result<bool, String> {
        self.connection
            .query_row(
                "SELECT 1 FROM ignored_tracks WHERE track_id = ?1",
                [id],
                |_| Ok(()),
            )
            .optional()
            .map(|result| result.is_some())
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub(super) fn set_ignored_tracks(
    track_ids: Vec<String>,
    ignored: bool,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<HashSet<String>, String> {
    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.")?;
    let transaction = library
        .connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let sql = if ignored {
        "INSERT OR IGNORE INTO ignored_tracks (track_id) SELECT id FROM tracks WHERE id = ?1"
    } else {
        "DELETE FROM ignored_tracks WHERE track_id = ?1"
    };
    {
        let mut statement = transaction
            .prepare(sql)
            .map_err(|error| error.to_string())?;
        for id in track_ids {
            statement.execute([id]).map_err(|error| error.to_string())?;
        }
    }
    transaction.commit().map_err(|error| error.to_string())?;
    library.ignored_track_ids()
}
