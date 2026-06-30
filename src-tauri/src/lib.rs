use gst::prelude::*;
use gstreamer as gst;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::Picture;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

mod mpris;

use mpris::{MprisState, MprisTrack};

const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "ogg", "opus", "wav", "m4a"];
const COVER_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const PREFERRED_COVER_NAMES: &[&str] = &["cover", "folder", "front", "album"];
const MAX_FOLDER_COVER_BYTES: u64 = 25 * 1024 * 1024;
const MAX_LYRICS_BYTES: u64 = 1024 * 1024;
const GENRE_SCOPE_ALBUM: &str = "album";
const GENRE_SCOPE_ARTIST: &str = "artist";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Track {
    id: String,
    file_path: String,
    file_name: String,
    extension: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    genres: Vec<String>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    year: Option<u16>,
    duration_seconds: Option<u32>,
    modified_time: Option<i64>,
    file_size: Option<i64>,
    scanned_at: Option<i64>,
    cover_art_path: Option<String>,
    lyrics_path: Option<String>,
    lyrics_kind: Option<String>,
    is_favorite: bool,
    play_count: i64,
    last_played_at: Option<i64>,
}

#[derive(Debug, Default)]
struct TrackMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    genres: Vec<String>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    year: Option<u16>,
    duration_seconds: Option<u32>,
    embedded_cover_art: Option<EmbeddedCoverArt>,
}

#[derive(Debug)]
struct EmbeddedCoverArt {
    extension: &'static str,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct CachedCoverArt {
    path: String,
    priority: CoverArtPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CoverArtPriority {
    FolderFallback,
    FolderNamed,
    Embedded,
}

#[derive(Debug)]
struct FolderCoverArt {
    path: PathBuf,
    extension: &'static str,
    priority: CoverArtPriority,
}

#[derive(Debug)]
struct LyricsFile {
    path: PathBuf,
    kind: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackLyrics {
    path: String,
    kind: String,
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoLyricsResult {
    status: String,
    lyrics: Option<TrackLyrics>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibLyrics {
    track_name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
    duration: Option<f64>,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Debug)]
struct FoundLyrics {
    kind: &'static str,
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackStatus {
    file_path: Option<String>,
    is_playing: bool,
    has_ended: bool,
    position_seconds: u64,
    duration_seconds: Option<u64>,
    volume: f64,
}

#[derive(Debug, Default)]
struct PlaybackState {
    playbin: Option<gst::Element>,
    current_path: Option<String>,
    is_playing: bool,
    has_ended: bool,
}

#[derive(Debug, Default)]
struct GenreAssignmentMaps {
    albums: HashMap<String, Vec<String>>,
    artists: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryCache {
    tracks: Vec<Track>,
    playlists: Vec<Playlist>,
    last_scanned_folder: Option<String>,
    last_scanned_at: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Playlist {
    id: String,
    name: String,
    created_at: i64,
    updated_at: i64,
    track_ids: Vec<String>,
}

struct LibraryDatabase {
    connection: Connection,
}

#[derive(Debug, Clone, Copy, Default)]
struct PlaybackHistory {
    play_count: i64,
    last_played_at: Option<i64>,
}

#[tauri::command]
fn get_library_cache(library: State<'_, Mutex<LibraryDatabase>>) -> Result<LibraryCache, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.load_cache()
}

#[tauri::command]
fn scan_library(
    root: String,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Vec<Track>, String> {
    let root_path = PathBuf::from(root);

    if !root_path.exists() {
        return Err("Selected folder does not exist.".into());
    }

    if !root_path.is_dir() {
        return Err("Selected path is not a folder.".into());
    }

    let mut tracks = Vec::new();
    let mut album_art_paths = HashMap::<String, CachedCoverArt>::new();
    let cover_art_dir = app
        .path()
        .app_data_dir()
        .ok()
        .map(|path| path.join("cover-art"));
    let scanned_at = unix_timestamp();
    scan_directory(
        &root_path,
        scanned_at,
        cover_art_dir.as_deref(),
        &mut album_art_paths,
        &mut tracks,
    )?;
    tracks.sort_by(|left, right| left.file_path.cmp(&right.file_path));

    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.replace_library(&root_path, &mut tracks, scanned_at)?;

    Ok(tracks)
}

#[tauri::command]
fn toggle_track_favorite(
    id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<bool, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.toggle_favorite(&id)
}

#[tauri::command]
fn record_track_play(
    id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Track, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.record_play(&id)
}

#[tauri::command]
fn set_album_genres(
    album_id: String,
    genres: Vec<String>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Vec<Track>, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.set_genres(GENRE_SCOPE_ALBUM, &album_id, genres)
}

#[tauri::command]
fn set_artist_genres(
    artist_name: String,
    genres: Vec<String>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Vec<Track>, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    let artist_key = artist_key_for_name(&artist_name);

    library.set_genres(GENRE_SCOPE_ARTIST, &artist_key, genres)
}

#[tauri::command]
fn create_playlist(
    name: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Playlist, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.create_playlist(&name)
}

#[tauri::command]
fn rename_playlist(
    playlist_id: String,
    name: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Playlist, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.rename_playlist(&playlist_id, &name)
}

#[tauri::command]
fn delete_playlist(
    playlist_id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<(), String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.delete_playlist(&playlist_id)
}

#[tauri::command]
fn add_track_to_playlist(
    playlist_id: String,
    track_id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Playlist, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.add_track_to_playlist(&playlist_id, &track_id)
}

#[tauri::command]
fn remove_track_from_playlist(
    playlist_id: String,
    track_id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Playlist, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.remove_track_from_playlist(&playlist_id, &track_id)
}

#[tauri::command]
fn move_playlist_track(
    playlist_id: String,
    track_id: String,
    direction: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Playlist, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.move_playlist_track(&playlist_id, &track_id, &direction)
}

#[tauri::command]
fn read_track_lyrics(
    track_path: String,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Option<TrackLyrics>, String> {
    let track = library
        .lock()
        .ok()
        .and_then(|library| library.track_by_id(&track_path).ok().flatten());
    let track_path = track
        .as_ref()
        .map(|track| PathBuf::from(&track.file_path))
        .unwrap_or_else(|| PathBuf::from(&track_path));
    let track_title = track
        .as_ref()
        .map(|track| track.title.clone())
        .unwrap_or_else(|| title_from_path(&track_path));
    let lyrics_file = track
        .as_ref()
        .and_then(cached_lyrics_file)
        .or_else(|| lyrics_file_for_track(&track_path, &track_title))
        .or_else(|| {
            track
                .as_ref()
                .and_then(|track| app_lyrics_file_for_track(&app, track))
        });

    Ok(lyrics_file.and_then(read_lyrics_file))
}

#[tauri::command]
fn auto_find_track_lyrics(
    track_path: String,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<AutoLyricsResult, String> {
    let track = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?
        .track_by_id(&track_path)?
        .ok_or_else(|| "Track is not in the library cache.".to_owned())?;
    let existing_lyrics = cached_lyrics_file(&track)
        .or_else(|| lyrics_file_for_track(Path::new(&track.file_path), &track.title))
        .or_else(|| app_lyrics_file_for_track(&app, &track))
        .and_then(read_lyrics_file);

    if let Some(lyrics) = existing_lyrics {
        return Ok(AutoLyricsResult {
            status: "existing".to_owned(),
            lyrics: Some(lyrics),
        });
    }

    let Some(found_lyrics) = find_lrclib_lyrics(&track)? else {
        return Ok(AutoLyricsResult {
            status: "not_found".to_owned(),
            lyrics: None,
        });
    };

    let saved_lyrics = save_app_lyrics(&app, &track, found_lyrics)?;
    let status = if saved_lyrics.kind == "synced" {
        "synced"
    } else {
        "plain"
    };

    Ok(AutoLyricsResult {
        status: status.to_owned(),
        lyrics: Some(saved_lyrics),
    })
}

impl LibraryDatabase {
    fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create app data directory: {error}"))?;
        }

        let connection = Connection::open(path)
            .map_err(|error| format!("Could not open library cache: {error}"))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|error| format!("Could not configure library cache: {error}"))?;
        let database = Self { connection };
        database
            .migrate()
            .map_err(|error| format!("Could not initialize library cache: {error}"))?;

        Ok(database)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tracks (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                album_artist TEXT,
                genres TEXT NOT NULL DEFAULT '[]',
                track_number INTEGER,
                disc_number INTEGER,
                year INTEGER,
                duration_seconds INTEGER,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                extension TEXT NOT NULL,
                modified_time INTEGER,
                file_size INTEGER,
                scanned_at INTEGER NOT NULL,
                cover_art_path TEXT,
                lyrics_path TEXT,
                lyrics_kind TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                play_count INTEGER NOT NULL DEFAULT 0,
                last_played_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS library_meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS genre_assignments (
                scope TEXT NOT NULL,
                entity_key TEXT NOT NULL,
                genres TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (scope, entity_key)
            );

            CREATE TABLE IF NOT EXISTS playlists (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
                playlist_id TEXT NOT NULL,
                track_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                added_at INTEGER NOT NULL,
                PRIMARY KEY (playlist_id, track_id),
                FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
            );
            ",
        )?;

        if !self.has_column("tracks", "cover_art_path")? {
            self.connection
                .execute("ALTER TABLE tracks ADD COLUMN cover_art_path TEXT", [])?;
        }

        if !self.has_column("tracks", "lyrics_path")? {
            self.connection
                .execute("ALTER TABLE tracks ADD COLUMN lyrics_path TEXT", [])?;
        }

        if !self.has_column("tracks", "lyrics_kind")? {
            self.connection
                .execute("ALTER TABLE tracks ADD COLUMN lyrics_kind TEXT", [])?;
        }

        if !self.has_column("tracks", "is_favorite")? {
            self.connection.execute(
                "ALTER TABLE tracks ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        if !self.has_column("tracks", "genres")? {
            self.connection.execute(
                "ALTER TABLE tracks ADD COLUMN genres TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }

        if !self.has_column("tracks", "play_count")? {
            self.connection.execute(
                "ALTER TABLE tracks ADD COLUMN play_count INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        if !self.has_column("tracks", "last_played_at")? {
            self.connection
                .execute("ALTER TABLE tracks ADD COLUMN last_played_at INTEGER", [])?;
        }

        Ok(())
    }

    fn has_column(&self, table: &str, column: &str) -> rusqlite::Result<bool> {
        let mut statement = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;

        for name in columns {
            if name? == column {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn load_cache(&self) -> Result<LibraryCache, String> {
        let mut statement = self
            .connection
            .prepare(
                "
                SELECT
                    id,
                    title,
                    artist,
                    album,
                    album_artist,
                    track_number,
                    disc_number,
                    year,
                    duration_seconds,
                    file_path,
                    file_name,
                    extension,
                    modified_time,
                    file_size,
                    scanned_at,
                    cover_art_path,
                    lyrics_path,
                    lyrics_kind,
                    is_favorite,
                    genres,
                    play_count,
                    last_played_at
                FROM tracks
                ORDER BY file_path
                ",
            )
            .map_err(|error| format!("Could not read library cache: {error}"))?;

        let mut tracks = statement
            .query_map([], row_to_track)
            .map_err(|error| format!("Could not read library cache: {error}"))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read cached tracks: {error}"))?;
        let genre_assignments = self.genre_assignments()?;
        apply_genre_assignments(&mut tracks, &genre_assignments);

        Ok(LibraryCache {
            tracks,
            playlists: self.playlists()?,
            last_scanned_folder: self.meta_value("last_scanned_folder")?,
            last_scanned_at: self
                .meta_value("last_scanned_at")?
                .and_then(|value| value.parse::<i64>().ok()),
        })
    }

    fn replace_library(
        &mut self,
        root_path: &Path,
        tracks: &mut [Track],
        scanned_at: i64,
    ) -> Result<(), String> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| format!("Could not update library cache: {error}"))?;
        let favorite_track_ids = favorite_track_ids(&transaction)
            .map_err(|error| format!("Could not read favorite tracks: {error}"))?;
        let playback_history = playback_history_by_id(&transaction)
            .map_err(|error| format!("Could not read playback history: {error}"))?;

        transaction
            .execute("DELETE FROM tracks", [])
            .map_err(|error| format!("Could not clear library cache: {error}"))?;

        {
            let mut statement = transaction
                .prepare(
                    "
                    INSERT INTO tracks (
                        id,
                        title,
                        artist,
                        album,
                        album_artist,
                        genres,
                        track_number,
                        disc_number,
                        year,
                        duration_seconds,
                        file_path,
                        file_name,
                        extension,
                        modified_time,
                        file_size,
                        scanned_at,
                        cover_art_path,
                        lyrics_path,
                        lyrics_kind,
                        is_favorite,
                        play_count,
                        last_played_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
                    ",
                )
                .map_err(|error| format!("Could not prepare library cache update: {error}"))?;

            for track in tracks.iter_mut() {
                track.is_favorite = track.is_favorite || favorite_track_ids.contains(&track.id);
                if let Some(history) = playback_history.get(&track.id) {
                    track.play_count = history.play_count;
                    track.last_played_at = history.last_played_at;
                }
                let genres_json =
                    serde_json::to_string(&track.genres).unwrap_or_else(|_| "[]".to_owned());
                statement
                    .execute(params![
                        &track.id,
                        &track.title,
                        &track.artist,
                        &track.album,
                        &track.album_artist,
                        &genres_json,
                        track.track_number,
                        track.disc_number,
                        track.year,
                        track.duration_seconds,
                        &track.file_path,
                        &track.file_name,
                        &track.extension,
                        track.modified_time,
                        track.file_size,
                        track.scanned_at,
                        &track.cover_art_path,
                        &track.lyrics_path,
                        &track.lyrics_kind,
                        track.is_favorite,
                        track.play_count,
                        track.last_played_at,
                    ])
                    .map_err(|error| format!("Could not cache scanned track: {error}"))?;
            }
        }

        upsert_meta(
            &transaction,
            "last_scanned_folder",
            &root_path.to_string_lossy(),
        )
        .map_err(|error| format!("Could not cache scanned folder: {error}"))?;
        upsert_meta(&transaction, "last_scanned_at", &scanned_at.to_string())
            .map_err(|error| format!("Could not cache scan time: {error}"))?;

        transaction
            .commit()
            .map_err(|error| format!("Could not save library cache: {error}"))?;

        let genre_assignments = self.genre_assignments()?;
        apply_genre_assignments(tracks, &genre_assignments);

        Ok(())
    }

    fn meta_value(&self, key: &str) -> Result<Option<String>, String> {
        self.connection
            .query_row(
                "SELECT value FROM library_meta WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Could not read library metadata: {error}"))
    }

    fn toggle_favorite(&self, id: &str) -> Result<bool, String> {
        let current = self
            .connection
            .query_row(
                "SELECT is_favorite FROM tracks WHERE id = ?1",
                [id],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(|error| format!("Could not read favorite state: {error}"))?
            .ok_or_else(|| "Track is not in the library cache.".to_owned())?;
        let next = !current;

        self.connection
            .execute(
                "UPDATE tracks SET is_favorite = ?2 WHERE id = ?1",
                params![id, next],
            )
            .map_err(|error| format!("Could not update favorite state: {error}"))?;

        Ok(next)
    }

    fn track_by_id(&self, id: &str) -> Result<Option<Track>, String> {
        let mut statement = self
            .connection
            .prepare(
                "
                SELECT
                    id,
                    title,
                    artist,
                    album,
                    album_artist,
                    track_number,
                    disc_number,
                    year,
                    duration_seconds,
                    file_path,
                    file_name,
                    extension,
                    modified_time,
                    file_size,
                    scanned_at,
                    cover_art_path,
                    lyrics_path,
                    lyrics_kind,
                    is_favorite,
                    genres,
                    play_count,
                    last_played_at
                FROM tracks
                WHERE id = ?1
                ",
            )
            .map_err(|error| format!("Could not prepare track lookup: {error}"))?;
        let mut track = statement
            .query_row([id], row_to_track)
            .optional()
            .map_err(|error| format!("Could not read cached track: {error}"))?;

        if let Some(track) = track.as_mut() {
            let genre_assignments = self.genre_assignments()?;
            apply_genre_assignments(std::slice::from_mut(track), &genre_assignments);
        }

        Ok(track)
    }

    fn record_play(&self, id: &str) -> Result<Track, String> {
        let played_at = unix_timestamp();
        let updated = self
            .connection
            .execute(
                "
                UPDATE tracks
                SET play_count = play_count + 1,
                    last_played_at = ?2
                WHERE id = ?1
                ",
                params![id, played_at],
            )
            .map_err(|error| format!("Could not update playback history: {error}"))?;

        if updated == 0 {
            return Err("Track is not in the library cache.".to_owned());
        }

        self.track_by_id(id)?
            .ok_or_else(|| "Track is not in the library cache.".to_owned())
    }

    fn set_genres(
        &self,
        scope: &str,
        entity_key: &str,
        genres: Vec<String>,
    ) -> Result<Vec<Track>, String> {
        let genres = normalize_manual_genres(genres);

        if genres.is_empty() {
            self.connection
                .execute(
                    "DELETE FROM genre_assignments WHERE scope = ?1 AND entity_key = ?2",
                    params![scope, entity_key],
                )
                .map_err(|error| format!("Could not clear genre assignment: {error}"))?;
        } else {
            let genres_json = serde_json::to_string(&genres).unwrap_or_else(|_| "[]".to_owned());
            self.connection
                .execute(
                    "
                    INSERT INTO genre_assignments (scope, entity_key, genres, updated_at)
                    VALUES (?1, ?2, ?3, ?4)
                    ON CONFLICT(scope, entity_key) DO UPDATE SET
                        genres = excluded.genres,
                        updated_at = excluded.updated_at
                    ",
                    params![scope, entity_key, genres_json, unix_timestamp()],
                )
                .map_err(|error| format!("Could not save genre assignment: {error}"))?;
        }

        self.load_cache().map(|cache| cache.tracks)
    }

    fn playlists(&self) -> Result<Vec<Playlist>, String> {
        let mut statement = self
            .connection
            .prepare(
                "
                SELECT id, name, created_at, updated_at
                FROM playlists
                ORDER BY created_at ASC
                ",
            )
            .map_err(|error| format!("Could not read playlists: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(Playlist {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    track_ids: Vec::new(),
                })
            })
            .map_err(|error| format!("Could not read playlists: {error}"))?;
        let mut playlists = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read playlist rows: {error}"))?;

        for playlist in &mut playlists {
            playlist.track_ids = self.playlist_track_ids(&playlist.id)?;
        }

        Ok(playlists)
    }

    fn playlist_by_id(&self, playlist_id: &str) -> Result<Playlist, String> {
        let mut playlist = self
            .connection
            .query_row(
                "
                SELECT id, name, created_at, updated_at
                FROM playlists
                WHERE id = ?1
                ",
                [playlist_id],
                |row| {
                    Ok(Playlist {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        track_ids: Vec::new(),
                    })
                },
            )
            .optional()
            .map_err(|error| format!("Could not read playlist: {error}"))?
            .ok_or_else(|| "Playlist does not exist.".to_owned())?;
        playlist.track_ids = self.playlist_track_ids(playlist_id)?;

        Ok(playlist)
    }

    fn playlist_track_ids(&self, playlist_id: &str) -> Result<Vec<String>, String> {
        let mut statement = self
            .connection
            .prepare(
                "
                SELECT track_id
                FROM playlist_tracks
                WHERE playlist_id = ?1
                ORDER BY position ASC
                ",
            )
            .map_err(|error| format!("Could not read playlist tracks: {error}"))?;

        let track_ids = statement
            .query_map([playlist_id], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Could not read playlist tracks: {error}"))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read playlist track rows: {error}"))?;

        Ok(track_ids)
    }

    fn create_playlist(&self, name: &str) -> Result<Playlist, String> {
        let name = name.trim();

        if name.is_empty() {
            return Err("Playlist name is required.".to_owned());
        }

        if self.playlist_name_exists(name)? {
            return Err("A playlist with this name already exists.".to_owned());
        }

        let now = unix_timestamp();
        let id = format!("playlist-{}", unique_timestamp_nanos());

        self.connection
            .execute(
                "
                INSERT INTO playlists (id, name, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?3)
                ",
                params![id, name, now],
            )
            .map_err(|error| format!("Could not create playlist: {error}"))?;

        self.playlist_by_id(&id)
    }

    fn rename_playlist(&self, playlist_id: &str, name: &str) -> Result<Playlist, String> {
        self.playlist_by_id(playlist_id)?;
        let name = name.trim();

        if name.is_empty() {
            return Err("Playlist name is required.".to_owned());
        }

        if self.playlist_name_exists_for_other_playlist(playlist_id, name)? {
            return Err("A playlist with this name already exists.".to_owned());
        }

        let now = unix_timestamp();
        let changed = self
            .connection
            .execute(
                "
                UPDATE playlists
                SET name = ?2,
                    updated_at = ?3
                WHERE id = ?1
                ",
                params![playlist_id, name, now],
            )
            .map_err(|error| format!("Could not rename playlist: {error}"))?;

        if changed == 0 {
            return Err("Playlist does not exist.".to_owned());
        }

        self.playlist_by_id(playlist_id)
    }

    fn delete_playlist(&self, playlist_id: &str) -> Result<(), String> {
        self.playlist_by_id(playlist_id)?;

        let changed = self
            .connection
            .execute("DELETE FROM playlists WHERE id = ?1", [playlist_id])
            .map_err(|error| format!("Could not delete playlist: {error}"))?;

        if changed == 0 {
            return Err("Playlist does not exist.".to_owned());
        }

        Ok(())
    }

    fn add_track_to_playlist(&self, playlist_id: &str, track_id: &str) -> Result<Playlist, String> {
        let playlist = self.playlist_by_id(playlist_id)?;

        if self.track_by_id(track_id)?.is_none() {
            return Err("Track is not in the library cache.".to_owned());
        }

        if playlist.track_ids.iter().any(|id| id == track_id) {
            return Err("Track is already in this playlist.".to_owned());
        }

        let now = unix_timestamp();
        let next_position = self
            .connection
            .query_row(
                "
                SELECT COALESCE(MAX(position), -1) + 1
                FROM playlist_tracks
                WHERE playlist_id = ?1
                ",
                [playlist_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| format!("Could not read playlist position: {error}"))?;
        let changed = self
            .connection
            .execute(
                "
                INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position, added_at)
                VALUES (?1, ?2, ?3, ?4)
                ",
                params![playlist_id, track_id, next_position, now],
            )
            .map_err(|error| format!("Could not add track to playlist: {error}"))?;

        if changed > 0 {
            self.touch_playlist(playlist_id, now)?;
        }

        self.playlist_by_id(playlist_id)
    }

    fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        track_id: &str,
    ) -> Result<Playlist, String> {
        self.playlist_by_id(playlist_id)?;

        let now = unix_timestamp();
        let changed = self
            .connection
            .execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                params![playlist_id, track_id],
            )
            .map_err(|error| format!("Could not remove track from playlist: {error}"))?;

        if changed > 0 {
            self.compact_playlist_positions(playlist_id)?;
            self.touch_playlist(playlist_id, now)?;
        }

        self.playlist_by_id(playlist_id)
    }

    fn move_playlist_track(
        &self,
        playlist_id: &str,
        track_id: &str,
        direction: &str,
    ) -> Result<Playlist, String> {
        self.playlist_by_id(playlist_id)?;
        self.compact_playlist_positions(playlist_id)?;

        let direction = direction.trim().to_lowercase();
        let current_position = self
            .connection
            .query_row(
                "
                SELECT position
                FROM playlist_tracks
                WHERE playlist_id = ?1 AND track_id = ?2
                ",
                params![playlist_id, track_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| format!("Could not read playlist track: {error}"))?
            .ok_or_else(|| "Track is not in this playlist.".to_owned())?;

        let target_position = match direction.as_str() {
            "up" => {
                if current_position == 0 {
                    return self.playlist_by_id(playlist_id);
                }

                current_position - 1
            }
            "down" => current_position + 1,
            _ => return Err("Move direction must be up or down.".to_owned()),
        };

        let Some(target_track_id) = self
            .connection
            .query_row(
                "
                SELECT track_id
                FROM playlist_tracks
                WHERE playlist_id = ?1 AND position = ?2
                ",
                params![playlist_id, target_position],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Could not read playlist track: {error}"))?
        else {
            return self.playlist_by_id(playlist_id);
        };

        self.connection
            .execute(
                "
                UPDATE playlist_tracks
                SET position = CASE track_id
                    WHEN ?2 THEN ?4
                    WHEN ?3 THEN ?5
                    ELSE position
                END
                WHERE playlist_id = ?1 AND track_id IN (?2, ?3)
                ",
                params![
                    playlist_id,
                    track_id,
                    target_track_id,
                    target_position,
                    current_position,
                ],
            )
            .map_err(|error| format!("Could not move playlist track: {error}"))?;
        self.touch_playlist(playlist_id, unix_timestamp())?;

        self.playlist_by_id(playlist_id)
    }

    fn touch_playlist(&self, playlist_id: &str, updated_at: i64) -> Result<(), String> {
        let changed = self
            .connection
            .execute(
                "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
                params![playlist_id, updated_at],
            )
            .map_err(|error| format!("Could not update playlist: {error}"))?;

        if changed == 0 {
            return Err("Playlist does not exist.".to_owned());
        }

        Ok(())
    }

    fn playlist_name_exists(&self, name: &str) -> Result<bool, String> {
        self.playlist_name_exists_for_other_playlist("", name)
    }

    fn playlist_name_exists_for_other_playlist(
        &self,
        playlist_id: &str,
        name: &str,
    ) -> Result<bool, String> {
        let normalized_name = normalize_playlist_name(name);
        let mut statement = self
            .connection
            .prepare("SELECT id, name FROM playlists")
            .map_err(|error| format!("Could not read playlists: {error}"))?;
        let existing_playlists = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| format!("Could not read playlists: {error}"))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read playlist rows: {error}"))?;

        Ok(existing_playlists
            .iter()
            .any(|(existing_id, existing_name)| {
                existing_id != playlist_id && normalize_playlist_name(existing_name) == normalized_name
            }))
    }

    fn compact_playlist_positions(&self, playlist_id: &str) -> Result<(), String> {
        let track_ids = self.playlist_track_ids(playlist_id)?;

        for (position, track_id) in track_ids.iter().enumerate() {
            self.connection
                .execute(
                    "
                    UPDATE playlist_tracks
                    SET position = ?3
                    WHERE playlist_id = ?1 AND track_id = ?2
                    ",
                    params![playlist_id, track_id, position as i64],
                )
                .map_err(|error| format!("Could not update playlist order: {error}"))?;
        }

        Ok(())
    }

    fn genre_assignments(&self) -> Result<GenreAssignmentMaps, String> {
        let mut statement = self
            .connection
            .prepare("SELECT scope, entity_key, genres FROM genre_assignments")
            .map_err(|error| format!("Could not read genre assignments: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| format!("Could not read genre assignments: {error}"))?;
        let mut assignments = GenreAssignmentMaps::default();

        for row in rows {
            let (scope, entity_key, genres_json) =
                row.map_err(|error| format!("Could not read genre assignment: {error}"))?;
            let genres = parse_assigned_genres(genres_json);

            if genres.is_empty() {
                continue;
            }

            match scope.as_str() {
                GENRE_SCOPE_ALBUM => {
                    assignments.albums.insert(entity_key, genres);
                }
                GENRE_SCOPE_ARTIST => {
                    assignments.artists.insert(entity_key, genres);
                }
                _ => {}
            }
        }

        Ok(assignments)
    }
}

#[tauri::command]
fn play_track(
    file_path: String,
    playback: State<'_, Mutex<PlaybackState>>,
    library: State<'_, Mutex<LibraryDatabase>>,
    mpris: State<'_, MprisState>,
) -> Result<PlaybackStatus, String> {
    let status = {
        let mut playback = playback
            .lock()
            .map_err(|_| "Playback state is unavailable.".to_owned())?;

        playback.play(&file_path)?
    };
    let track = library
        .lock()
        .ok()
        .and_then(|library| library.track_by_id(&file_path).ok().flatten())
        .map(|track| MprisTrack::from(&track));
    mpris.update_track(track, status.is_playing);

    Ok(status)
}

#[tauri::command]
fn pause_playback(
    playback: State<'_, Mutex<PlaybackState>>,
    mpris: State<'_, MprisState>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    let status = playback.pause()?;
    mpris.update_playback(status.is_playing, status.position_seconds, status.volume);

    Ok(status)
}

#[tauri::command]
fn resume_playback(
    playback: State<'_, Mutex<PlaybackState>>,
    mpris: State<'_, MprisState>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    let status = playback.resume()?;
    mpris.update_playback(status.is_playing, status.position_seconds, status.volume);

    Ok(status)
}

#[tauri::command]
fn get_playback_status(
    playback: State<'_, Mutex<PlaybackState>>,
    mpris: State<'_, MprisState>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    playback.refresh()?;
    let status = playback.status();
    mpris.update_playback(status.is_playing, status.position_seconds, status.volume);

    Ok(status)
}

#[tauri::command]
fn seek_playback(
    position_seconds: f64,
    playback: State<'_, Mutex<PlaybackState>>,
    mpris: State<'_, MprisState>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    let status = playback.seek(position_seconds)?;
    mpris.update_playback(status.is_playing, status.position_seconds, status.volume);

    Ok(status)
}

#[tauri::command]
fn set_playback_volume(
    volume: f64,
    playback: State<'_, Mutex<PlaybackState>>,
    mpris: State<'_, MprisState>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    let status = playback.set_volume(volume)?;
    mpris.update_playback(status.is_playing, status.position_seconds, status.volume);

    Ok(status)
}

impl PlaybackState {
    fn play(&mut self, file_path: &str) -> Result<PlaybackStatus, String> {
        let path = PathBuf::from(file_path);

        if !path.exists() {
            return Err("Selected track no longer exists.".to_owned());
        }

        if !path.is_file() {
            return Err("Selected track is not a file.".to_owned());
        }

        let uri = gst::glib::filename_to_uri(&path, None)
            .map_err(|error| format!("Could not prepare track for playback: {error}"))?;

        {
            let playbin = self.playbin()?;
            set_gst_state(playbin, gst::State::Null)?;
            drain_playback_bus(playbin);
            playbin.set_property("uri", uri.as_str());
            set_gst_state(playbin, gst::State::Playing)?;
            check_for_playback_error(playbin)?;
        }

        self.current_path = Some(file_path.to_owned());
        self.is_playing = true;
        self.has_ended = false;

        Ok(self.status())
    }

    fn pause(&mut self) -> Result<PlaybackStatus, String> {
        if let Some(playbin) = self.playbin.as_ref() {
            set_gst_state(playbin, gst::State::Paused)?;
            self.is_playing = false;
        }

        Ok(self.status())
    }

    fn resume(&mut self) -> Result<PlaybackStatus, String> {
        let Some(playbin) = self.playbin.as_ref() else {
            return Ok(self.status());
        };

        set_gst_state(playbin, gst::State::Playing)?;
        self.is_playing = self.current_path.is_some();
        self.has_ended = false;

        Ok(self.status())
    }

    fn refresh(&mut self) -> Result<(), String> {
        let Some(playbin) = self.playbin.clone() else {
            return Ok(());
        };

        while let Some(message) = playbin.bus().and_then(|bus| {
            bus.timed_pop_filtered(
                gst::ClockTime::ZERO,
                &[gst::MessageType::Error, gst::MessageType::Eos],
            )
        }) {
            match message.view() {
                gst::MessageView::Eos(_) => {
                    self.is_playing = false;
                    self.has_ended = true;
                    set_gst_state(&playbin, gst::State::Paused)?;
                }
                gst::MessageView::Error(error) => {
                    self.is_playing = false;
                    return Err(format!("Playback failed: {}", error.error()));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn seek(&mut self, position_seconds: f64) -> Result<PlaybackStatus, String> {
        if !position_seconds.is_finite() {
            return Ok(self.status());
        }

        if self.current_path.is_none() {
            return Ok(self.status());
        }

        let Some(playbin) = self.playbin.as_ref() else {
            return Ok(self.status());
        };

        let Some(duration) = playbin
            .query_duration::<gst::ClockTime>()
            .filter(|duration| *duration > gst::ClockTime::ZERO)
        else {
            return Ok(self.status());
        };
        let clamped_position_seconds = position_seconds.clamp(0.0, duration.seconds_f64());
        let seek_position = gst::ClockTime::try_from_seconds_f64(clamped_position_seconds)
            .map_err(|error| format!("Could not prepare seek position: {error}"))?;

        playbin
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                seek_position,
            )
            .map_err(|error| format!("Could not seek track: {error}"))?;
        self.has_ended = false;

        Ok(self.status())
    }

    fn set_volume(&mut self, volume: f64) -> Result<PlaybackStatus, String> {
        let volume = volume.clamp(0.0, 1.0);

        if let Some(playbin) = self.playbin.as_ref() {
            playbin.set_property("volume", volume);
        }

        Ok(self.status())
    }

    fn playbin(&mut self) -> Result<&gst::Element, String> {
        if self.playbin.is_none() {
            gst::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;

            let playbin = gst::ElementFactory::make_with_name("playbin", Some("cassette-playbin"))
                .map_err(|_| {
                    "GStreamer playbin is unavailable. Check that GStreamer plugins are installed."
                        .to_owned()
                })?;

            self.playbin = Some(playbin);
        }

        self.playbin
            .as_ref()
            .ok_or_else(|| "Playback backend is unavailable.".to_owned())
    }

    fn status(&self) -> PlaybackStatus {
        let (position_seconds, duration_seconds, volume) = self
            .playbin
            .as_ref()
            .map(playback_details)
            .unwrap_or((0, None, 1.0));

        PlaybackStatus {
            file_path: self.current_path.clone(),
            is_playing: self.is_playing,
            has_ended: self.has_ended,
            position_seconds,
            duration_seconds,
            volume,
        }
    }
}

fn playback_details(playbin: &gst::Element) -> (u64, Option<u64>, f64) {
    let position_seconds = playbin
        .query_position::<gst::ClockTime>()
        .map(|position| position.seconds())
        .unwrap_or(0);
    let duration_seconds = playbin
        .query_duration::<gst::ClockTime>()
        .map(|duration| duration.seconds())
        .filter(|duration| *duration > 0);
    let volume = playbin.property::<f64>("volume").clamp(0.0, 1.0);

    (position_seconds, duration_seconds, volume)
}

fn drain_playback_bus(playbin: &gst::Element) {
    let Some(bus) = playbin.bus() else {
        return;
    };

    while bus
        .timed_pop_filtered(
            gst::ClockTime::ZERO,
            &[gst::MessageType::Error, gst::MessageType::Eos],
        )
        .is_some()
    {}
}

fn set_gst_state(playbin: &gst::Element, state: gst::State) -> Result<(), String> {
    playbin
        .set_state(state)
        .map(|_| ())
        .map_err(|error| format!("Playback failed: {error}"))
}

fn check_for_playback_error(playbin: &gst::Element) -> Result<(), String> {
    let Some(bus) = playbin.bus() else {
        return Ok(());
    };

    let Some(message) = bus.timed_pop_filtered(
        gst::ClockTime::from_mseconds(150),
        &[gst::MessageType::Error],
    ) else {
        return Ok(());
    };

    match message.view() {
        gst::MessageView::Error(error) => Err(format!("Playback failed: {}", error.error())),
        _ => Ok(()),
    }
}

fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        album: row.get(3)?,
        album_artist: row.get(4)?,
        track_number: row.get(5)?,
        disc_number: row.get(6)?,
        year: row.get(7)?,
        duration_seconds: row.get(8)?,
        file_path: row.get(9)?,
        file_name: row.get(10)?,
        extension: row.get(11)?,
        modified_time: row.get(12)?,
        file_size: row.get(13)?,
        scanned_at: row.get(14)?,
        cover_art_path: row.get(15)?,
        lyrics_path: row.get(16)?,
        lyrics_kind: row.get(17)?,
        is_favorite: row.get(18)?,
        genres: parse_cached_genres(row.get(19)?),
        play_count: row.get(20)?,
        last_played_at: row.get(21)?,
    })
}

impl From<&Track> for MprisTrack {
    fn from(track: &Track) -> Self {
        Self {
            title: track.title.clone(),
            artist: track.artist.clone().or_else(|| track.album_artist.clone()),
            album: track.album.clone(),
            duration_seconds: track.duration_seconds.map(u64::from),
            art_path: track.cover_art_path.clone(),
        }
    }
}

fn parse_cached_genres(value: String) -> Vec<String> {
    let genres = serde_json::from_str::<Vec<String>>(&value)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|genre| clean_text(Some(genre)))
        .collect::<Vec<_>>();

    normalize_genres(genres)
}

fn parse_assigned_genres(value: String) -> Vec<String> {
    let genres = serde_json::from_str::<Vec<String>>(&value).unwrap_or_default();

    normalize_manual_genres(genres)
}

fn normalize_manual_genres(genres: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for genre in genres {
        for candidate in genre.split([',', ';', '\0']) {
            let Some(genre) = clean_text(Some(candidate.to_owned())) else {
                continue;
            };
            let key = genre.to_lowercase();

            if seen.insert(key) {
                normalized.push(genre);
            }
        }
    }

    normalized
}

fn favorite_track_ids(connection: &Connection) -> rusqlite::Result<HashSet<String>> {
    let mut statement = connection.prepare("SELECT id FROM tracks WHERE is_favorite = 1")?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<HashSet<_>>>()?;

    Ok(ids)
}

fn playback_history_by_id(
    connection: &Connection,
) -> rusqlite::Result<HashMap<String, PlaybackHistory>> {
    let mut statement = connection
        .prepare("SELECT id, play_count, last_played_at FROM tracks WHERE play_count > 0")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            PlaybackHistory {
                play_count: row.get(1)?,
                last_played_at: row.get(2)?,
            },
        ))
    })?;

    rows.collect()
}

fn upsert_meta(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "
        INSERT INTO library_meta (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        params![key, value],
    )?;

    Ok(())
}

fn scan_directory(
    directory: &Path,
    scanned_at: i64,
    cover_art_dir: Option<&Path>,
    album_art_paths: &mut HashMap<String, CachedCoverArt>,
    tracks: &mut Vec<Track>,
) -> Result<(), String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };

        let path = entry.path();

        if file_type.is_dir() {
            scan_directory(&path, scanned_at, cover_art_dir, album_art_paths, tracks)?;
        } else if file_type.is_file() && is_supported_audio_file(&path) {
            if let Some((mut track, embedded_cover_art)) = track_from_path(path.clone(), scanned_at)
            {
                let album_art_key = album_art_key_for_track(&track, &path);
                let existing_priority = album_art_paths
                    .get(&album_art_key)
                    .map(|cover_art| cover_art.priority);
                let should_check_cover_art =
                    !matches!(existing_priority, Some(CoverArtPriority::Embedded))
                        && (existing_priority.is_none() || embedded_cover_art.is_some());

                if should_check_cover_art {
                    if let Some(candidate) = cover_art_candidate(
                        cover_art_dir,
                        &album_art_key,
                        &path,
                        embedded_cover_art.as_ref(),
                    ) {
                        let should_update = album_art_paths
                            .get(&album_art_key)
                            .map(|current| candidate.priority > current.priority)
                            .unwrap_or(true);

                        if should_update {
                            for existing_track in tracks.iter_mut() {
                                let existing_path = PathBuf::from(&existing_track.file_path);
                                if album_art_key_for_track(existing_track, &existing_path)
                                    == album_art_key
                                {
                                    existing_track.cover_art_path = Some(candidate.path.clone());
                                }
                            }

                            album_art_paths.insert(album_art_key.clone(), candidate);
                        }
                    }
                }

                if let Some(cover_art) = album_art_paths.get(&album_art_key) {
                    track.cover_art_path = Some(cover_art.path.clone());
                }

                tracks.push(track);
            }
        }
    }

    Ok(())
}

fn is_supported_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

fn track_from_path(path: PathBuf, scanned_at: i64) -> Option<(Track, Option<EmbeddedCoverArt>)> {
    let metadata = read_track_metadata(&path);
    let file_info = file_info(&path);
    let path_string = path.to_string_lossy().into_owned();
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let title = metadata
        .title
        .clone()
        .unwrap_or_else(|| title_from_path(&path));

    let track = Track {
        id: path_string.clone(),
        file_path: path_string,
        file_name,
        extension,
        title,
        artist: metadata.artist,
        album: metadata.album,
        album_artist: metadata.album_artist,
        genres: normalize_genres(metadata.genres),
        track_number: metadata.track_number,
        disc_number: metadata.disc_number,
        year: metadata.year,
        duration_seconds: metadata.duration_seconds,
        modified_time: file_info.modified_time,
        file_size: file_info.file_size,
        scanned_at: Some(scanned_at),
        cover_art_path: None,
        lyrics_path: None,
        lyrics_kind: None,
        is_favorite: false,
        play_count: 0,
        last_played_at: None,
    };
    let lyrics = lyrics_file_for_track(&path, &track.title);
    let track = Track {
        lyrics_path: lyrics
            .as_ref()
            .map(|lyrics| lyrics.path.to_string_lossy().into_owned()),
        lyrics_kind: lyrics.as_ref().map(|lyrics| lyrics.kind.to_owned()),
        ..track
    };

    Some((track, metadata.embedded_cover_art))
}

#[derive(Debug, Default)]
struct FileInfo {
    modified_time: Option<i64>,
    file_size: Option<i64>,
}

fn file_info(path: &Path) -> FileInfo {
    let Ok(metadata) = fs::metadata(path) else {
        return FileInfo::default();
    };

    FileInfo {
        modified_time: metadata.modified().ok().and_then(system_time_to_unix),
        file_size: i64::try_from(metadata.len()).ok(),
    }
}

fn album_key_for_track(track: &Track) -> String {
    let title = track.album.as_deref().unwrap_or("Unknown Album");
    let artist = track
        .album_artist
        .as_deref()
        .or(track.artist.as_deref())
        .unwrap_or("Unknown Artist");

    format!("{}\0{}", artist.to_lowercase(), title.to_lowercase())
}

fn artist_key_for_track(track: &Track) -> String {
    let artist = track
        .artist
        .as_deref()
        .or(track.album_artist.as_deref())
        .unwrap_or("Unknown Artist");

    artist_key_for_name(artist)
}

fn artist_key_for_name(artist: &str) -> String {
    artist.trim().to_lowercase()
}

fn normalize_playlist_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn apply_genre_assignments(tracks: &mut [Track], assignments: &GenreAssignmentMaps) {
    for track in tracks {
        if let Some(genres) = assignments.albums.get(&album_key_for_track(track)) {
            track.genres = genres.clone();
        } else if let Some(genres) = assignments.artists.get(&artist_key_for_track(track)) {
            track.genres = genres.clone();
        }
    }
}

fn album_art_key_for_track(track: &Track, path: &Path) -> String {
    let folder = path
        .parent()
        .map(|parent| parent.to_string_lossy())
        .unwrap_or_default();

    format!("{}\0{}", album_key_for_track(track), folder)
}

fn cover_art_candidate(
    cover_art_dir: Option<&Path>,
    album_art_key: &str,
    track_path: &Path,
    embedded_cover_art: Option<&EmbeddedCoverArt>,
) -> Option<CachedCoverArt> {
    if let Some(path) = embedded_cover_art
        .and_then(|cover_art| save_embedded_album_cover(cover_art_dir, album_art_key, cover_art))
    {
        return Some(CachedCoverArt {
            path,
            priority: CoverArtPriority::Embedded,
        });
    }

    let folder_cover_art = folder_cover_art(track_path)?;
    let priority = folder_cover_art.priority;
    let path = save_folder_album_cover(cover_art_dir, album_art_key, &folder_cover_art)?;

    Some(CachedCoverArt { path, priority })
}

fn save_embedded_album_cover(
    cover_art_dir: Option<&Path>,
    album_art_key: &str,
    cover_art: &EmbeddedCoverArt,
) -> Option<String> {
    let cover_art_dir = cover_art_dir?;
    fs::create_dir_all(cover_art_dir).ok()?;

    let file_name = format!(
        "{:016x}-{:016x}.{}",
        stable_hash(album_art_key),
        stable_hash_bytes(&cover_art.data),
        cover_art.extension
    );
    let cover_art_path = cover_art_dir.join(file_name);
    fs::write(&cover_art_path, &cover_art.data).ok()?;

    Some(cover_art_path.to_string_lossy().into_owned())
}

fn save_folder_album_cover(
    cover_art_dir: Option<&Path>,
    album_art_key: &str,
    cover_art: &FolderCoverArt,
) -> Option<String> {
    let cover_art_dir = cover_art_dir?;
    fs::create_dir_all(cover_art_dir).ok()?;

    let metadata = fs::metadata(&cover_art.path).ok()?;
    let modified_time = metadata
        .modified()
        .ok()
        .and_then(system_time_to_unix)
        .unwrap_or(0);
    let source_key = format!(
        "{}\0{}\0{}\0{}",
        album_art_key,
        cover_art.path.to_string_lossy(),
        metadata.len(),
        modified_time
    );
    let file_name = format!(
        "{:016x}-{:016x}.{}",
        stable_hash(album_art_key),
        stable_hash(&source_key),
        cover_art.extension
    );
    let cover_art_path = cover_art_dir.join(file_name);
    fs::copy(&cover_art.path, &cover_art_path).ok()?;

    Some(cover_art_path.to_string_lossy().into_owned())
}

fn folder_cover_art(track_path: &Path) -> Option<FolderCoverArt> {
    let directory = track_path.parent()?;
    let entries = folder_image_candidates(directory)?;

    for preferred_name in PREFERRED_COVER_NAMES {
        for preferred_extension in COVER_IMAGE_EXTENSIONS {
            if let Some(cover_art) = entries.iter().find_map(|path| {
                let stem_matches = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| stem.eq_ignore_ascii_case(preferred_name))
                    .unwrap_or(false);
                let extension_matches = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extension.eq_ignore_ascii_case(preferred_extension))
                    .unwrap_or(false);

                if stem_matches && extension_matches {
                    valid_folder_cover_art(path, CoverArtPriority::FolderNamed)
                } else {
                    None
                }
            }) {
                return Some(cover_art);
            }
        }
    }

    entries
        .iter()
        .find_map(|path| valid_folder_cover_art(path, CoverArtPriority::FolderFallback))
}

fn cached_lyrics_file(track: &Track) -> Option<LyricsFile> {
    let path = PathBuf::from(track.lyrics_path.as_ref()?);
    let kind = track.lyrics_kind.as_deref()?;

    if !is_valid_lyrics_file(&path) {
        return None;
    }

    match kind {
        "synced" => Some(LyricsFile {
            path,
            kind: "synced",
        }),
        "plain" => Some(LyricsFile {
            path,
            kind: "plain",
        }),
        _ => None,
    }
}

fn lyrics_file_for_track(track_path: &Path, track_title: &str) -> Option<LyricsFile> {
    let directory = track_path.parent()?;
    let basename = track_path.file_stem()?.to_string_lossy();
    let candidates = [
        (basename.as_ref(), "lrc", "synced"),
        (basename.as_ref(), "txt", "plain"),
        (track_title, "lrc", "synced"),
        (track_title, "txt", "plain"),
        ("lyrics", "lrc", "synced"),
        ("lyrics", "txt", "plain"),
    ];

    candidates
        .iter()
        .find_map(|(stem, extension, kind)| lyrics_candidate(directory, stem, extension, kind))
}

fn lyrics_candidate(
    directory: &Path,
    expected_stem: &str,
    expected_extension: &str,
    kind: &'static str,
) -> Option<LyricsFile> {
    let entries = fs::read_dir(directory).ok()?;

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let file_type = entry.file_type().ok()?;

        if !file_type.is_file() {
            continue;
        }

        let stem_matches = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.eq_ignore_ascii_case(expected_stem))
            .unwrap_or(false);
        let extension_matches = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case(expected_extension))
            .unwrap_or(false);

        if stem_matches && extension_matches && is_valid_lyrics_file(&path) {
            return Some(LyricsFile { path, kind });
        }
    }

    None
}

fn is_valid_lyrics_file(path: &Path) -> bool {
    let Some(kind) = lyrics_kind_for_path(path) else {
        return false;
    };
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    matches!(kind, "synced" | "plain")
        && metadata.is_file()
        && metadata.len() > 0
        && metadata.len() <= MAX_LYRICS_BYTES
}

fn lyrics_kind_for_path(path: &Path) -> Option<&'static str> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| {
            if extension.eq_ignore_ascii_case("lrc") {
                Some("synced")
            } else if extension.eq_ignore_ascii_case("txt") {
                Some("plain")
            } else {
                None
            }
        })
}

fn read_lyrics_file(lyrics_file: LyricsFile) -> Option<TrackLyrics> {
    let text = fs::read_to_string(&lyrics_file.path).ok()?;

    Some(TrackLyrics {
        path: lyrics_file.path.to_string_lossy().into_owned(),
        kind: lyrics_file.kind.to_owned(),
        text,
    })
}

fn app_lyrics_file_for_track(app: &AppHandle, track: &Track) -> Option<LyricsFile> {
    let lyrics_dir = app.path().app_data_dir().ok()?.join("lyrics");
    let cache_key = format!("{:016x}", stable_hash(&track.file_path));
    let synced_path = lyrics_dir.join(format!("{cache_key}.lrc"));
    let plain_path = lyrics_dir.join(format!("{cache_key}.txt"));

    if is_valid_lyrics_file(&synced_path) {
        return Some(LyricsFile {
            path: synced_path,
            kind: "synced",
        });
    }

    if is_valid_lyrics_file(&plain_path) {
        return Some(LyricsFile {
            path: plain_path,
            kind: "plain",
        });
    }

    None
}

fn find_lrclib_lyrics(track: &Track) -> Result<Option<FoundLyrics>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Cassette/0.1.0 local lyrics lookup")
        .build()
        .map_err(|error| format!("Could not prepare lyrics lookup: {error}"))?;

    if let Some(lyrics) = get_lrclib_lyrics(&client, track)? {
        return Ok(Some(lyrics));
    }

    search_lrclib_lyrics(&client, track)
}

fn get_lrclib_lyrics(
    client: &reqwest::blocking::Client,
    track: &Track,
) -> Result<Option<FoundLyrics>, String> {
    let query = lrclib_query(track);

    if query.len() < 2 {
        return Ok(None);
    }

    let url = reqwest::Url::parse_with_params("https://lrclib.net/api/get", &query)
        .map_err(|error| format!("Could not prepare LRCLIB request: {error}"))?;
    let response = client
        .get(url)
        .send()
        .map_err(|_| "Could not reach LRCLIB. Check your connection.".to_owned())?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err("LRCLIB did not return lyrics for this track.".to_owned());
    }

    let lyrics = response
        .json::<LrclibLyrics>()
        .map_err(|_| "LRCLIB returned an unreadable lyrics response.".to_owned())?;

    Ok(found_lyrics_from_lrclib(&lyrics))
}

fn search_lrclib_lyrics(
    client: &reqwest::blocking::Client,
    track: &Track,
) -> Result<Option<FoundLyrics>, String> {
    let query = lrclib_query(track);

    if query.len() < 2 {
        return Ok(None);
    }

    let url = reqwest::Url::parse_with_params("https://lrclib.net/api/search", &query)
        .map_err(|error| format!("Could not prepare LRCLIB request: {error}"))?;
    let response = client
        .get(url)
        .send()
        .map_err(|_| "Could not reach LRCLIB. Check your connection.".to_owned())?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err("LRCLIB search failed. Try again later.".to_owned());
    }

    let mut results = response
        .json::<Vec<LrclibLyrics>>()
        .map_err(|_| "LRCLIB returned an unreadable search response.".to_owned())?;

    results.sort_by(|left, right| {
        lrclib_result_score(right, track).cmp(&lrclib_result_score(left, track))
    });

    Ok(results.iter().find_map(found_lyrics_from_lrclib))
}

fn lrclib_query(track: &Track) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();

    query.push(("track_name", track.title.clone()));

    if let Some(artist) = track.artist.as_ref().or(track.album_artist.as_ref()) {
        query.push(("artist_name", artist.clone()));
    }

    if let Some(album) = track.album.as_ref() {
        query.push(("album_name", album.clone()));
    }

    if let Some(duration) = track.duration_seconds {
        query.push(("duration", duration.to_string()));
    }

    query
}

fn lrclib_result_score(result: &LrclibLyrics, track: &Track) -> i64 {
    let mut score = 0;

    if result
        .synced_lyrics
        .as_deref()
        .map(|lyrics| !lyrics.trim().is_empty())
        .unwrap_or(false)
    {
        score += 10_000;
    }

    if text_matches(result.track_name.as_deref(), &track.title) {
        score += 1_000;
    }

    if let Some(artist) = track.artist.as_ref().or(track.album_artist.as_ref()) {
        if text_matches(result.artist_name.as_deref(), artist) {
            score += 500;
        }
    }

    if let Some(album) = track.album.as_ref() {
        if text_matches(result.album_name.as_deref(), album) {
            score += 250;
        }
    }

    if let (Some(left), Some(right)) = (result.duration, track.duration_seconds) {
        let difference = (left - f64::from(right)).abs().round() as i64;
        score += 100_i64.saturating_sub(difference.min(100));
    }

    score
}

fn text_matches(left: Option<&str>, right: &str) -> bool {
    left.map(|left| left.trim().eq_ignore_ascii_case(right.trim()))
        .unwrap_or(false)
}

fn found_lyrics_from_lrclib(lyrics: &LrclibLyrics) -> Option<FoundLyrics> {
    if let Some(text) = lyrics
        .synced_lyrics
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(FoundLyrics {
            kind: "synced",
            text: text.to_owned(),
        });
    }

    lyrics
        .plain_lyrics
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| FoundLyrics {
            kind: "plain",
            text: text.to_owned(),
        })
}

fn save_app_lyrics(
    app: &AppHandle,
    track: &Track,
    lyrics: FoundLyrics,
) -> Result<TrackLyrics, String> {
    let lyrics_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve Cassette data folder: {error}"))?
        .join("lyrics");
    fs::create_dir_all(&lyrics_dir)
        .map_err(|error| format!("Could not create lyrics cache folder: {error}"))?;

    let cache_key = format!("{:016x}", stable_hash(&track.file_path));
    let extension = if lyrics.kind == "synced" { "lrc" } else { "txt" };
    let path = lyrics_dir.join(format!("{cache_key}.{extension}"));

    if path.exists() {
        return Err("Lyrics are already cached for this track.".to_owned());
    }

    let text = lyrics.text;
    fs::write(&path, &text)
        .map_err(|error| format!("Could not save lyrics cache: {error}"))?;

    Ok(TrackLyrics {
        path: path.to_string_lossy().into_owned(),
        kind: lyrics.kind.to_owned(),
        text,
    })
}

fn folder_image_candidates(directory: &Path) -> Option<Vec<PathBuf>> {
    let entries = fs::read_dir(directory).ok()?;
    let mut paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_type = entry.file_type().ok()?;

            if file_type.is_file() && is_supported_cover_image_file(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    paths.sort();

    Some(paths)
}

fn is_supported_cover_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            COVER_IMAGE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

fn valid_folder_cover_art(path: &Path, priority: CoverArtPriority) -> Option<FolderCoverArt> {
    let metadata = fs::metadata(path).ok()?;

    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_FOLDER_COVER_BYTES {
        return None;
    }

    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 16];
    let bytes_read = file.read(&mut header).ok()?;
    let extension = folder_image_extension(&header[..bytes_read])?;

    Some(FolderCoverArt {
        path: path.to_path_buf(),
        extension,
        priority,
    })
}

fn folder_image_extension(data: &[u8]) -> Option<&'static str> {
    match image_extension(data)? {
        "jpg" => Some("jpg"),
        "png" => Some("png"),
        "webp" => Some("webp"),
        _ => None,
    }
}

fn stable_hash(value: &str) -> u64 {
    stable_hash_bytes(value.as_bytes())
}

fn stable_hash_bytes(value: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;

    for byte in value {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
}

fn unix_timestamp() -> i64 {
    system_time_to_unix(SystemTime::now()).unwrap_or(0)
}

fn unique_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn system_time_to_unix(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
}

fn read_track_metadata(path: &Path) -> TrackMetadata {
    let tagged_file = match Probe::open(path).and_then(|probe| probe.read()) {
        Ok(tagged_file) => tagged_file,
        Err(_) => return TrackMetadata::default(),
    };

    let mut metadata = TrackMetadata {
        duration_seconds: duration_seconds(&tagged_file),
        ..TrackMetadata::default()
    };

    if let Some(tag) = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
        metadata.title = clean_text(tag.title().map(|value| value.into_owned()));
        metadata.artist = clean_text(tag.artist().map(|value| value.into_owned()));
        metadata.album = clean_text(tag.album().map(|value| value.into_owned()));
        metadata.album_artist = album_artist(tag);
        metadata.track_number = tag.track();
        metadata.disc_number = tag.disk();
        metadata.year = tag.date().map(|date| date.year);
    }

    metadata.genres = genres(&tagged_file);
    metadata.embedded_cover_art = embedded_cover_art(&tagged_file);

    metadata
}

fn duration_seconds(tagged_file: &lofty::file::TaggedFile) -> Option<u32> {
    let seconds = tagged_file.properties().duration().as_secs();
    u32::try_from(seconds).ok().filter(|seconds| *seconds > 0)
}

fn album_artist(tag: &Tag) -> Option<String> {
    clean_text(
        tag.get_string(ItemKey::AlbumArtist)
            .or_else(|| tag.get_string(ItemKey::AlbumArtists))
            .map(str::to_owned),
    )
}

fn genres(tagged_file: &lofty::file::TaggedFile) -> Vec<String> {
    let genres = tagged_file
        .primary_tag()
        .into_iter()
        .chain(tagged_file.tags().iter())
        .flat_map(|tag| tag.get_strings(ItemKey::Genre))
        .flat_map(split_genres)
        .collect::<Vec<_>>();

    normalize_genres(genres)
}

fn split_genres(value: &str) -> Vec<String> {
    value
        .split([';', ',', '\0'])
        .filter_map(|genre| clean_text(Some(genre.to_owned())))
        .collect()
}

fn normalize_genres(genres: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for genre in genres {
        let key = genre.to_lowercase();

        if seen.insert(key) {
            normalized.push(genre);
        }
    }

    if normalized.is_empty() {
        normalized.push("Unknown Genre".to_owned());
    }

    normalized
}

fn embedded_cover_art(tagged_file: &lofty::file::TaggedFile) -> Option<EmbeddedCoverArt> {
    tagged_file
        .primary_tag()
        .into_iter()
        .chain(tagged_file.tags().iter())
        .flat_map(Tag::pictures)
        .find_map(cover_art_from_picture)
}

fn cover_art_from_picture(picture: &Picture) -> Option<EmbeddedCoverArt> {
    let data = picture.data();
    let extension = image_extension(data)?;

    Some(EmbeddedCoverArt {
        extension,
        data: data.to_vec(),
    })
}

fn image_extension(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("jpg");
    }

    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some("png");
    }

    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some("webp");
    }

    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some("gif");
    }

    if data.starts_with(b"BM") {
        return Some("bmp");
    }

    if data.starts_with(&[b'I', b'I', 0x2a, 0x00]) || data.starts_with(&[b'M', b'M', 0x00, 0x2a]) {
        return Some("tif");
    }

    None
}

fn clean_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.replace(['_', '-'], " "))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "Unknown Track".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(PlaybackState::default()))
        .setup(|app| {
            let db_path = app
                .path()
                .app_data_dir()
                .map_err(|error| {
                    io::Error::other(format!("Could not resolve app data dir: {error}"))
                })?
                .join("library.sqlite3");
            let library = LibraryDatabase::open(db_path).map_err(io::Error::other)?;
            let mpris = MprisState::new(app.handle().clone());

            app.manage(Mutex::new(library));
            app.manage(mpris);

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_library_cache,
            scan_library,
            toggle_track_favorite,
            record_track_play,
            set_album_genres,
            set_artist_genres,
            create_playlist,
            rename_playlist,
            delete_playlist,
            add_track_to_playlist,
            remove_track_from_playlist,
            move_playlist_track,
            read_track_lyrics,
            auto_find_track_lyrics,
            play_track,
            pause_playback,
            resume_playback,
            get_playback_status,
            seek_playback,
            set_playback_volume
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
