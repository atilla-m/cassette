use super::{LibraryDatabase, TrackLyrics, MAX_LYRICS_BYTES};
use rusqlite::{params, OptionalExtension};
use std::sync::Mutex;
use tauri::State;

impl LibraryDatabase {
    pub(super) fn edited_lyrics(&self, track_id: &str) -> Result<Option<TrackLyrics>, String> {
        let edit = self
            .connection
            .query_row(
                "SELECT kind, body FROM track_lyrics_edits WHERE track_id = ?1",
                [track_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| format!("Could not load edited lyrics: {error}"))?;
        let Some((kind, text)) = edit else {
            return Ok(None);
        };
        Ok(Some(TrackLyrics {
            path: String::new(),
            kind,
            text,
            source: "edited".into(),
            fetched_at: None,
            track_path: Some(track_id.into()),
            selected_track_name: None,
            selected_artist_name: None,
            selected_album_name: None,
            offset_seconds: self.lyrics_settings(track_id)?.offset_seconds,
        }))
    }
}

#[tauri::command]
pub(super) fn save_edited_lyrics(
    track_path: String,
    kind: String,
    text: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<TrackLyrics, String> {
    if !matches!(kind.as_str(), "plain" | "synced")
        || text.trim().is_empty()
        || text.len() as u64 > MAX_LYRICS_BYTES
        || text.contains('\0')
    {
        return Err("Enter plain or synced lyrics under 1 MB.".into());
    }
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.")?;
    if library.track_by_id(&track_path)?.is_none() {
        return Err("This track is no longer in the library.".into());
    }
    library
        .connection
        .execute(
            "INSERT INTO track_lyrics_edits (track_id, kind, body) VALUES (?1, ?2, ?3)
             ON CONFLICT(track_id) DO UPDATE SET kind = excluded.kind, body = excluded.body",
            params![track_path, kind, text],
        )
        .map_err(|error| format!("Could not save edited lyrics: {error}"))?;
    library
        .edited_lyrics(&track_path)?
        .ok_or_else(|| "The saved lyrics could not be loaded.".into())
}

#[tauri::command]
pub(super) fn restore_original_lyrics(
    track_path: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<(), String> {
    library
        .lock()
        .map_err(|_| "Library cache is unavailable.")?
        .connection
        .execute(
            "DELETE FROM track_lyrics_edits WHERE track_id = ?1",
            [track_path],
        )
        .map_err(|error| format!("Could not restore original lyrics: {error}"))?;
    Ok(())
}
