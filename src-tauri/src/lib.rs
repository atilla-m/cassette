use gst::prelude::*;
use gstreamer as gst;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};

const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "ogg", "opus", "wav", "m4a"];

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
    track_number: Option<u32>,
    disc_number: Option<u32>,
    year: Option<u16>,
    duration_seconds: Option<u32>,
    modified_time: Option<i64>,
    file_size: Option<i64>,
    scanned_at: Option<i64>,
}

#[derive(Debug, Default)]
struct TrackMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    year: Option<u16>,
    duration_seconds: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackStatus {
    file_path: Option<String>,
    is_playing: bool,
    position_seconds: u64,
    duration_seconds: Option<u64>,
    volume: f64,
}

#[derive(Debug, Default)]
struct PlaybackState {
    playbin: Option<gst::Element>,
    current_path: Option<String>,
    is_playing: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryCache {
    tracks: Vec<Track>,
    last_scanned_folder: Option<String>,
    last_scanned_at: Option<i64>,
}

struct LibraryDatabase {
    connection: Connection,
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
    let scanned_at = unix_timestamp();
    scan_directory(&root_path, scanned_at, &mut tracks)?;
    tracks.sort_by(|left, right| left.file_path.cmp(&right.file_path));

    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.replace_library(&root_path, &tracks, scanned_at)?;

    Ok(tracks)
}

impl LibraryDatabase {
    fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create app data directory: {error}"))?;
        }

        let connection = Connection::open(path)
            .map_err(|error| format!("Could not open library cache: {error}"))?;
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
                track_number INTEGER,
                disc_number INTEGER,
                year INTEGER,
                duration_seconds INTEGER,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                extension TEXT NOT NULL,
                modified_time INTEGER,
                file_size INTEGER,
                scanned_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS library_meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            ",
        )
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
                    scanned_at
                FROM tracks
                ORDER BY file_path
                ",
            )
            .map_err(|error| format!("Could not read library cache: {error}"))?;

        let tracks = statement
            .query_map([], row_to_track)
            .map_err(|error| format!("Could not read library cache: {error}"))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read cached tracks: {error}"))?;

        Ok(LibraryCache {
            tracks,
            last_scanned_folder: self.meta_value("last_scanned_folder")?,
            last_scanned_at: self
                .meta_value("last_scanned_at")?
                .and_then(|value| value.parse::<i64>().ok()),
        })
    }

    fn replace_library(
        &mut self,
        root_path: &Path,
        tracks: &[Track],
        scanned_at: i64,
    ) -> Result<(), String> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| format!("Could not update library cache: {error}"))?;

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
                        track_number,
                        disc_number,
                        year,
                        duration_seconds,
                        file_path,
                        file_name,
                        extension,
                        modified_time,
                        file_size,
                        scanned_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                    ",
                )
                .map_err(|error| format!("Could not prepare library cache update: {error}"))?;

            for track in tracks {
                statement
                    .execute(params![
                        &track.id,
                        &track.title,
                        &track.artist,
                        &track.album,
                        &track.album_artist,
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
            .map_err(|error| format!("Could not save library cache: {error}"))
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
}

#[tauri::command]
fn play_track(
    file_path: String,
    playback: State<'_, Mutex<PlaybackState>>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    playback.play(&file_path)
}

#[tauri::command]
fn pause_playback(playback: State<'_, Mutex<PlaybackState>>) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    playback.pause()
}

#[tauri::command]
fn resume_playback(playback: State<'_, Mutex<PlaybackState>>) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    playback.resume()
}

#[tauri::command]
fn get_playback_status(
    playback: State<'_, Mutex<PlaybackState>>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    playback.refresh()?;

    Ok(playback.status())
}

#[tauri::command]
fn seek_playback(
    position_seconds: u64,
    playback: State<'_, Mutex<PlaybackState>>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    playback.seek(position_seconds)
}

#[tauri::command]
fn set_playback_volume(
    volume: f64,
    playback: State<'_, Mutex<PlaybackState>>,
) -> Result<PlaybackStatus, String> {
    let mut playback = playback
        .lock()
        .map_err(|_| "Playback state is unavailable.".to_owned())?;

    playback.set_volume(volume)
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
            playbin.set_property("uri", uri.as_str());
            set_gst_state(playbin, gst::State::Playing)?;
            check_for_playback_error(playbin)?;
        }

        self.current_path = Some(file_path.to_owned());
        self.is_playing = true;

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

        Ok(self.status())
    }

    fn refresh(&mut self) -> Result<(), String> {
        let Some(playbin) = self.playbin.clone() else {
            return Ok(());
        };

        if let Some(message) = playbin.bus().and_then(|bus| {
            bus.timed_pop_filtered(
                gst::ClockTime::ZERO,
                &[gst::MessageType::Error, gst::MessageType::Eos],
            )
        }) {
            match message.view() {
                gst::MessageView::Eos(_) => {
                    self.is_playing = false;
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

    fn seek(&mut self, position_seconds: u64) -> Result<PlaybackStatus, String> {
        let Some(playbin) = self.playbin.as_ref() else {
            return Ok(self.status());
        };

        playbin
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_seconds(position_seconds),
            )
            .map_err(|error| format!("Could not seek track: {error}"))?;

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
    })
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
    tracks: &mut Vec<Track>,
) -> Result<(), String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };

        let path = entry.path();

        if file_type.is_dir() {
            scan_directory(&path, scanned_at, tracks)?;
        } else if file_type.is_file() && is_supported_audio_file(&path) {
            if let Some(track) = track_from_path(path, scanned_at) {
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

fn track_from_path(path: PathBuf, scanned_at: i64) -> Option<Track> {
    let metadata = read_track_metadata(&path);
    let file_info = file_info(&path);
    let path_string = path.to_string_lossy().into_owned();
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let title = metadata.title.unwrap_or_else(|| title_from_path(&path));

    Some(Track {
        id: path_string.clone(),
        file_path: path_string,
        file_name,
        extension,
        title,
        artist: metadata.artist,
        album: metadata.album,
        album_artist: metadata.album_artist,
        track_number: metadata.track_number,
        disc_number: metadata.disc_number,
        year: metadata.year,
        duration_seconds: metadata.duration_seconds,
        modified_time: file_info.modified_time,
        file_size: file_info.file_size,
        scanned_at: Some(scanned_at),
    })
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

fn unix_timestamp() -> i64 {
    system_time_to_unix(SystemTime::now()).unwrap_or(0)
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

            app.manage(Mutex::new(library));

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_library_cache,
            scan_library,
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
