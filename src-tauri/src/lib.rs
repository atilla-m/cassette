use gst::prelude::*;
use gstreamer as gst;
use libloading::Library;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::items::Timestamp;
use lofty::tag::{Accessor, ItemKey, Tag, TagType};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

mod mpris;

use mpris::{MprisState, MprisTrack};

const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "ogg", "opus", "wav", "m4a"];
const SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "mov", "m4v", "avi"];
const COVER_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const PREFERRED_COVER_NAMES: &[&str] = &["cover", "folder", "front", "album"];
const MAX_FOLDER_COVER_BYTES: u64 = 25 * 1024 * 1024;
const MAX_LYRICS_BYTES: u64 = 1024 * 1024;
const GENRE_SCOPE_ALBUM: &str = "album";
const GENRE_SCOPE_ARTIST: &str = "artist";
const MUSICBRAINZ_USER_AGENT: &str = "Cassette/0.1.0 (local music player; contact: none)";

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTrackTagsRequest {
    track_id: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    genre: Option<String>,
    year: Option<u16>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackTagValues {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    genre: Option<String>,
    year: Option<u16>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackTagEditorData {
    track: Track,
    file_values: TrackTagValues,
    genre_override_active: bool,
    tag_editing_supported: bool,
    unsupported_reason: Option<String>,
}

#[derive(Debug)]
struct EmbeddedCoverArt {
    extension: &'static str,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoEntry {
    id: String,
    file_path: String,
    file_name: String,
    title: String,
    artist: Option<String>,
    video_type: String,
    source: String,
    release_or_collection: Option<String>,
    year: Option<u16>,
    venue: Option<String>,
    city: Option<String>,
    country: Option<String>,
    description_or_notes: Option<String>,
    duration_seconds: Option<u32>,
    thumbnail_path: Option<String>,
    last_position_seconds: u32,
    play_count: i64,
    last_played_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoLibrary {
    videos: Vec<VideoEntry>,
    last_video_folder: Option<String>,
    last_video_scanned_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoInfoUpdate {
    title: String,
    artist: Option<String>,
    video_type: String,
    release_or_collection: Option<String>,
    year: Option<u16>,
    venue: Option<String>,
    city: Option<String>,
    country: Option<String>,
    description_or_notes: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DvdDetectResult {
    found: bool,
    device_path: Option<String>,
    readable: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DvdTitle {
    number: u32,
    duration: Option<String>,
    duration_seconds: Option<u32>,
    chapters: Option<u32>,
    likely_main_title: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DvdTitleScanResult {
    source_type: String,
    source_path: String,
    titles: Vec<DvdTitle>,
    raw_output: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DvdImportEvent {
    output_folder: Option<String>,
    output_path: Option<String>,
    title_number: Option<u32>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DvdImportMetadata {
    title: String,
    artist: String,
    video_type: String,
    release_or_collection: Option<String>,
    year: Option<u16>,
    venue: Option<String>,
    city: Option<String>,
    country: Option<String>,
    description_or_notes: Option<String>,
    output_filename: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DvdImportResult {
    video: VideoEntry,
    output_folder: String,
    output_path: String,
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
    source: &'static str,
    fetched_at: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackLyrics {
    path: String,
    kind: String,
    text: String,
    source: String,
    fetched_at: Option<i64>,
    track_path: Option<String>,
    selected_track_name: Option<String>,
    selected_artist_name: Option<String>,
    selected_album_name: Option<String>,
    offset_seconds: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoLyricsResult {
    status: String,
    lyrics: Option<TrackLyrics>,
    results: Vec<LrclibLyricsResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibLyricsResult {
    track_name: String,
    artist_name: String,
    album_name: Option<String>,
    duration_seconds: Option<f64>,
    has_synced_lyrics: bool,
    has_plain_lyrics: bool,
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
    title_match: String,
    artist_match: String,
    duration_difference_seconds: Option<i64>,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CdRipTrack {
    number: u32,
    duration: Option<String>,
    duration_seconds: Option<u32>,
    status: Option<String>,
    output_filename: Option<String>,
    error: Option<String>,
    warning: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CdDetectResult {
    drive_found: bool,
    disc_found: bool,
    tracks: Vec<CdRipTrack>,
    raw_output: String,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CdRipResult {
    output_folder: String,
    tracks: Vec<CdRipTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CdRipMetadataTrack {
    number: u32,
    title: String,
    artist: String,
    disc_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CdRipCover {
    source: String,
    path: String,
    mime_type: String,
    extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CdRipMetadata {
    album_artist: String,
    album_title: String,
    year: String,
    genre: String,
    disc_number: Option<u32>,
    cover: Option<CdRipCover>,
    tracks: Vec<CdRipMetadataTrack>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CdMetadataRelease {
    id: String,
    title: String,
    artist: String,
    date: Option<String>,
    year: Option<String>,
    country: Option<String>,
    format: Option<String>,
    label: Option<String>,
    catalog_number: Option<String>,
    track_count: u32,
    disc_number: Option<u32>,
    tracks: Vec<CdRipMetadataTrack>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CdMetadataLookupResult {
    disc_id: String,
    toc: String,
    releases: Vec<CdMetadataRelease>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CdCoverLookupResult {
    found: bool,
    cover: Option<CdRipCover>,
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CdRipEvent {
    output_folder: Option<String>,
    track_number: Option<u32>,
    output_filename: Option<String>,
    output_path: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone)]
struct PreparedCdCover {
    data: Vec<u8>,
    mime_type: String,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LyricsCacheMetadata {
    track_path: String,
    lyrics_kind: String,
    source: String,
    fetched_at: i64,
    selected_track_name: Option<String>,
    selected_artist_name: Option<String>,
    selected_album_name: Option<String>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoPlaybackStatus {
    video_id: Option<String>,
    file_path: Option<String>,
    is_playing: bool,
    has_ended: bool,
    position_seconds: u64,
    duration_seconds: Option<u64>,
    volume: f64,
    has_video_window: bool,
    is_fullscreen: bool,
    backend: String,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoCodecInfo {
    container: Option<String>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    resolution: Option<String>,
    duration_seconds: Option<u32>,
    error: Option<String>,
}

#[derive(Debug, Default)]
struct PlaybackState {
    playbin: Option<gst::Element>,
    current_path: Option<String>,
    is_playing: bool,
    has_ended: bool,
}

#[derive(Debug, Default)]
struct VideoPlaybackState {
    child: Option<Child>,
    ipc_path: Option<PathBuf>,
    log_path: Option<PathBuf>,
    current_video_id: Option<String>,
    current_path: Option<String>,
    is_playing: bool,
    has_ended: bool,
    position_seconds: u64,
    duration_seconds: Option<u64>,
    volume: f64,
    has_video_window: bool,
    is_fullscreen: bool,
    request_id: u64,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct VideoProgressSnapshot {
    video_id: String,
    position_seconds: u64,
    has_ended: bool,
}

#[derive(Debug, Default)]
struct TrackLyricsSettings {
    offset_seconds: f64,
    preferred_source: Option<String>,
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

#[derive(Debug, Default)]
struct TagWriteState {
    active_paths: HashSet<String>,
}

struct ImportedDvdVideo {
    video: VideoEntry,
    output_folder: String,
    output_path: String,
}

#[tauri::command]
fn get_library_cache(library: State<'_, Mutex<LibraryDatabase>>) -> Result<LibraryCache, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.load_cache()
}

#[tauri::command]
fn send_linux_notification(title: String, body: String) -> Result<(), String> {
    let output = Command::new("notify-send")
        .arg("--app-name=Cassette")
        .arg(title)
        .arg(body)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                "notify-send is unavailable.".to_owned()
            } else {
                format!("Could not run notify-send: {error}")
            }
        })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();

    if stderr.is_empty() {
        Err(format!(
            "notify-send exited unsuccessfully with status {}.",
            output
                .status
                .code()
                .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
        ))
    } else {
        Err(format!("notify-send failed: {stderr}"))
    }
}

#[tauri::command]
fn get_video_library(library: State<'_, Mutex<LibraryDatabase>>) -> Result<VideoLibrary, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.video_library()
}

#[tauri::command]
async fn scan_video_folder(
    folder_path: String,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoLibrary, String> {
    let root_path = PathBuf::from(folder_path);

    if !root_path.exists() {
        return Err("Selected video folder does not exist.".into());
    }

    if !root_path.is_dir() {
        return Err("Selected video path is not a folder.".into());
    }

    let scan_root = root_path.clone();
    let thumbnail_dir = app.path().app_data_dir().ok().map(|path| path.join("video-thumbnails"));
    let scanned_at = unix_timestamp();
    let mut videos = tauri::async_runtime::spawn_blocking(move || {
        scan_video_directory_root(&scan_root, scanned_at, thumbnail_dir.as_deref())
    })
    .await
    .map_err(|error| format!("Could not scan video folder: {error}"))??;

    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.replace_videos(&root_path, &mut videos, scanned_at)?;
    library.video_library()
}

#[tauri::command]
fn update_video_info(
    video_id: String,
    info: VideoInfoUpdate,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoEntry, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.update_video_info(&video_id, info)
}

#[tauri::command]
fn update_video_progress(
    video_id: String,
    last_position_seconds: u32,
    increment_play_count: bool,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoEntry, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library.update_video_progress(&video_id, last_position_seconds, increment_play_count)
}

#[tauri::command]
async fn play_video(
    video_id: String,
    start_position_seconds: Option<f64>,
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
    library: State<'_, Mutex<LibraryDatabase>>,
    playback: State<'_, Mutex<PlaybackState>>,
    mpris: State<'_, MprisState>,
) -> Result<VideoPlaybackStatus, String> {
    let video = {
        let library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;

        library
            .video_by_id(&video_id)?
            .ok_or_else(|| "Video is not in the library.".to_owned())?
    };

    if let Ok(mut playback) = playback.lock() {
        let status = playback.pause()?;
        mpris.update_playback(status.is_playing, status.position_seconds, status.volume);
    }

    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    video_playback.play(&video, start_position_seconds)
}

#[tauri::command]
fn pause_video(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
) -> Result<VideoPlaybackStatus, String> {
    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    video_playback.pause()
}

#[tauri::command]
fn resume_video(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
) -> Result<VideoPlaybackStatus, String> {
    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    video_playback.resume()
}

#[tauri::command]
fn stop_video(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoPlaybackStatus, String> {
    save_active_video_progress(&video_playback, &library, false)?;

    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    video_playback.stop()
}

#[tauri::command]
fn seek_video(
    position_seconds: f64,
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
) -> Result<VideoPlaybackStatus, String> {
    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    video_playback.seek(position_seconds)
}

#[tauri::command]
fn set_video_volume(
    volume: f64,
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
) -> Result<VideoPlaybackStatus, String> {
    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    video_playback.set_volume(volume)
}

#[tauri::command]
fn get_video_state(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoPlaybackStatus, String> {
    let (status, closed_progress) = {
        let mut video_playback = video_playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let closed_progress = video_playback.refresh();

        (video_playback.status(), closed_progress)
    };

    if let Some(progress) = closed_progress {
        save_video_progress_snapshot(&library, &progress)?;
    }

    Ok(status)
}

#[tauri::command]
fn get_video_position(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoPlaybackStatus, String> {
    get_video_state(video_playback, library)
}

#[tauri::command]
fn bring_video_window_to_front(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
) -> Result<VideoPlaybackStatus, String> {
    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    video_playback.bring_to_front()?;

    Ok(video_playback.status())
}

#[tauri::command]
fn fullscreen_video_window(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
) -> Result<VideoPlaybackStatus, String> {
    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    video_playback.toggle_fullscreen()?;

    Ok(video_playback.status())
}

#[tauri::command]
fn close_video_window(
    video_playback: State<'_, Mutex<VideoPlaybackState>>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoPlaybackStatus, String> {
    save_active_video_progress(&video_playback, &library, false)?;
    let mut video_playback = video_playback
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    video_playback.stop()
}

#[tauri::command]
fn get_video_codec_info(
    video_id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<VideoCodecInfo, String> {
    let video = {
        let library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;

        library
            .video_by_id(&video_id)?
            .ok_or_else(|| "Video is not in the library.".to_owned())?
    };

    Ok(ffprobe_video_codec_info(Path::new(&video.file_path)))
}

#[tauri::command]
async fn detect_dvd() -> Result<DvdDetectResult, String> {
    tauri::async_runtime::spawn_blocking(detect_dvd_blocking)
        .await
        .map_err(|error| format!("Could not detect DVD: {error}"))
}

#[tauri::command]
async fn scan_dvd_titles(source: String) -> Result<DvdTitleScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || scan_dvd_titles_blocking(source))
        .await
        .map_err(|error| format!("Could not scan DVD titles: {error}"))?
}

#[tauri::command]
async fn import_dvd_title(
    source: String,
    title_number: u32,
    output_folder: String,
    metadata: DvdImportMetadata,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<DvdImportResult, String> {
    let imported = tauri::async_runtime::spawn_blocking(move || {
        import_dvd_title_blocking(source, title_number, output_folder, metadata, app)
    })
    .await
    .map_err(|error| format!("Could not import DVD title: {error}"))??;

    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    let video = library.upsert_video(imported.video)?;

    Ok(DvdImportResult {
        video,
        output_folder: imported.output_folder,
        output_path: imported.output_path,
    })
}

#[tauri::command]
async fn scan_library(
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

    let cover_art_dir = app
        .path()
        .app_data_dir()
        .ok()
        .map(|path| path.join("cover-art"));
    let scanned_at = unix_timestamp();
    let scan_root = root_path.clone();
    let mut tracks = tauri::async_runtime::spawn_blocking(move || {
        scan_audio_directory_root(&scan_root, scanned_at, cover_art_dir.as_deref())
    })
    .await
    .map_err(|error| format!("Could not scan library: {error}"))??;

    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.replace_library(&root_path, &mut tracks, scanned_at)?;

    Ok(tracks)
}

#[tauri::command]
fn get_track_tag_editor_data(
    track_id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<TrackTagEditorData, String> {
    let (track, root_path, genre_assignments) = {
        let library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;
        let track = library
            .track_by_id(&track_id)?
            .ok_or_else(|| "Track is not in the library cache.".to_owned())?;
        let root_path = library
            .meta_value("last_scanned_folder")?
            .ok_or_else(|| "No active library folder is configured.".to_owned())?;
        let genre_assignments = library.genre_assignments()?;

        (track, PathBuf::from(root_path), genre_assignments)
    };
    let target_path = validated_cached_track_path(&track, &root_path)?;
    let tagged_file = read_tagged_file(&target_path)?;
    let file_values = track_tag_values_from_file(&tagged_file);
    let tag_editing_supported = tag_editing_supported(&tagged_file);
    let unsupported_reason = if tag_editing_supported {
        None
    } else {
        Some("Tag editing is not currently supported for this file format.".to_owned())
    };
    let genre_override_active = genre_override_active_for_values(
        &file_values,
        &track,
        &genre_assignments,
    );

    Ok(TrackTagEditorData {
        track,
        file_values,
        genre_override_active,
        tag_editing_supported,
        unsupported_reason,
    })
}

#[tauri::command]
fn update_track_tags(
    request: UpdateTrackTagsRequest,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
    playback: State<'_, Mutex<PlaybackState>>,
    mpris: State<'_, MprisState>,
    tag_writes: State<'_, Mutex<TagWriteState>>,
) -> Result<Track, String> {
    validate_update_track_tags_request(&request)?;

    let (cached_track, root_path) = {
        let library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;
        let track = library
            .track_by_id(&request.track_id)?
            .ok_or_else(|| "Track is not in the library cache.".to_owned())?;
        let root_path = library
            .meta_value("last_scanned_folder")?
            .ok_or_else(|| "No active library folder is configured.".to_owned())?;

        (track, PathBuf::from(root_path))
    };
    let target_path = validated_cached_track_path(&cached_track, &root_path)?;
    let canonical_key = target_path.to_string_lossy().into_owned();
    let _write_guard = TagWriteGuard::new(&tag_writes, canonical_key)?;
    safe_update_track_tags(&target_path, &request)?;

    let scanned_at = unix_timestamp();
    let mut updated_track = rescan_single_track_after_tag_write(
        &PathBuf::from(&cached_track.file_path),
        scanned_at,
        app.path().app_data_dir().ok().map(|path| path.join("cover-art")),
        cached_track.cover_art_path.clone(),
    )?;

    {
        let mut library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;
        library.update_cached_track(&mut updated_track)?;
    }

    let current_status = playback
        .lock()
        .ok()
        .map(|playback| playback.status());
    if current_status
        .as_ref()
        .and_then(|status| status.file_path.as_deref())
        == Some(updated_track.file_path.as_str())
    {
        mpris.update_track(Some(MprisTrack::from(&updated_track)), current_status.map(|status| status.is_playing).unwrap_or(false));
    }

    Ok(updated_track)
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
    let (track, settings) = library
        .lock()
        .ok()
        .map(|library| {
            let track = library.track_by_id(&track_path).ok().flatten();
            let settings = library.lyrics_settings(&track_path).unwrap_or_default();
            (track, settings)
        })
        .unwrap_or_default();
    let track_path = track
        .as_ref()
        .map(|track| PathBuf::from(&track.file_path))
        .unwrap_or_else(|| PathBuf::from(&track_path));
    let track_title = track
        .as_ref()
        .map(|track| track.title.clone())
        .unwrap_or_else(|| title_from_path(&track_path));
    let local_lyrics_file = || {
        track
            .as_ref()
            .and_then(cached_lyrics_file)
            .or_else(|| lyrics_file_for_track(&track_path, &track_title))
    };
    let cached_lrclib_file = || {
        track
            .as_ref()
            .and_then(|track| app_lyrics_file_for_track(&app, track))
    };
    let lyrics_file = match settings.preferred_source.as_deref() {
        Some("cached_lrclib") => cached_lrclib_file().or_else(local_lyrics_file),
        Some("local") => local_lyrics_file().or_else(cached_lrclib_file),
        _ => local_lyrics_file().or_else(cached_lrclib_file),
    };

    Ok(lyrics_file.and_then(|lyrics_file| read_lyrics_file(lyrics_file, settings.offset_seconds)))
}

#[tauri::command]
async fn auto_find_track_lyrics(
    track_path: String,
    replace_cached: bool,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<AutoLyricsResult, String> {
    let (track, offset_seconds) = {
        let library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;

        let track = library
            .track_by_id(&track_path)?
            .ok_or_else(|| "Track is not in the library cache.".to_owned())?;
        let offset_seconds = library
            .lyrics_settings(&track_path)
            .unwrap_or_default()
            .offset_seconds;

        (track, offset_seconds)
    };
    let local_lyrics = cached_lyrics_file(&track)
        .or_else(|| lyrics_file_for_track(Path::new(&track.file_path), &track.title))
        .and_then(|lyrics_file| read_lyrics_file(lyrics_file, offset_seconds));

    if let Some(lyrics) = local_lyrics.filter(|_| !replace_cached) {
        return Ok(AutoLyricsResult {
            status: "existing".to_owned(),
            lyrics: Some(lyrics),
            results: Vec::new(),
        });
    }

    let cached_lyrics = app_lyrics_file_for_track(&app, &track)
        .and_then(|lyrics_file| read_lyrics_file(lyrics_file, offset_seconds));

    if let Some(lyrics) = cached_lyrics.filter(|_| !replace_cached) {
        return Ok(AutoLyricsResult {
            status: "existing".to_owned(),
            lyrics: Some(lyrics),
            results: Vec::new(),
        });
    }

    let results = search_lrclib_lyrics_results(&track).await?;

    if results.is_empty() {
        return Ok(AutoLyricsResult {
            status: "not_found".to_owned(),
            lyrics: None,
            results,
        });
    }

    let selected_result = results
        .first()
        .cloned()
        .ok_or_else(|| "No usable LRCLIB lyrics were found.".to_owned())?;
    let lyrics = found_lyrics_from_result(&selected_result)
        .ok_or_else(|| "The best LRCLIB result does not include usable lyrics.".to_owned())?;
    let saved_lyrics = save_app_lyrics(
        &app,
        &track,
        lyrics,
        Some(&selected_result),
        true,
        offset_seconds,
    )?;

    Ok(AutoLyricsResult {
        status: "found".to_owned(),
        lyrics: Some(saved_lyrics),
        results: Vec::new(),
    })
}

#[tauri::command]
async fn search_track_lyrics_results(
    track_path: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Vec<LrclibLyricsResult>, String> {
    let track = {
        let library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;

        library
            .track_by_id(&track_path)?
            .ok_or_else(|| "Track is not in the library cache.".to_owned())?
    };

    search_lrclib_lyrics_results(&track).await
}

#[tauri::command]
fn save_track_lyrics_result(
    track_path: String,
    result: LrclibLyricsResult,
    replace_cached: bool,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<TrackLyrics, String> {
    let (track, offset_seconds) = {
        let library = library
            .lock()
            .map_err(|_| "Library cache is unavailable.".to_owned())?;

        let track = library
            .track_by_id(&track_path)?
            .ok_or_else(|| "Track is not in the library cache.".to_owned())?;
        let offset_seconds = library
            .lyrics_settings(&track_path)
            .unwrap_or_default()
            .offset_seconds;

        (track, offset_seconds)
    };
    let lyrics = found_lyrics_from_result(&result)
        .ok_or_else(|| "The selected LRCLIB result does not include usable lyrics.".to_owned())?;

    let saved_lyrics = save_app_lyrics(
        &app,
        &track,
        lyrics,
        Some(&result),
        replace_cached,
        offset_seconds,
    )?;
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.set_lyrics_preferred_source(&track.file_path, Some("cached_lrclib"))?;

    Ok(saved_lyrics)
}

#[tauri::command]
fn remove_cached_track_lyrics(
    track_path: String,
    app: AppHandle,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<(), String> {
    let lyrics_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve Cassette data folder: {error}"))?
        .join("lyrics");
    let cache_key = format!("{:016x}", stable_hash(&track_path));
    let paths = [
        lyrics_dir.join(format!("{cache_key}.lrc")),
        lyrics_dir.join(format!("{cache_key}.txt")),
        lyrics_dir.join(format!("{cache_key}.json")),
    ];

    for path in paths {
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|error| format!("Could not remove cached lyrics: {error}"))?;
        }
    }

    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.set_lyrics_preferred_source(&track_path, None)?;

    Ok(())
}

#[tauri::command]
fn set_track_lyrics_offset(
    track_path: String,
    offset_seconds: f64,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<f64, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;

    library
        .track_by_id(&track_path)?
        .ok_or_else(|| "Track is not in the library cache.".to_owned())?;
    library.set_lyrics_offset(&track_path, offset_seconds)
}

#[tauri::command]
async fn detect_audio_cd() -> Result<CdDetectResult, String> {
    tauri::async_runtime::spawn_blocking(run_cdparanoia_query)
        .await
        .map_err(|error| format!("Could not detect audio CD: {error}"))?
}

#[tauri::command]
async fn lookup_cd_metadata() -> Result<CdMetadataLookupResult, String> {
    let disc_info = tauri::async_runtime::spawn_blocking(read_musicbrainz_disc)
        .await
        .map_err(|error| format!("Could not read CD Disc ID: {error}"))??;

    lookup_musicbrainz_disc(&disc_info).await
}

#[tauri::command]
async fn lookup_cd_cover(release_id: String) -> Result<CdCoverLookupResult, String> {
    lookup_cover_art_archive(&release_id).await
}

#[tauri::command]
async fn inspect_cover_image(path: String) -> Result<CdCoverLookupResult, String> {
    tauri::async_runtime::spawn_blocking(move || inspect_cover_image_blocking(path))
        .await
        .map_err(|error| format!("Could not inspect cover image: {error}"))?
}

#[tauri::command]
async fn rip_cd_to_flac(
    output_folder: String,
    metadata: Option<CdRipMetadata>,
    app: AppHandle,
) -> Result<CdRipResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rip_cd_to_flac_blocking(output_folder, metadata, app)
    })
        .await
        .map_err(|error| format!("Could not rip audio CD: {error}"))?
}

#[derive(Debug)]
struct MusicBrainzDisc {
    id: String,
    toc: String,
}

fn read_musicbrainz_disc() -> Result<MusicBrainzDisc, String> {
    unsafe {
        let library = load_libdiscid()?;
        let discid_new = library
            .get::<unsafe extern "C" fn() -> *mut c_void>(b"discid_new\0")
            .map_err(|error| format!("Installed libdiscid is missing discid_new: {error}"))?;
        let discid_free = library
            .get::<unsafe extern "C" fn(*mut c_void)>(b"discid_free\0")
            .map_err(|error| format!("Installed libdiscid is missing discid_free: {error}"))?;
        let discid_read_sparse = library
            .get::<unsafe extern "C" fn(*mut c_void, *const c_char, c_uint) -> c_int>(
                b"discid_read_sparse\0",
            )
            .map_err(|error| {
                format!("Installed libdiscid is missing discid_read_sparse: {error}")
            })?;
        let discid_get_id = library
            .get::<unsafe extern "C" fn(*mut c_void) -> *const c_char>(b"discid_get_id\0")
            .map_err(|error| format!("Installed libdiscid is missing discid_get_id: {error}"))?;
        let discid_get_toc_string = library
            .get::<unsafe extern "C" fn(*mut c_void) -> *const c_char>(
                b"discid_get_toc_string\0",
            )
            .map_err(|error| {
                format!("Installed libdiscid is missing discid_get_toc_string: {error}")
            })?;
        let discid_get_error_msg = library
            .get::<unsafe extern "C" fn(*mut c_void) -> *const c_char>(
                b"discid_get_error_msg\0",
            )
            .map_err(|error| {
                format!("Installed libdiscid is missing discid_get_error_msg: {error}")
            })?;
        let disc = discid_new();

        if disc.is_null() {
            return Err("libdiscid could not allocate a Disc ID reader.".to_owned());
        }

        let read_feature = 1;
        let read_result = discid_read_sparse(disc, std::ptr::null(), read_feature);
        if read_result != 1 {
            let error = c_string_from_ptr(discid_get_error_msg(disc))
                .unwrap_or_else(|| "Could not read the audio CD TOC.".to_owned());
            discid_free(disc);
            return Err(format!("Could not read Disc ID with libdiscid: {error}"));
        }

        let id = c_string_from_ptr(discid_get_id(disc))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "libdiscid returned an empty Disc ID.".to_owned())?;
        let toc = c_string_from_ptr(discid_get_toc_string(disc)).unwrap_or_default();
        discid_free(disc);

        Ok(MusicBrainzDisc { id, toc })
    }
}

fn load_libdiscid() -> Result<Library, String> {
    for name in ["libdiscid.so.0", "libdiscid.so"] {
        if let Ok(library) = unsafe { Library::new(name) } {
            return Ok(library);
        }
    }

    Err("libdiscid is not installed. Install libdiscid and try metadata lookup again.".to_owned())
}

unsafe fn c_string_from_ptr(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }

    CStr::from_ptr(value)
        .to_str()
        .ok()
        .map(str::to_owned)
}

async fn lookup_musicbrainz_disc(disc: &MusicBrainzDisc) -> Result<CdMetadataLookupResult, String> {
    let client = reqwest::Client::builder()
        .user_agent(MUSICBRAINZ_USER_AGENT)
        .build()
        .map_err(|error| format!("Could not prepare MusicBrainz lookup: {error}"))?;
    let url = format!(
        "https://musicbrainz.org/ws/2/discid/{}?fmt=json&inc=artists+recordings+release-groups+labels",
        disc.id
    );
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("MusicBrainz is unavailable or offline: {error}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(CdMetadataLookupResult {
            disc_id: disc.id.clone(),
            toc: disc.toc.clone(),
            releases: Vec::new(),
            error: Some("No matching MusicBrainz release was found for this Disc ID.".to_owned()),
        });
    }

    if !response.status().is_success() {
        return Err(format!(
            "MusicBrainz lookup failed with HTTP status {}.",
            response.status()
        ));
    }

    let payload = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("Could not read MusicBrainz metadata: {error}"))?;
    let mut releases = payload
        .get("releases")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|release| musicbrainz_release_from_json(release, &disc.id))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    releases.sort_by(|left, right| {
        right
            .track_count
            .cmp(&left.track_count)
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.artist.cmp(&right.artist))
    });
    let error = if releases.is_empty() {
        Some("No matching MusicBrainz release was found for this Disc ID.".to_owned())
    } else {
        None
    };

    Ok(CdMetadataLookupResult {
        disc_id: disc.id.clone(),
        toc: disc.toc.clone(),
        releases,
        error,
    })
}

fn musicbrainz_release_from_json(
    release: &serde_json::Value,
    disc_id: &str,
) -> Option<CdMetadataRelease> {
    let id = json_string(release.get("id"))?;
    let title = json_string(release.get("title")).unwrap_or_else(|| "Unknown Album".to_owned());
    let artist = musicbrainz_artist_credit(release)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Unknown Artist".to_owned());
    let date = json_string(release.get("date"));
    let year = date
        .as_deref()
        .and_then(|value| value.get(0..4))
        .filter(|value| value.chars().all(|character| character.is_ascii_digit()))
        .map(str::to_owned);
    let country = json_string(release.get("country"));
    let (format, disc_number, tracks) = musicbrainz_medium_for_disc(release, disc_id);
    let track_count = tracks.len() as u32;
    let (label, catalog_number) = musicbrainz_label_info(release);

    Some(CdMetadataRelease {
        id,
        title,
        artist,
        date,
        year,
        country,
        format,
        label,
        catalog_number,
        track_count,
        disc_number,
        tracks,
    })
}

fn musicbrainz_medium_for_disc(
    release: &serde_json::Value,
    disc_id: &str,
) -> (Option<String>, Option<u32>, Vec<CdRipMetadataTrack>) {
    let media = release
        .get("media")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let selected_medium = media
        .iter()
        .find(|medium| medium_has_disc_id(medium, disc_id))
        .or_else(|| media.first());

    let Some(medium) = selected_medium else {
        return (None, None, Vec::new());
    };

    let format = json_string(medium.get("format"));
    let disc_number = medium
        .get("position")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let tracks = medium
        .get("tracks")
        .and_then(serde_json::Value::as_array)
        .map(|tracks| {
            tracks
                .iter()
                .enumerate()
                .map(|(index, track)| {
                    let number = track
                        .get("number")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|value| value.parse::<u32>().ok())
                        .or_else(|| {
                            track
                                .get("position")
                                .and_then(serde_json::Value::as_u64)
                                .and_then(|value| u32::try_from(value).ok())
                        })
                        .unwrap_or((index + 1) as u32);
                    let title = json_string(track.get("title"))
                        .or_else(|| {
                            track
                                .get("recording")
                                .and_then(|recording| json_string(recording.get("title")))
                        })
                        .unwrap_or_else(|| format!("Track {number:02}"));
                    let artist = musicbrainz_artist_credit(track)
                        .or_else(|| {
                            track
                                .get("recording")
                                .and_then(musicbrainz_artist_credit)
                        })
                        .unwrap_or_default();

                    CdRipMetadataTrack {
                        number,
                        title,
                        artist,
                        disc_number,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    (format, disc_number, tracks)
}

fn medium_has_disc_id(medium: &serde_json::Value, disc_id: &str) -> bool {
    medium
        .get("discs")
        .and_then(serde_json::Value::as_array)
        .map(|discs| {
            discs
                .iter()
                .any(|disc| disc.get("id").and_then(serde_json::Value::as_str) == Some(disc_id))
        })
        .unwrap_or(false)
}

fn musicbrainz_artist_credit(value: &serde_json::Value) -> Option<String> {
    let artists = value.get("artist-credit")?.as_array()?;
    let mut name = String::new();

    for artist_credit in artists {
        if let Some(part) = json_string(artist_credit.get("name")).or_else(|| {
            artist_credit
                .get("artist")
                .and_then(|artist| json_string(artist.get("name")))
        }) {
            name.push_str(&part);
        }

        if let Some(joinphrase) = artist_credit
            .get("joinphrase")
            .and_then(serde_json::Value::as_str)
        {
            name.push_str(joinphrase);
        }
    }

    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn musicbrainz_label_info(release: &serde_json::Value) -> (Option<String>, Option<String>) {
    let Some(label_info) = release
        .get("label-info")
        .and_then(serde_json::Value::as_array)
        .and_then(|labels| labels.first())
    else {
        return (None, None);
    };
    let label = label_info
        .get("label")
        .and_then(|label| json_string(label.get("name")));
    let catalog_number = json_string(label_info.get("catalog-number"));

    (label, catalog_number)
}

fn json_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

async fn lookup_cover_art_archive(release_id: &str) -> Result<CdCoverLookupResult, String> {
    let client = reqwest::Client::builder()
        .user_agent(MUSICBRAINZ_USER_AGENT)
        .build()
        .map_err(|error| format!("Could not prepare cover lookup: {error}"))?;
    let url = format!("https://coverartarchive.org/release/{release_id}/");
    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| format!("Cover lookup failed: {error}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(CdCoverLookupResult {
            found: false,
            cover: None,
            message: Some("No cover art found".to_owned()),
        });
    }

    if !response.status().is_success() {
        return Err(format!(
            "Cover lookup failed with HTTP status {}.",
            response.status()
        ));
    }

    let payload = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("Could not read cover metadata: {error}"))?;
    let Some(image_url) = cover_art_archive_front_image_url(&payload) else {
        return Ok(CdCoverLookupResult {
            found: false,
            cover: None,
            message: Some("No cover art found".to_owned()),
        });
    };

    let image_response = client
        .get(&image_url)
        .send()
        .await
        .map_err(|error| format!("Could not download cover art: {error}"))?;

    if !image_response.status().is_success() {
        return Err(format!(
            "Cover image download failed with HTTP status {}.",
            image_response.status()
        ));
    }

    let content_type = image_response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let bytes = image_response
        .bytes()
        .await
        .map_err(|error| format!("Could not read cover image: {error}"))?
        .to_vec();
    let (mime_type, extension) = cover_mime_and_extension(&bytes, content_type.as_deref())
        .ok_or_else(|| "Cover Art Archive returned an unsupported image type.".to_owned())?;
    let cover_path = save_temp_cd_cover(release_id, &extension, &bytes)?;

    Ok(CdCoverLookupResult {
        found: true,
        cover: Some(CdRipCover {
            source: "cover-art-archive".to_owned(),
            path: cover_path.to_string_lossy().to_string(),
            mime_type,
            extension,
        }),
        message: Some("Cover found".to_owned()),
    })
}

fn cover_art_archive_front_image_url(payload: &serde_json::Value) -> Option<String> {
    let images = payload.get("images")?.as_array()?;
    let selected = images
        .iter()
        .find(|image| image.get("front").and_then(serde_json::Value::as_bool) == Some(true))
        .or_else(|| {
            images.iter().find(|image| {
                image
                    .get("types")
                    .and_then(serde_json::Value::as_array)
                    .map(|types| {
                        types.iter().any(|value| {
                            value
                                .as_str()
                                .map(|value| value.eq_ignore_ascii_case("front"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })?;

    json_string(selected.get("image"))
}

fn save_temp_cd_cover(release_id: &str, extension: &str, data: &[u8]) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("cassette-cd-covers");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create temporary cover folder: {error}"))?;
    let path = directory.join(format!(
        "{:016x}-{}.{}",
        stable_hash(release_id),
        unique_timestamp_nanos(),
        extension
    ));
    fs::write(&path, data).map_err(|error| format!("Could not cache cover image: {error}"))?;

    Ok(path)
}

fn inspect_cover_image_blocking(path: String) -> Result<CdCoverLookupResult, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("Selected cover image does not exist.".to_owned());
    }
    if !path.is_file() {
        return Err("Selected cover image is not a file.".to_owned());
    }

    let data = fs::read(&path).map_err(|error| format!("Could not read cover image: {error}"))?;
    let (mime_type, extension) = cover_mime_and_extension(&data, None)
        .ok_or_else(|| "Selected cover image must be JPG, PNG, or WEBP.".to_owned())?;

    Ok(CdCoverLookupResult {
        found: true,
        cover: Some(CdRipCover {
            source: "manual".to_owned(),
            path: path.to_string_lossy().to_string(),
            mime_type,
            extension,
        }),
        message: Some("Manual cover selected".to_owned()),
    })
}

fn cover_mime_and_extension(data: &[u8], content_type: Option<&str>) -> Option<(String, String)> {
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some(("image/jpeg".to_owned(), "jpg".to_owned()));
    }

    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some(("image/png".to_owned(), "png".to_owned()));
    }

    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some(("image/webp".to_owned(), "webp".to_owned()));
    }

    match content_type?.to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => Some(("image/jpeg".to_owned(), "jpg".to_owned())),
        "image/png" => Some(("image/png".to_owned(), "png".to_owned())),
        "image/webp" => Some(("image/webp".to_owned(), "webp".to_owned())),
        _ => None,
    }
}

fn run_cdparanoia_query() -> Result<CdDetectResult, String> {
    let output = match Command::new("cdparanoia").arg("-Q").output() {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(CdDetectResult {
                drive_found: false,
                disc_found: false,
                tracks: Vec::new(),
                raw_output: String::new(),
                error: Some(
                    "cdparanoia is not installed. Install cdparanoia and try again.".to_owned(),
                ),
            });
        }
        Err(error) => {
            return Ok(CdDetectResult {
                drive_found: false,
                disc_found: false,
                tracks: Vec::new(),
                raw_output: String::new(),
                error: Some(format!("Could not run cdparanoia: {error}")),
            });
        }
    };

    Ok(parse_cdparanoia_query_output(output))
}

fn parse_cdparanoia_query_output(output: Output) -> CdDetectResult {
    let raw_output = command_output_text(&output);
    let tracks = parse_cdparanoia_tracks(&raw_output);
    let lower_output = raw_output.to_lowercase();
    let drive_found = !lower_output.contains("unable to open cdrom")
        && !lower_output.contains("could not open")
        && !lower_output.contains("no cdrom")
        && !lower_output.contains("no such file or directory");
    let disc_found = !tracks.is_empty();
    let error = if disc_found {
        None
    } else {
        cdparanoia_query_error(&lower_output)
    };

    CdDetectResult {
        drive_found,
        disc_found,
        tracks,
        raw_output,
        error,
    }
}

fn parse_cdparanoia_tracks(raw_output: &str) -> Vec<CdRipTrack> {
    raw_output
        .lines()
        .filter_map(parse_cdparanoia_track_line)
        .collect()
}

fn parse_cdparanoia_track_line(line: &str) -> Option<CdRipTrack> {
    let trimmed = line.trim_start();
    let first_token = trimmed.split_whitespace().next()?;
    let number = first_token.strip_suffix('.')?.parse::<u32>().ok()?;
    let duration = trimmed
        .find('[')
        .and_then(|start| {
            trimmed[start + 1..]
                .find(']')
                .map(|end| trimmed[start + 1..start + 1 + end].to_owned())
        })
        .filter(|value| !value.trim().is_empty());
    let duration_seconds = duration.as_deref().and_then(cd_duration_to_seconds);

    Some(CdRipTrack {
        number,
        duration,
        duration_seconds,
        status: Some("pending".to_owned()),
        output_filename: Some(cd_track_filename(number)),
        error: None,
        warning: None,
    })
}

fn cd_duration_to_seconds(value: &str) -> Option<u32> {
    let mut parts = value.split([':', '.']);
    let minutes = parts.next()?.parse::<u32>().ok()?;
    let seconds = parts.next()?.parse::<u32>().ok()?;

    Some(minutes.saturating_mul(60).saturating_add(seconds))
}

fn cdparanoia_query_error(lower_output: &str) -> Option<String> {
    if lower_output.contains("unable to open cdrom")
        || lower_output.contains("could not open")
        || lower_output.contains("no cdrom")
        || lower_output.contains("no such file or directory")
    {
        return Some("No CD drive was found or the drive could not be opened.".to_owned());
    }

    if lower_output.contains("no audio")
        || lower_output.contains("no table of contents")
        || lower_output.contains("no medium")
        || lower_output.contains("no disc")
        || lower_output.contains("not contain")
    {
        return Some("No audio CD was detected in the drive.".to_owned());
    }

    Some("No audio CD tracks were found. Insert an audio CD and try again.".to_owned())
}

fn rip_cd_to_flac_blocking(
    output_folder: String,
    metadata: Option<CdRipMetadata>,
    app: AppHandle,
) -> Result<CdRipResult, String> {
    ensure_command_available(
        "cdparanoia",
        "cdparanoia is not installed. Install cdparanoia and try again.",
    )?;
    ensure_command_available("flac", "flac is not installed. Install flac and try again.")?;

    let output_root = PathBuf::from(output_folder);
    if !output_root.exists() {
        return Err("Selected output folder does not exist.".to_owned());
    }
    if !output_root.is_dir() {
        return Err("Selected output path is not a folder.".to_owned());
    }

    let detection = run_cdparanoia_query()?;
    if !detection.drive_found {
        return Err(detection.error.unwrap_or_else(|| {
            "No CD drive was found or the drive could not be opened.".to_owned()
        }));
    }
    if !detection.disc_found {
        return Err(detection
            .error
            .unwrap_or_else(|| "No audio CD was detected in the drive.".to_owned()));
    }

    let (rip_folder_name, folder_warning) = cd_rip_folder_name(metadata.as_ref());
    let rip_folder = unique_child_folder(&output_root, &rip_folder_name);
    fs::create_dir_all(&rip_folder)
        .map_err(|error| format!("Could not create rip folder: {error}"))?;
    let rip_folder_string = rip_folder.to_string_lossy().to_string();
    let (prepared_cover, cover_warning) = prepare_cd_cover(metadata.as_ref(), &rip_folder);

    emit_cd_rip_event(
        &app,
        "cd-rip-started",
        CdRipEvent {
            output_folder: Some(rip_folder_string.clone()),
            track_number: None,
            output_filename: None,
            output_path: None,
            message: Some("CD rip started.".to_owned()),
        },
    );

    let track_total = detection.tracks.len() as u32;
    let mut ripped_tracks = Vec::with_capacity(detection.tracks.len());

    for detected_track in detection.tracks {
        let metadata_track = metadata
            .as_ref()
            .and_then(|metadata| metadata.tracks.iter().find(|track| track.number == detected_track.number));
        let (output_filename, filename_warning) =
            cd_track_filename_from_metadata(detected_track.number, metadata_track);
        let output_path = rip_folder.join(&output_filename);
        let wav_path = rip_folder.join(format!(
            "{:02} - Track {:02}.wav",
            detected_track.number, detected_track.number
        ));
        let output_path_string = output_path.to_string_lossy().to_string();

        emit_cd_rip_event(
            &app,
            "cd-rip-track-started",
            CdRipEvent {
                output_folder: Some(rip_folder_string.clone()),
                track_number: Some(detected_track.number),
                output_filename: Some(output_filename.clone()),
                output_path: Some(output_path_string.clone()),
                message: Some(format!("Ripping track {:02}.", detected_track.number)),
            },
        );

        let mut track_result = detected_track.clone();
        track_result.status = Some("ripping".to_owned());
        track_result.output_filename = Some(output_filename.clone());
        track_result.warning = merge_warnings(
            merge_warnings(folder_warning.clone(), filename_warning),
            cover_warning.clone(),
        );

        match rip_single_track_to_flac(
            detected_track.number,
            &wav_path,
            &output_path,
            metadata.as_ref(),
            metadata_track,
            track_total,
            prepared_cover.as_ref(),
        ) {
            Ok(tag_warning) => {
                track_result.status = Some("done".to_owned());
                track_result.error = None;
                if let Some(tag_warning) = tag_warning {
                    track_result.warning = merge_warnings(track_result.warning, Some(tag_warning));
                }
                emit_cd_rip_event(
                    &app,
                    "cd-rip-track-finished",
                    CdRipEvent {
                        output_folder: Some(rip_folder_string.clone()),
                        track_number: Some(detected_track.number),
                        output_filename: Some(output_filename),
                        output_path: Some(output_path_string),
                        message: Some(format!("Track {:02} finished.", detected_track.number)),
                    },
                );
            }
            Err(error) => {
                track_result.status = Some("error".to_owned());
                track_result.error = Some(error.clone());
                emit_cd_rip_event(
                    &app,
                    "cd-rip-track-error",
                    CdRipEvent {
                        output_folder: Some(rip_folder_string.clone()),
                        track_number: Some(detected_track.number),
                        output_filename: Some(output_filename),
                        output_path: Some(output_path_string),
                        message: Some(error),
                    },
                );
            }
        }

        ripped_tracks.push(track_result);
    }

    emit_cd_rip_event(
        &app,
        "cd-rip-finished",
        CdRipEvent {
            output_folder: Some(rip_folder_string.clone()),
            track_number: None,
            output_filename: None,
            output_path: None,
            message: Some("CD rip finished.".to_owned()),
        },
    );

    Ok(CdRipResult {
        output_folder: rip_folder_string,
        tracks: ripped_tracks,
    })
}

fn rip_single_track_to_flac(
    track_number: u32,
    wav_path: &Path,
    flac_path: &Path,
    metadata: Option<&CdRipMetadata>,
    metadata_track: Option<&CdRipMetadataTrack>,
    track_total: u32,
    cover: Option<&PreparedCdCover>,
) -> Result<Option<String>, String> {
    let rip_output = Command::new("cdparanoia")
        .arg(track_number.to_string())
        .arg(wav_path)
        .output()
        .map_err(|error| format!("Could not rip track {track_number:02}: {error}"))?;

    if !rip_output.status.success() {
        let _ = fs::remove_file(wav_path);
        return Err(format!(
            "Track {track_number:02} rip failed: {}",
            short_command_error(&rip_output)
        ));
    }

    let flac_output = Command::new("flac")
        .arg("-f")
        .arg("-o")
        .arg(flac_path)
        .arg(wav_path)
        .output()
        .map_err(|error| format!("Could not encode track {track_number:02} to FLAC: {error}"))?;

    if !flac_output.status.success() {
        return Err(format!(
            "Track {track_number:02} FLAC encoding failed: {}",
            short_command_error(&flac_output)
        ));
    }

    fs::remove_file(wav_path)
        .map_err(|error| format!("Track {track_number:02} was encoded, but the temporary WAV could not be removed: {error}"))?;

    let tag_warning = if let (Some(metadata), Some(metadata_track)) = (metadata, metadata_track) {
        write_flac_tags(flac_path, metadata, metadata_track, track_total, cover).err()
    } else {
        None
    };

    Ok(tag_warning.map(|error| {
        format!("Rip succeeded, but metadata tags could not be written: {error}")
    }))
}

fn ensure_command_available(command: &str, missing_message: &str) -> Result<(), String> {
    match Command::new(command).arg("--version").output() {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(missing_message.to_owned()),
        Err(error) => Err(format!("Could not run {command}: {error}")),
    }
}

fn command_output_text(output: &Output) -> String {
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text
}

fn short_command_error(output: &Output) -> String {
    let text = command_output_text(output);
    text.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("system command failed")
        .to_owned()
}

fn detect_dvd_blocking() -> DvdDetectResult {
    for device_path in ["/dev/dvd", "/dev/sr0", "/dev/cdrom"] {
        let path = Path::new(device_path);

        if !path.exists() {
            continue;
        }

        return DvdDetectResult {
            found: true,
            device_path: Some(device_path.to_owned()),
            readable: fs::File::open(path).is_ok(),
            error: None,
        };
    }

    DvdDetectResult {
        found: false,
        device_path: None,
        readable: false,
        error: Some("No DVD drive was found at /dev/dvd, /dev/sr0, or /dev/cdrom.".to_owned()),
    }
}

fn scan_dvd_titles_blocking(source: String) -> Result<DvdTitleScanResult, String> {
    ensure_command_available(
        "lsdvd",
        "lsdvd is not installed. Install lsdvd to scan readable DVD titles.",
    )?;
    let source_path = PathBuf::from(source);
    let source_type = dvd_source_type(&source_path)?;
    let output = Command::new("lsdvd")
        .arg(&source_path)
        .output()
        .map_err(|error| format!("Could not run lsdvd: {error}"))?;
    let raw_output = command_output_text(&output);

    if !output.status.success() {
        return Ok(DvdTitleScanResult {
            source_type,
            source_path: source_path.to_string_lossy().into_owned(),
            titles: Vec::new(),
            raw_output: Some(raw_output.clone()),
            error: Some(dvd_tool_error(&raw_output, "Could not scan DVD titles with lsdvd.")),
        });
    }

    let mut titles = parse_lsdvd_titles(&raw_output);
    mark_likely_main_title(&mut titles);
    let error = titles
        .is_empty()
        .then(|| "No DVD titles were found. The source may not be a readable video DVD.".to_owned());

    Ok(DvdTitleScanResult {
        source_type,
        source_path: source_path.to_string_lossy().into_owned(),
        titles,
        raw_output: Some(raw_output),
        error,
    })
}

fn import_dvd_title_blocking(
    source: String,
    title_number: u32,
    output_folder: String,
    metadata: DvdImportMetadata,
    app: AppHandle,
) -> Result<ImportedDvdVideo, String> {
    ensure_command_available(
        "ffmpeg",
        "ffmpeg is not installed. Install ffmpeg to import readable DVD titles.",
    )?;

    if title_number == 0 {
        return Err("Choose a DVD title before importing.".to_owned());
    }

    let source_path = PathBuf::from(source);
    let _source_type = dvd_source_type(&source_path)?;
    let output_root = PathBuf::from(output_folder);

    if !output_root.exists() {
        return Err("Selected DVD import output folder does not exist.".to_owned());
    }

    if !output_root.is_dir() {
        return Err("Selected DVD import output path is not a folder.".to_owned());
    }

    let title = clean_text(Some(metadata.title)).unwrap_or_else(|| format!("DVD Title {title_number:02}"));
    let artist = clean_text(Some(metadata.artist)).unwrap_or_else(|| "Unknown Artist".to_owned());
    let video_type = normalize_video_type(&metadata.video_type);
    let fallback_folder = format!("DVD Import {}", rip_folder_timestamp());
    let raw_folder = metadata
        .year
        .map(|year| format!("{artist} - {title} ({year})"))
        .unwrap_or_else(|| format!("{artist} - {title}"));
    let (folder_name, _) = sanitize_path_component(&raw_folder, &fallback_folder);
    let import_folder = unique_child_folder(&output_root, &folder_name);
    fs::create_dir_all(&import_folder)
        .map_err(|error| format!("Could not create DVD import folder: {error}"))?;

    let fallback_filename = format!("DVD Import {}.mkv", rip_folder_timestamp());
    let output_stem = metadata
        .output_filename
        .as_deref()
        .and_then(|value| clean_text(Some(value.to_owned())))
        .unwrap_or_else(|| raw_folder.clone());
    let (mut output_filename, _) = sanitize_path_component(&output_stem, &fallback_filename);

    if !output_filename.to_lowercase().ends_with(".mkv") {
        output_filename.push_str(".mkv");
    }

    let output_path = import_folder.join(output_filename);
    let _ = app.emit(
        "dvd-import-started",
        DvdImportEvent {
            output_folder: Some(import_folder.to_string_lossy().into_owned()),
            output_path: Some(output_path.to_string_lossy().into_owned()),
            title_number: Some(title_number),
            message: Some("DVD import started.".to_owned()),
        },
    );
    let _ = app.emit(
        "dvd-import-title-started",
        DvdImportEvent {
            output_folder: Some(import_folder.to_string_lossy().into_owned()),
            output_path: Some(output_path.to_string_lossy().into_owned()),
            title_number: Some(title_number),
            message: Some(format!("Importing DVD title {title_number:02}.")),
        },
    );

    let copy_result = run_dvd_ffmpeg_import(&source_path, title_number, &output_path, true);

    if let Err(copy_error) = copy_result {
        let _ = app.emit(
            "dvd-import-progress",
            DvdImportEvent {
                output_folder: Some(import_folder.to_string_lossy().into_owned()),
                output_path: Some(output_path.to_string_lossy().into_owned()),
                title_number: Some(title_number),
                message: Some("Stream copy failed; retrying with an encode fallback.".to_owned()),
            },
        );

        if is_dvd_unsupported_error(&copy_error) {
            let message = dvd_tool_error(&copy_error, "DVD import failed.");
            let _ = app.emit(
                "dvd-import-error",
                DvdImportEvent {
                    output_folder: Some(import_folder.to_string_lossy().into_owned()),
                    output_path: Some(output_path.to_string_lossy().into_owned()),
                    title_number: Some(title_number),
                    message: Some(message.clone()),
                },
            );
            return Err(message);
        }

        if let Err(encode_error) = run_dvd_ffmpeg_import(&source_path, title_number, &output_path, false) {
            let message = dvd_tool_error(&encode_error, "DVD import failed.");
            let _ = app.emit(
                "dvd-import-error",
                DvdImportEvent {
                    output_folder: Some(import_folder.to_string_lossy().into_owned()),
                    output_path: Some(output_path.to_string_lossy().into_owned()),
                    title_number: Some(title_number),
                    message: Some(message.clone()),
                },
            );
            return Err(message);
        }
    }

    let duration_seconds = ffprobe_video_duration_seconds(&output_path);
    let thumbnail_path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|path| path.join("video-thumbnails"))
        .and_then(|thumbnail_dir| generate_video_thumbnail(&output_path, duration_seconds, &thumbnail_dir));
    let now = unix_timestamp();
    let file_name = output_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dvd-import.mkv".to_owned());
    let video = VideoEntry {
        id: output_path.to_string_lossy().into_owned(),
        file_path: output_path.to_string_lossy().into_owned(),
        file_name,
        title,
        artist: Some(artist),
        video_type,
        source: normalize_video_source("dvd_import"),
        release_or_collection: clean_text(metadata.release_or_collection),
        year: metadata.year,
        venue: clean_text(metadata.venue),
        city: clean_text(metadata.city),
        country: clean_text(metadata.country),
        description_or_notes: clean_text(metadata.description_or_notes),
        duration_seconds,
        thumbnail_path,
        last_position_seconds: 0,
        play_count: 0,
        last_played_at: None,
        created_at: now,
        updated_at: now,
    };

    let _ = app.emit(
        "dvd-import-finished",
        DvdImportEvent {
            output_folder: Some(import_folder.to_string_lossy().into_owned()),
            output_path: Some(output_path.to_string_lossy().into_owned()),
            title_number: Some(title_number),
            message: Some("DVD import complete.".to_owned()),
        },
    );

    Ok(ImportedDvdVideo {
        video,
        output_folder: import_folder.to_string_lossy().into_owned(),
        output_path: output_path.to_string_lossy().into_owned(),
    })
}

fn run_dvd_ffmpeg_import(
    source_path: &Path,
    title_number: u32,
    output_path: &Path,
    stream_copy: bool,
) -> Result<(), String> {
    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-hide_banner", "-loglevel", "info", "-f", "dvdvideo", "-title"])
        .arg(title_number.to_string())
        .arg("-i")
        .arg(source_path)
        .args(["-map", "0"]);

    if stream_copy {
        command.args(["-c", "copy"]);
    } else {
        command.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "18", "-c:a", "aac", "-b:a", "192k"]);
    }

    let output = command
        .arg(output_path)
        .output()
        .map_err(|error| format!("Could not run ffmpeg: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_output_text(&output))
    }
}

fn dvd_source_type(path: &Path) -> Result<String, String> {
    if path.is_dir() {
        if is_video_ts_folder(path) {
            return Ok("video_ts_folder".to_owned());
        }

        return Err("Selected folder is not a VIDEO_TS folder.".to_owned());
    }

    if path.exists() {
        return Ok("physical_device".to_owned());
    }

    Err("DVD source does not exist.".to_owned())
}

fn is_video_ts_folder(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("VIDEO_TS"))
        .unwrap_or(false)
        && path.join("VIDEO_TS.IFO").exists()
}

fn parse_lsdvd_titles(raw_output: &str) -> Vec<DvdTitle> {
    let mut titles = Vec::new();

    for line in raw_output.lines().map(str::trim) {
        if !line.starts_with("Title:") {
            continue;
        }

        let Some(number) = parse_lsdvd_title_number(line) else {
            continue;
        };
        let duration = parse_lsdvd_field(line, "Length:");
        let duration_seconds = duration.as_deref().and_then(parse_dvd_duration_seconds);
        let chapters = parse_lsdvd_field(line, "Chapters:")
            .and_then(|value| value.parse::<u32>().ok());

        titles.push(DvdTitle {
            number,
            duration,
            duration_seconds,
            chapters,
            likely_main_title: false,
        });
    }

    titles
}

fn parse_lsdvd_title_number(line: &str) -> Option<u32> {
    let value = line.strip_prefix("Title:")?.trim();
    let number = value
        .split([',', ' '])
        .find(|part| !part.trim().is_empty())?
        .trim()
        .trim_start_matches('0');

    if number.is_empty() {
        Some(0)
    } else {
        number.parse::<u32>().ok()
    }
}

fn parse_lsdvd_field(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let value = line[start..]
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    Some(value.to_owned())
}

fn parse_dvd_duration_seconds(value: &str) -> Option<u32> {
    let time = value.split('.').next().unwrap_or(value);
    let parts = time
        .split(':')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [hours, minutes, seconds] => Some(hours * 3600 + minutes * 60 + seconds),
        [minutes, seconds] => Some(minutes * 60 + seconds),
        [seconds] => Some(*seconds),
        _ => None,
    }
}

fn mark_likely_main_title(titles: &mut [DvdTitle]) {
    let Some((index, _)) = titles
        .iter()
        .enumerate()
        .max_by_key(|(_, title)| title.duration_seconds.unwrap_or(0))
    else {
        return;
    };

    if titles[index].duration_seconds.unwrap_or(0) > 0 {
        titles[index].likely_main_title = true;
    }
}

fn is_dvd_unsupported_error(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("encrypted")
        || lower.contains("libdvdcss")
        || lower.contains("css")
        || lower.contains("scrambled")
        || lower.contains("permission denied")
        || lower.contains("input/output error")
}

fn dvd_tool_error(output: &str, fallback: &str) -> String {
    if is_dvd_unsupported_error(output) {
        return "This DVD is not readable by system tools. Cassette does not bypass DVD DRM.".to_owned();
    }

    output
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| format!("{fallback} {line}"))
        .unwrap_or_else(|| fallback.to_owned())
}

fn cd_track_filename(track_number: u32) -> String {
    format!("{track_number:02} - Track {track_number:02}.flac")
}

fn cd_track_filename_from_metadata(
    track_number: u32,
    metadata_track: Option<&CdRipMetadataTrack>,
) -> (String, Option<String>) {
    let fallback = format!("Track {track_number:02}");
    let title = metadata_track
        .map(|track| track.title.as_str())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(&fallback);
    let (safe_title, warning) = sanitize_path_component(title, &fallback);

    (format!("{track_number:02} - {safe_title}.flac"), warning)
}

fn cd_rip_folder_name(metadata: Option<&CdRipMetadata>) -> (String, Option<String>) {
    let Some(metadata) = metadata else {
        return (format!("Cassette Rip {}", rip_folder_timestamp()), None);
    };
    let artist = metadata.album_artist.trim();
    let album = metadata.album_title.trim();

    if artist.is_empty() || album.is_empty() {
        return (format!("Cassette Rip {}", rip_folder_timestamp()), None);
    }

    let year = metadata.year.trim();
    let raw_name = if year.is_empty() {
        format!("{artist} - {album}")
    } else {
        format!("{artist} - {album} ({year})")
    };

    sanitize_path_component(&raw_name, &format!("Cassette Rip {}", rip_folder_timestamp()))
}

fn unique_child_folder(parent: &Path, folder_name: &str) -> PathBuf {
    let first = parent.join(folder_name);
    if !first.exists() {
        return first;
    }

    for index in 2..1000 {
        let candidate = parent.join(format!("{folder_name} ({index})"));
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{folder_name} {}", rip_folder_timestamp()))
}

fn prepare_cd_cover(
    metadata: Option<&CdRipMetadata>,
    rip_folder: &Path,
) -> (Option<PreparedCdCover>, Option<String>) {
    let Some(cover) = metadata.and_then(|metadata| metadata.cover.as_ref()) else {
        return (None, None);
    };

    let data = match fs::read(&cover.path) {
        Ok(data) => data,
        Err(error) => {
            return (
                None,
                Some(format!("Cover image could not be read and was not embedded: {error}")),
            );
        }
    };
    let Some((mime_type, extension)) = cover_mime_and_extension(&data, Some(&cover.mime_type))
    else {
        return (
            None,
            Some("Cover image type is unsupported and was not embedded.".to_owned()),
        );
    };
    let save_warning = fs::write(rip_folder.join(format!("cover.{extension}")), &data)
        .err()
        .map(|error| format!("Cover image could not be saved in the rip folder: {error}"));

    (
        Some(PreparedCdCover {
            data,
            mime_type,
        }),
        save_warning,
    )
}

fn merge_warnings(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(format!("{left} {right}")),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn sanitize_path_component(raw: &str, fallback: &str) -> (String, Option<String>) {
    let mut safe = String::with_capacity(raw.len());

    for character in raw.chars() {
        if character == '/' || character == '\\' {
            safe.push('-');
        } else if !character.is_control() {
            safe.push(character);
        }
    }

    while safe.contains("  ") {
        safe = safe.replace("  ", " ");
    }

    let safe = safe
        .trim()
        .trim_matches(['.', ' '])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if safe.is_empty() {
        return (
            fallback.to_owned(),
            Some(format!("Filename sanitization fallback used for {raw:?}.")),
        );
    }

    let warning = (safe != raw.trim()).then(|| {
        format!("Filename characters were sanitized for {raw:?}.")
    });

    (safe, warning)
}

fn write_flac_tags(
    path: &Path,
    metadata: &CdRipMetadata,
    metadata_track: &CdRipMetadataTrack,
    track_total: u32,
    cover: Option<&PreparedCdCover>,
) -> Result<(), String> {
    let mut tagged_file = lofty::read_from_path(path)
        .map_err(|error| format!("Could not read encoded FLAC for tagging: {error}"))?;
    let mut tag = Tag::new(TagType::VorbisComments);

    if !metadata_track.title.trim().is_empty() {
        tag.set_title(metadata_track.title.trim().to_owned());
    }

    if !metadata_track.artist.trim().is_empty() {
        tag.set_artist(metadata_track.artist.trim().to_owned());
    } else if !metadata.album_artist.trim().is_empty() {
        tag.set_artist(metadata.album_artist.trim().to_owned());
    }

    if !metadata.album_title.trim().is_empty() {
        tag.set_album(metadata.album_title.trim().to_owned());
    }

    if !metadata.album_artist.trim().is_empty() {
        tag.insert_text(ItemKey::AlbumArtist, metadata.album_artist.trim().to_owned());
    }

    if !metadata.year.trim().is_empty() {
        tag.insert_text(ItemKey::RecordingDate, metadata.year.trim().to_owned());
        tag.insert_text(ItemKey::Year, metadata.year.trim().to_owned());
    }

    tag.set_track(metadata_track.number);
    tag.set_track_total(track_total);

    if let Some(disc_number) = metadata_track.disc_number.or(metadata.disc_number) {
        tag.set_disk(disc_number);
    }

    if !metadata.genre.trim().is_empty() {
        tag.set_genre(metadata.genre.trim().to_owned());
    }

    if let Some(cover) = cover {
        let picture = Picture::unchecked(cover.data.clone())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::from_str(&cover.mime_type))
            .description("Cover")
            .build();
        tag.push_picture(picture);
    }

    tagged_file.insert_tag(tag);
    tagged_file
        .save_to_path(path, WriteOptions::default())
        .map_err(|error| format!("Could not save FLAC tags: {error}"))
}

fn rip_folder_timestamp() -> String {
    let timestamp = unix_timestamp().max(0);
    let days = timestamp / 86_400;
    let seconds_of_day = timestamp % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02} {hour:02}-{minute:02}-{second:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month, day)
}

fn emit_cd_rip_event(app: &AppHandle, event: &str, payload: CdRipEvent) {
    let _ = app.emit(event, payload);
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

            CREATE TABLE IF NOT EXISTS track_lyrics_settings (
                track_id TEXT PRIMARY KEY NOT NULL,
                offset_seconds REAL NOT NULL DEFAULT 0,
                preferred_source TEXT CHECK(preferred_source IN ('local', 'cached_lrclib') OR preferred_source IS NULL),
                updated_at INTEGER NOT NULL
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

            CREATE TABLE IF NOT EXISTS videos (
                id TEXT PRIMARY KEY NOT NULL,
                file_path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT,
                show_title TEXT,
                album_or_release TEXT,
                video_type TEXT NOT NULL DEFAULT 'other',
                source TEXT NOT NULL DEFAULT 'local_file',
                release_or_collection TEXT,
                year INTEGER,
                venue TEXT,
                city TEXT,
                country TEXT,
                description_or_notes TEXT,
                duration_seconds INTEGER,
                thumbnail_path TEXT,
                last_position_seconds INTEGER NOT NULL DEFAULT 0,
                play_count INTEGER NOT NULL DEFAULT 0,
                last_played_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
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

        if !self.has_column("track_lyrics_settings", "preferred_source")? {
            self.connection.execute(
                "ALTER TABLE track_lyrics_settings ADD COLUMN preferred_source TEXT",
                [],
            )?;
        }

        if !self.has_column("videos", "video_type")? {
            self.connection.execute(
                "ALTER TABLE videos ADD COLUMN video_type TEXT NOT NULL DEFAULT 'live_show'",
                [],
            )?;
        }

        if !self.has_column("videos", "source")? {
            self.connection.execute(
                "ALTER TABLE videos ADD COLUMN source TEXT NOT NULL DEFAULT 'local_file'",
                [],
            )?;
        }

        if !self.has_column("videos", "release_or_collection")? {
            self.connection
                .execute("ALTER TABLE videos ADD COLUMN release_or_collection TEXT", [])?;
            if self.has_column("videos", "album_or_release")? {
                self.connection.execute(
                    "
                    UPDATE videos
                    SET release_or_collection = album_or_release
                    WHERE release_or_collection IS NULL
                        AND album_or_release IS NOT NULL
                        AND trim(album_or_release) != ''
                    ",
                    [],
                )?;
            }
        }

        if !self.has_column("videos", "description_or_notes")? {
            self.connection
                .execute("ALTER TABLE videos ADD COLUMN description_or_notes TEXT", [])?;
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

    fn video_library(&self) -> Result<VideoLibrary, String> {
        let mut statement = self
            .connection
            .prepare(
                "
                SELECT
                    id,
                    file_path,
                    file_name,
                    title,
                    artist,
                    video_type,
                    source,
                    release_or_collection,
                    year,
                    venue,
                    city,
                    country,
                    description_or_notes,
                    duration_seconds,
                    thumbnail_path,
                    last_position_seconds,
                    play_count,
                    last_played_at,
                    created_at,
                    updated_at
                FROM videos
                ORDER BY title COLLATE NOCASE ASC, file_name COLLATE NOCASE ASC
                ",
            )
            .map_err(|error| format!("Could not read video library: {error}"))?;
        let videos = statement
            .query_map([], row_to_video)
            .map_err(|error| format!("Could not read video library: {error}"))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read video rows: {error}"))?;

        Ok(VideoLibrary {
            videos,
            last_video_folder: self.meta_value("last_video_folder")?,
            last_video_scanned_at: self
                .meta_value("last_video_scanned_at")?
                .and_then(|value| value.parse::<i64>().ok()),
        })
    }

    fn replace_videos(
        &mut self,
        root_path: &Path,
        videos: &mut [VideoEntry],
        scanned_at: i64,
    ) -> Result<(), String> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| format!("Could not update video library: {error}"))?;
        let existing_videos = existing_videos_by_id(&transaction)
            .map_err(|error| format!("Could not read existing videos: {error}"))?;
        let scanned_ids = videos
            .iter()
            .map(|video| video.id.clone())
            .collect::<HashSet<_>>();

        for video in videos.iter_mut() {
            if let Some(existing) = existing_videos.get(&video.id) {
                video.title = existing.title.clone();
                video.artist = existing.artist.clone();
                video.video_type = existing.video_type.clone();
                video.source = existing.source.clone();
                video.release_or_collection = existing.release_or_collection.clone();
                video.year = existing.year;
                video.venue = existing.venue.clone();
                video.city = existing.city.clone();
                video.country = existing.country.clone();
                video.description_or_notes = existing.description_or_notes.clone();
                video.last_position_seconds = existing.last_position_seconds;
                video.play_count = existing.play_count;
                video.last_played_at = existing.last_played_at;
                video.created_at = existing.created_at;
            }
        }

        {
            let mut statement = transaction
                .prepare(
                    "
                    INSERT INTO videos (
                        id,
                        file_path,
                        file_name,
                        title,
                        artist,
                        video_type,
                        source,
                        release_or_collection,
                        year,
                        venue,
                        city,
                        country,
                        description_or_notes,
                        duration_seconds,
                        thumbnail_path,
                        last_position_seconds,
                        play_count,
                        last_played_at,
                        created_at,
                        updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                    ON CONFLICT(id) DO UPDATE SET
                        file_path = excluded.file_path,
                        file_name = excluded.file_name,
                        title = excluded.title,
                        artist = excluded.artist,
                        video_type = excluded.video_type,
                        source = excluded.source,
                        release_or_collection = excluded.release_or_collection,
                        year = excluded.year,
                        venue = excluded.venue,
                        city = excluded.city,
                        country = excluded.country,
                        description_or_notes = excluded.description_or_notes,
                        duration_seconds = excluded.duration_seconds,
                        thumbnail_path = excluded.thumbnail_path,
                        last_position_seconds = excluded.last_position_seconds,
                        play_count = excluded.play_count,
                        last_played_at = excluded.last_played_at,
                        created_at = excluded.created_at,
                        updated_at = excluded.updated_at
                    ",
                )
                .map_err(|error| format!("Could not prepare video cache update: {error}"))?;

            for video in videos.iter() {
                statement
                    .execute(params![
                        &video.id,
                        &video.file_path,
                        &video.file_name,
                        &video.title,
                        &video.artist,
                        &video.video_type,
                        &video.source,
                        &video.release_or_collection,
                        video.year,
                        &video.venue,
                        &video.city,
                        &video.country,
                        &video.description_or_notes,
                        video.duration_seconds,
                        &video.thumbnail_path,
                        video.last_position_seconds,
                        video.play_count,
                        video.last_played_at,
                        video.created_at,
                        video.updated_at,
                    ])
                    .map_err(|error| format!("Could not cache scanned video: {error}"))?;
            }
        }

        let root_prefix = root_path.to_string_lossy().into_owned();
        let existing_under_root = existing_videos
            .values()
            .filter(|video| video.file_path.starts_with(&root_prefix))
            .map(|video| video.id.clone())
            .collect::<Vec<_>>();

        for id in existing_under_root {
            if !scanned_ids.contains(&id) {
                transaction
                    .execute("DELETE FROM videos WHERE id = ?1", [&id])
                    .map_err(|error| format!("Could not remove missing video: {error}"))?;
            }
        }

        upsert_meta(
            &transaction,
            "last_video_folder",
            &root_path.to_string_lossy(),
        )
        .map_err(|error| format!("Could not cache video folder: {error}"))?;
        upsert_meta(&transaction, "last_video_scanned_at", &scanned_at.to_string())
            .map_err(|error| format!("Could not cache video scan time: {error}"))?;

        transaction
            .commit()
            .map_err(|error| format!("Could not save video library: {error}"))?;

        Ok(())
    }

    fn video_by_id(&self, id: &str) -> Result<Option<VideoEntry>, String> {
        self.connection
            .query_row(
                "
                SELECT
                    id,
                    file_path,
                    file_name,
                    title,
                    artist,
                    video_type,
                    source,
                    release_or_collection,
                    year,
                    venue,
                    city,
                    country,
                    description_or_notes,
                    duration_seconds,
                    thumbnail_path,
                    last_position_seconds,
                    play_count,
                    last_played_at,
                    created_at,
                    updated_at
                FROM videos
                WHERE id = ?1
                ",
                [id],
                row_to_video,
            )
            .optional()
            .map_err(|error| format!("Could not read video: {error}"))
    }

    fn upsert_video(&mut self, video: VideoEntry) -> Result<VideoEntry, String> {
        self.connection
            .execute(
                "
                INSERT INTO videos (
                    id,
                    file_path,
                    file_name,
                    title,
                    artist,
                    video_type,
                    source,
                    release_or_collection,
                    year,
                    venue,
                    city,
                    country,
                    description_or_notes,
                    duration_seconds,
                    thumbnail_path,
                    last_position_seconds,
                    play_count,
                    last_played_at,
                    created_at,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                ON CONFLICT(id) DO UPDATE SET
                    file_path = excluded.file_path,
                    file_name = excluded.file_name,
                    title = excluded.title,
                    artist = excluded.artist,
                    video_type = excluded.video_type,
                    source = excluded.source,
                    release_or_collection = excluded.release_or_collection,
                    year = excluded.year,
                    venue = excluded.venue,
                    city = excluded.city,
                    country = excluded.country,
                    description_or_notes = excluded.description_or_notes,
                    duration_seconds = excluded.duration_seconds,
                    thumbnail_path = excluded.thumbnail_path,
                    updated_at = excluded.updated_at
                ",
                params![
                    &video.id,
                    &video.file_path,
                    &video.file_name,
                    &video.title,
                    &video.artist,
                    &video.video_type,
                    &video.source,
                    &video.release_or_collection,
                    video.year,
                    &video.venue,
                    &video.city,
                    &video.country,
                    &video.description_or_notes,
                    video.duration_seconds,
                    &video.thumbnail_path,
                    video.last_position_seconds,
                    video.play_count,
                    video.last_played_at,
                    video.created_at,
                    video.updated_at,
                ],
            )
            .map_err(|error| format!("Could not save imported video: {error}"))?;

        self.video_by_id(&video.id)?
            .ok_or_else(|| "Imported video was not saved.".to_owned())
    }

    fn update_video_info(&self, id: &str, info: VideoInfoUpdate) -> Result<VideoEntry, String> {
        let title = clean_text(Some(info.title)).unwrap_or_else(|| "Untitled Video".to_owned());
        let updated = self
            .connection
            .execute(
                "
                UPDATE videos
                SET title = ?2,
                    artist = ?3,
                    video_type = ?4,
                    release_or_collection = ?5,
                    year = ?6,
                    venue = ?7,
                    city = ?8,
                    country = ?9,
                    description_or_notes = ?10,
                    updated_at = ?11
                WHERE id = ?1
                ",
                params![
                    id,
                    title,
                    clean_text(info.artist),
                    normalize_video_type(&info.video_type),
                    clean_text(info.release_or_collection),
                    info.year,
                    clean_text(info.venue),
                    clean_text(info.city),
                    clean_text(info.country),
                    clean_text(info.description_or_notes),
                    unix_timestamp(),
                ],
            )
            .map_err(|error| format!("Could not save video info: {error}"))?;

        if updated == 0 {
            return Err("Video is not in the library.".to_owned());
        }

        self.video_by_id(id)?
            .ok_or_else(|| "Video is not in the library.".to_owned())
    }

    fn update_video_progress(
        &self,
        id: &str,
        last_position_seconds: u32,
        increment_play_count: bool,
    ) -> Result<VideoEntry, String> {
        let played_at = if increment_play_count {
            Some(unix_timestamp())
        } else {
            None
        };
        let updated = if increment_play_count {
            self.connection.execute(
                "
                UPDATE videos
                SET last_position_seconds = ?2,
                    play_count = play_count + 1,
                    last_played_at = ?3,
                    updated_at = ?3
                WHERE id = ?1
                ",
                params![id, last_position_seconds, played_at],
            )
        } else {
            self.connection.execute(
                "
                UPDATE videos
                SET last_position_seconds = ?2,
                    updated_at = ?3
                WHERE id = ?1
                ",
                params![id, last_position_seconds, unix_timestamp()],
            )
        }
        .map_err(|error| format!("Could not save video progress: {error}"))?;

        if updated == 0 {
            return Err("Video is not in the library.".to_owned());
        }

        self.video_by_id(id)?
            .ok_or_else(|| "Video is not in the library.".to_owned())
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

    fn update_cached_track(&mut self, track: &mut Track) -> Result<(), String> {
        let favorite_track_ids = favorite_track_ids(&self.connection)
            .map_err(|error| format!("Could not read favorite tracks: {error}"))?;
        let playback_history = playback_history_by_id(&self.connection)
            .map_err(|error| format!("Could not read playback history: {error}"))?;
        track.is_favorite = track.is_favorite || favorite_track_ids.contains(&track.id);
        if let Some(history) = playback_history.get(&track.id) {
            track.play_count = history.play_count;
            track.last_played_at = history.last_played_at;
        }

        let genres_json = serde_json::to_string(&track.genres).unwrap_or_else(|_| "[]".to_owned());
        self.connection
            .execute(
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
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    artist = excluded.artist,
                    album = excluded.album,
                    album_artist = excluded.album_artist,
                    genres = excluded.genres,
                    track_number = excluded.track_number,
                    disc_number = excluded.disc_number,
                    year = excluded.year,
                    duration_seconds = excluded.duration_seconds,
                    file_path = excluded.file_path,
                    file_name = excluded.file_name,
                    extension = excluded.extension,
                    modified_time = excluded.modified_time,
                    file_size = excluded.file_size,
                    scanned_at = excluded.scanned_at,
                    cover_art_path = excluded.cover_art_path,
                    lyrics_path = excluded.lyrics_path,
                    lyrics_kind = excluded.lyrics_kind,
                    is_favorite = excluded.is_favorite,
                    play_count = excluded.play_count,
                    last_played_at = excluded.last_played_at
                ",
                params![
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
                ],
            )
            .map_err(|error| format!("Could not update cached track: {error}"))?;

        let genre_assignments = self.genre_assignments()?;
        apply_genre_assignments(std::slice::from_mut(track), &genre_assignments);

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

    fn lyrics_settings(&self, track_id: &str) -> Result<TrackLyricsSettings, String> {
        self.connection
            .query_row(
                "
                SELECT offset_seconds, preferred_source
                FROM track_lyrics_settings
                WHERE track_id = ?1
                ",
                [track_id],
                |row| {
                    let preferred_source = row
                        .get::<_, Option<String>>(1)?
                        .filter(|source| matches!(source.as_str(), "local" | "cached_lrclib"));
                    Ok(TrackLyricsSettings {
                        offset_seconds: row.get::<_, f64>(0)?,
                        preferred_source,
                    })
                },
            )
            .optional()
            .map(|settings| settings.unwrap_or_default())
            .map_err(|error| format!("Could not read lyrics settings: {error}"))
    }

    fn set_lyrics_offset(&self, track_id: &str, offset_seconds: f64) -> Result<f64, String> {
        let clamped_offset = offset_seconds.clamp(-5.0, 5.0);

        self.connection
            .execute(
                "
                INSERT INTO track_lyrics_settings (track_id, offset_seconds, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(track_id) DO UPDATE SET
                    offset_seconds = excluded.offset_seconds,
                    updated_at = excluded.updated_at
                ",
                params![track_id, clamped_offset, unix_timestamp()],
            )
            .map_err(|error| format!("Could not save lyrics offset: {error}"))?;

        Ok(clamped_offset)
    }

    fn set_lyrics_preferred_source(
        &self,
        track_id: &str,
        preferred_source: Option<&str>,
    ) -> Result<(), String> {
        if let Some(source) = preferred_source {
            if !matches!(source, "local" | "cached_lrclib") {
                return Err("Unsupported lyrics source preference.".to_owned());
            }
        }

        self.connection
            .execute(
                "
                INSERT INTO track_lyrics_settings (track_id, preferred_source, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(track_id) DO UPDATE SET
                    preferred_source = excluded.preferred_source,
                    updated_at = excluded.updated_at
                ",
                params![track_id, preferred_source, unix_timestamp()],
            )
            .map_err(|error| format!("Could not save lyrics source preference: {error}"))?;

        Ok(())
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

    fn shutdown(&mut self) {
        if let Some(playbin) = self.playbin.take() {
            let _ = set_gst_state(&playbin, gst::State::Null);
            drain_playback_bus(&playbin);
        }

        self.current_path = None;
        self.is_playing = false;
        self.has_ended = false;
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

impl VideoPlaybackState {
    fn play(
        &mut self,
        video: &VideoEntry,
        start_position_seconds: Option<f64>,
    ) -> Result<VideoPlaybackStatus, String> {
        let path = PathBuf::from(&video.file_path);

        if !path.exists() {
            return Err("Selected video no longer exists.".to_owned());
        }

        if !path.is_file() {
            return Err("Selected video is not a file.".to_owned());
        }

        self.stop_process();
        self.last_error = None;

        ensure_mpv_available(&video.file_path)?;

        let ipc_path = mpv_ipc_path();
        let log_path = mpv_log_path();
        let _ = fs::remove_file(&ipc_path);
        let _ = fs::remove_file(&log_path);
        let start_position = start_position_seconds.unwrap_or(0.0).max(0.0);
        let title = video_window_title(video);
        let mut command = Command::new("mpv");
        command
            .arg("--force-window=yes")
            .arg("--keep-open=yes")
            .arg("--input-terminal=no")
            .arg("--no-terminal")
            .arg(format!("--input-ipc-server={}", ipc_path.to_string_lossy()))
            .arg(format!("--log-file={}", log_path.to_string_lossy()))
            .arg(format!("--title={title}"))
            .arg("--autofit=1280x720")
            .arg("--autofit-smaller=960x540")
            .arg("--geometry=50%:50%")
            .arg(format!("--start={start_position:.3}"))
            .arg(format!("--volume={}", (self.volume_or_default() * 100.0).round()))
            .arg(&video.file_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = command
            .spawn()
            .map_err(|error| video_playback_error("mpv", &video.file_path, &format!("Could not start mpv: {error}")))?;

        wait_for_mpv_ipc(&mut child, &ipc_path, &log_path, &video.file_path)?;

        self.child = Some(child);
        self.ipc_path = Some(ipc_path);
        self.log_path = Some(log_path);
        self.current_video_id = Some(video.id.clone());
        self.current_path = Some(video.file_path.clone());
        self.is_playing = true;
        self.has_ended = false;
        self.position_seconds = start_position.floor() as u64;
        self.duration_seconds = video.duration_seconds.map(u64::from);
        self.has_video_window = true;
        self.is_fullscreen = false;
        self.update_from_mpv()?;

        Ok(self.status())
    }

    fn pause(&mut self) -> Result<VideoPlaybackStatus, String> {
        if self.child.is_some() {
            self.mpv_command(serde_json::json!(["set_property", "pause", true]))?;
            self.is_playing = false;
            self.update_from_mpv()?;
        }

        Ok(self.status())
    }

    fn resume(&mut self) -> Result<VideoPlaybackStatus, String> {
        if self.child.is_some() {
            self.mpv_command(serde_json::json!(["set_property", "pause", false]))?;
            self.is_playing = self.current_path.is_some();
            self.has_ended = false;
            self.update_from_mpv()?;
        }

        Ok(self.status())
    }

    fn stop(&mut self) -> Result<VideoPlaybackStatus, String> {
        self.stop_process();
        self.clear_active_video(false);

        Ok(self.status())
    }

    fn refresh(&mut self) -> Option<VideoProgressSnapshot> {
        if self.child.is_none() {
            return None;
        }

        if self.process_has_exited() {
            let snapshot = self.progress_snapshot();
            self.clear_active_video(false);
            return snapshot;
        }

        if let Err(error) = self.update_from_mpv() {
            self.last_error = Some(error);
        }

        None
    }

    fn seek(&mut self, position_seconds: f64) -> Result<VideoPlaybackStatus, String> {
        if !position_seconds.is_finite() || self.current_path.is_none() {
            return Ok(self.status());
        }

        if self.child.is_none() {
            return Ok(self.status());
        }
        let duration = self
            .duration_seconds
            .map(|duration| duration as f64)
            .filter(|duration| *duration > 0.0)
            .unwrap_or(position_seconds.max(0.0));
        let clamped_position_seconds = position_seconds.clamp(0.0, duration);
        self.mpv_command(serde_json::json!(["seek", clamped_position_seconds, "absolute", "exact"]))?;
        self.has_ended = false;
        self.position_seconds = clamped_position_seconds.floor() as u64;
        self.update_from_mpv()?;

        Ok(self.status())
    }

    fn set_volume(&mut self, volume: f64) -> Result<VideoPlaybackStatus, String> {
        let volume = volume.clamp(0.0, 1.0);
        self.volume = volume;

        if self.child.is_some() {
            self.mpv_command(serde_json::json!(["set_property", "volume", volume * 100.0]))?;
            self.update_from_mpv()?;
        }

        Ok(self.status())
    }

    fn bring_to_front(&mut self) -> Result<VideoPlaybackStatus, String> {
        if self.child.is_some() {
            let _ = self.mpv_command(serde_json::json!(["set_property", "window-minimized", false]));
            let _ = self.mpv_command(serde_json::json!(["set_property", "ontop", true]));
            let _ = self.mpv_command(serde_json::json!(["set_property", "ontop", false]));
            self.update_from_mpv()?;
        }

        Ok(self.status())
    }

    fn toggle_fullscreen(&mut self) -> Result<VideoPlaybackStatus, String> {
        if self.child.is_some() {
            let next_fullscreen = !self.is_fullscreen;
            self.mpv_command(serde_json::json!(["set_property", "fullscreen", next_fullscreen]))?;
            self.is_fullscreen = next_fullscreen;
            self.update_from_mpv()?;
        }

        Ok(self.status())
    }

    fn update_from_mpv(&mut self) -> Result<(), String> {
        if self.process_has_exited() {
            return Ok(());
        }

        let pause = self
            .mpv_get_property("pause")
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(!self.is_playing);
        let eof_reached = self
            .mpv_get_property("eof-reached")
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        if let Some(position) = self
            .mpv_get_property("time-pos")
            .ok()
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value >= 0.0)
        {
            self.position_seconds = position.floor() as u64;
        }

        if let Some(duration) = self
            .mpv_get_property("duration")
            .ok()
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            self.duration_seconds = Some(duration.round() as u64);
        }

        if let Some(volume) = self
            .mpv_get_property("volume")
            .ok()
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite())
        {
            self.volume = (volume / 100.0).clamp(0.0, 1.0);
        }

        if let Some(fullscreen) = self
            .mpv_get_property("fullscreen")
            .ok()
            .and_then(|value| value.as_bool())
        {
            self.is_fullscreen = fullscreen;
        }

        self.has_ended = eof_reached;
        self.is_playing = self.current_path.is_some() && !pause && !eof_reached;
        self.has_video_window = self.current_path.is_some() && self.child.is_some();
        self.last_error = None;

        Ok(())
    }

    fn mpv_get_property(&mut self, property: &str) -> Result<serde_json::Value, String> {
        self.mpv_command(serde_json::json!(["get_property", property]))
    }

    #[cfg(unix)]
    fn mpv_command(&mut self, command: serde_json::Value) -> Result<serde_json::Value, String> {
        let Some(ipc_path) = self.ipc_path.as_ref() else {
            return Err("mpv IPC is not available.".to_owned());
        };
        let file_path = self.current_path.as_deref().unwrap_or("unknown video");
        self.request_id = self.request_id.wrapping_add(1).max(1);
        let request_id = self.request_id;
        let mut stream = UnixStream::connect(ipc_path)
            .map_err(|error| video_playback_error("mpv", file_path, &format!("Could not connect to mpv IPC: {error}")))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(1200)))
            .ok();
        stream
            .set_write_timeout(Some(Duration::from_millis(1200)))
            .ok();
        let request = serde_json::json!({
            "command": command,
            "request_id": request_id,
        });
        writeln!(stream, "{request}")
            .map_err(|error| video_playback_error("mpv", file_path, &format!("Could not send mpv command: {error}")))?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader
                .read_line(&mut line)
                .map_err(|error| video_playback_error("mpv", file_path, &format!("Could not read mpv response: {error}")))?;

            if bytes == 0 {
                return Err(video_playback_error("mpv", file_path, "mpv closed the IPC connection."));
            }

            let response = serde_json::from_str::<serde_json::Value>(&line)
                .map_err(|error| video_playback_error("mpv", file_path, &format!("Could not parse mpv response: {error}")))?;

            if response
                .get("request_id")
                .and_then(serde_json::Value::as_u64)
                != Some(request_id)
            {
                continue;
            }

            let error = response
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");

            if error != "success" {
                return Err(video_playback_error(
                    "mpv",
                    file_path,
                    &format!("mpv command failed: {error}"),
                ));
            }

            return Ok(response.get("data").cloned().unwrap_or(serde_json::Value::Null));
        }
    }

    #[cfg(not(unix))]
    fn mpv_command(&mut self, _command: serde_json::Value) -> Result<serde_json::Value, String> {
        Err("Cassette video playback through mpv IPC is only implemented on Unix-like systems.".to_owned())
    }

    fn process_has_exited(&mut self) -> bool {
        match self.child.as_mut().and_then(|child| child.try_wait().ok()).flatten() {
            Some(_) => true,
            None => false,
        }
    }

    fn progress_snapshot(&self) -> Option<VideoProgressSnapshot> {
        self.current_video_id
            .as_ref()
            .map(|video_id| VideoProgressSnapshot {
                video_id: video_id.clone(),
                position_seconds: self.position_seconds,
                has_ended: self.has_ended,
            })
    }

    fn stop_process(&mut self) {
        if self.child.is_some() {
            let _ = self.mpv_command(serde_json::json!(["quit"]));
        }

        if let Some(mut child) = self.child.take() {
            for _ in 0..20 {
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }

            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        self.cleanup_mpv_files();
    }

    fn cleanup_mpv_files(&mut self) {
        if let Some(path) = self.ipc_path.take() {
            let _ = fs::remove_file(path);
        }

        if let Some(path) = self.log_path.take() {
            let _ = fs::remove_file(path);
        }
    }

    fn clear_active_video(&mut self, has_ended: bool) {
        self.current_video_id = None;
        self.current_path = None;
        self.is_playing = false;
        self.has_ended = has_ended;
        self.position_seconds = 0;
        self.duration_seconds = None;
        self.has_video_window = false;
        self.is_fullscreen = false;
        self.last_error = None;
    }

    fn volume_or_default(&self) -> f64 {
        if self.volume > 0.0 {
            self.volume
        } else {
            1.0
        }
    }

    fn status(&self) -> VideoPlaybackStatus {
        VideoPlaybackStatus {
            video_id: self.current_video_id.clone(),
            file_path: self.current_path.clone(),
            is_playing: self.is_playing,
            has_ended: self.has_ended,
            position_seconds: self.position_seconds,
            duration_seconds: self.duration_seconds,
            volume: self.volume_or_default(),
            has_video_window: self.has_video_window,
            is_fullscreen: self.is_fullscreen,
            backend: "mpv".to_owned(),
            error: self.last_error.clone(),
        }
    }
}

fn video_window_title(video: &VideoEntry) -> String {
    let artist = video
        .artist
        .as_deref()
        .map(str::trim)
        .filter(|artist| !artist.is_empty())
        .unwrap_or("Unknown Artist");

    format!("Cassette — {artist} - {}", video.title)
}

fn save_active_video_progress(
    video_playback: &State<'_, Mutex<VideoPlaybackState>>,
    library: &State<'_, Mutex<LibraryDatabase>>,
    reset_finished: bool,
) -> Result<(), String> {
    let snapshot = {
        let mut video_playback = video_playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        video_playback.refresh();
        video_playback.progress_snapshot()
    };

    let Some(snapshot) = snapshot else {
        return Ok(());
    };

    save_video_progress_snapshot_with_reset(library, &snapshot, reset_finished)
}

fn save_video_progress_snapshot(
    library: &State<'_, Mutex<LibraryDatabase>>,
    snapshot: &VideoProgressSnapshot,
) -> Result<(), String> {
    save_video_progress_snapshot_with_reset(library, snapshot, false)
}

fn save_video_progress_snapshot_with_reset(
    library: &State<'_, Mutex<LibraryDatabase>>,
    snapshot: &VideoProgressSnapshot,
    reset_finished: bool,
) -> Result<(), String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    let position = if snapshot.has_ended && reset_finished {
        0
    } else {
        u32::try_from(snapshot.position_seconds).unwrap_or(u32::MAX)
    };
    library.update_video_progress(&snapshot.video_id, position, false)?;

    Ok(())
}

fn cleanup_playback_for_exit(app: &AppHandle) {
    if let (Some(video_playback), Some(library)) = (
        app.try_state::<Mutex<VideoPlaybackState>>(),
        app.try_state::<Mutex<LibraryDatabase>>(),
    ) {
        let _ = save_active_video_progress(&video_playback, &library, false);
    }

    if let Some(video_playback) = app.try_state::<Mutex<VideoPlaybackState>>() {
        let mut video_playback = video_playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        video_playback.stop_process();
        video_playback.clear_active_video(false);
    }

    if let Some(playback) = app.try_state::<Mutex<PlaybackState>>() {
        let mut playback = playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        playback.shutdown();
    }
}

fn mpv_ipc_path() -> PathBuf {
    std::env::temp_dir().join(format!("cassette-mpv-{}.sock", unique_timestamp_nanos()))
}

fn mpv_log_path() -> PathBuf {
    std::env::temp_dir().join(format!("cassette-mpv-{}.log", unique_timestamp_nanos()))
}

fn ensure_mpv_available(file_path: &str) -> Result<(), String> {
    match Command::new("mpv")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(video_playback_error(
            "mpv",
            file_path,
            &format!("mpv --version exited with status {status}."),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(video_playback_error(
            "mpv",
            file_path,
            "Cassette video engine requires mpv on Linux.\nInstall with: sudo dnf install mpv",
        )),
        Err(error) => Err(video_playback_error(
            "mpv",
            file_path,
            &format!("Could not run mpv: {error}"),
        )),
    }
}

fn wait_for_mpv_ipc(
    child: &mut Child,
    ipc_path: &Path,
    log_path: &Path,
    file_path: &str,
) -> Result<(), String> {
    for _ in 0..80 {
        if ipc_path.exists() {
            return Ok(());
        }

        if let Ok(Some(status)) = child.try_wait() {
            let log = read_mpv_log(log_path);
            let detail = if log.is_empty() {
                format!("mpv exited before playback became controllable with status {status}.")
            } else {
                format!("mpv exited before playback became controllable with status {status}.\nmpv log:\n{log}")
            };

            return Err(video_playback_error("mpv", file_path, &detail));
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    let _ = child.wait();

    Err(video_playback_error(
        "mpv",
        file_path,
        "mpv started, but its control socket did not become available.",
    ))
}

fn read_mpv_log(log_path: &Path) -> String {
    fs::read_to_string(log_path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
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

fn video_playback_error(backend: &str, file_path: &str, detail: &str) -> String {
    let codec_info = ffprobe_video_codec_info(Path::new(file_path));
    let mut message = format!(
        "Cassette could not play this video internally.\nBackend: {backend}\nFile: {file_path}"
    );

    if let Some(container) = codec_info.container {
        message.push_str(&format!("\nContainer: {container}"));
    }

    if let Some(video_codec) = codec_info.video_codec {
        message.push_str(&format!("\nVideo codec: {video_codec}"));
    }

    if let Some(audio_codec) = codec_info.audio_codec {
        message.push_str(&format!("\nAudio codec: {audio_codec}"));
    }

    if let Some(resolution) = codec_info.resolution {
        message.push_str(&format!("\nResolution: {resolution}"));
    }

    if let Some(error) = codec_info.error {
        message.push_str(&format!("\nffprobe: {error}"));
    }

    message.push_str("\nTry Open External as a backup.");

    if !detail.trim().is_empty() {
        message.push_str(&format!("\n{backend}: {}", detail.trim()));
    }

    message
}

fn ffprobe_video_codec_info(path: &Path) -> VideoCodecInfo {
    let output = match Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=format_name,duration",
            "-show_entries",
            "stream=codec_type,codec_name,width,height",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return VideoCodecInfo {
                container: None,
                video_codec: None,
                audio_codec: None,
                resolution: None,
                duration_seconds: None,
                error: Some("ffprobe is not installed.".to_owned()),
            };
        }
        Err(error) => {
            return VideoCodecInfo {
                container: None,
                video_codec: None,
                audio_codec: None,
                resolution: None,
                duration_seconds: None,
                error: Some(format!("Could not run ffprobe: {error}")),
            };
        }
    };

    if !output.status.success() {
        return VideoCodecInfo {
            container: None,
            video_codec: None,
            audio_codec: None,
            resolution: None,
            duration_seconds: None,
            error: Some(short_command_error(&output)),
        };
    }

    let value = match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
        Ok(value) => value,
        Err(error) => {
            return VideoCodecInfo {
                container: None,
                video_codec: None,
                audio_codec: None,
                resolution: None,
                duration_seconds: None,
                error: Some(format!("Could not parse ffprobe output: {error}")),
            };
        }
    };
    let container = value
        .get("format")
        .and_then(|format| format.get("format_name"))
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let duration_seconds = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.round() as u32);
    let streams = value
        .get("streams")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let video_stream = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(|value| value.as_str()) == Some("video"));
    let audio_stream = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(|value| value.as_str()) == Some("audio"));
    let video_codec = video_stream
        .and_then(|stream| stream.get("codec_name"))
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let audio_codec = audio_stream
        .and_then(|stream| stream.get("codec_name"))
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let resolution = video_stream.and_then(|stream| {
        let width = stream.get("width").and_then(|value| value.as_u64())?;
        let height = stream.get("height").and_then(|value| value.as_u64())?;

        Some(format!("{width}x{height}"))
    });

    VideoCodecInfo {
        container,
        video_codec,
        audio_codec,
        resolution,
        duration_seconds,
        error: None,
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

fn row_to_video(row: &rusqlite::Row<'_>) -> rusqlite::Result<VideoEntry> {
    Ok(VideoEntry {
        id: row.get(0)?,
        file_path: row.get(1)?,
        file_name: row.get(2)?,
        title: row.get(3)?,
        artist: row.get(4)?,
        video_type: row.get(5)?,
        source: row.get(6)?,
        release_or_collection: row.get(7)?,
        year: row.get(8)?,
        venue: row.get(9)?,
        city: row.get(10)?,
        country: row.get(11)?,
        description_or_notes: row.get(12)?,
        duration_seconds: row.get(13)?,
        thumbnail_path: row.get(14)?,
        last_position_seconds: row.get(15)?,
        play_count: row.get(16)?,
        last_played_at: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
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

fn normalize_video_type(value: &str) -> String {
    match value {
        "music_video" | "live_show" | "concert" | "interview_documentary" | "behind_the_scenes" | "other" => {
            value.to_owned()
        }
        _ => "other".to_owned(),
    }
}

fn normalize_video_source(value: &str) -> String {
    match value {
        "local_file" | "dvd_import" => value.to_owned(),
        _ => "local_file".to_owned(),
    }
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

fn existing_videos_by_id(connection: &Connection) -> rusqlite::Result<HashMap<String, VideoEntry>> {
    let mut statement = connection.prepare(
        "
        SELECT
            id,
            file_path,
            file_name,
            title,
            artist,
            video_type,
            source,
            release_or_collection,
            year,
            venue,
            city,
            country,
            description_or_notes,
            duration_seconds,
            thumbnail_path,
            last_position_seconds,
            play_count,
            last_played_at,
            created_at,
            updated_at
        FROM videos
        ",
    )?;
    let rows = statement.query_map([], row_to_video)?;
    let mut videos = HashMap::new();

    for row in rows {
        let video = row?;
        videos.insert(video.id.clone(), video);
    }

    Ok(videos)
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

fn scan_audio_directory_root(
    root_path: &Path,
    scanned_at: i64,
    cover_art_dir: Option<&Path>,
) -> Result<Vec<Track>, String> {
    let mut tracks = Vec::new();
    let mut album_art_paths = HashMap::<String, CachedCoverArt>::new();
    scan_directory(
        root_path,
        scanned_at,
        cover_art_dir,
        &mut album_art_paths,
        &mut tracks,
    )?;
    tracks.sort_by(|left, right| left.file_path.cmp(&right.file_path));

    Ok(tracks)
}

fn scan_video_directory_root(
    root_path: &Path,
    scanned_at: i64,
    thumbnail_dir: Option<&Path>,
) -> Result<Vec<VideoEntry>, String> {
    let mut videos = Vec::new();
    scan_video_directory(root_path, scanned_at, thumbnail_dir, &mut videos)?;
    videos.sort_by(|left, right| left.file_path.cmp(&right.file_path));

    Ok(videos)
}

fn scan_video_directory(
    directory: &Path,
    scanned_at: i64,
    thumbnail_dir: Option<&Path>,
    videos: &mut Vec<VideoEntry>,
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
            scan_video_directory(&path, scanned_at, thumbnail_dir, videos)?;
        } else if file_type.is_file() && is_supported_video_file(&path) {
            if let Some(video) = video_from_path(path, scanned_at, thumbnail_dir) {
                videos.push(video);
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

fn is_supported_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_VIDEO_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

fn video_from_path(
    path: PathBuf,
    scanned_at: i64,
    thumbnail_dir: Option<&Path>,
) -> Option<VideoEntry> {
    let path_string = path.to_string_lossy().into_owned();
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let duration_seconds = ffprobe_video_duration_seconds(&path);
    let thumbnail_path = thumbnail_dir
        .and_then(|thumbnail_dir| generate_video_thumbnail(&path, duration_seconds, thumbnail_dir));

    Some(VideoEntry {
        id: path_string.clone(),
        file_path: path_string,
        file_name,
        title: video_title_from_path(&path),
        artist: None,
        video_type: "other".to_owned(),
        source: "local_file".to_owned(),
        release_or_collection: None,
        year: None,
        venue: None,
        city: None,
        country: None,
        description_or_notes: None,
        duration_seconds,
        thumbnail_path,
        last_position_seconds: 0,
        play_count: 0,
        last_played_at: None,
        created_at: scanned_at,
        updated_at: scanned_at,
    })
}

fn ffprobe_video_duration_seconds(path: &Path) -> Option<u32> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nokey=1:noprint_wrappers=1",
        ])
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout);
    let seconds = value.trim().parse::<f64>().ok()?;

    if seconds.is_finite() && seconds > 0.0 {
        Some(seconds.round() as u32)
    } else {
        None
    }
}

fn generate_video_thumbnail(
    path: &Path,
    duration_seconds: Option<u32>,
    thumbnail_dir: &Path,
) -> Option<String> {
    fs::create_dir_all(thumbnail_dir).ok()?;
    let file_info = file_info(path);
    let source_key = format!(
        "{}\0{}\0{}",
        path.to_string_lossy(),
        file_info.file_size.unwrap_or(0),
        file_info.modified_time.unwrap_or(0),
    );
    let thumbnail_path = thumbnail_dir.join(format!("{:016x}.jpg", stable_hash(&source_key)));

    if thumbnail_path.exists() {
        return Some(thumbnail_path.to_string_lossy().into_owned());
    }

    let seek_seconds = match duration_seconds {
        Some(duration) if duration >= 300 => 30,
        Some(duration) if duration > 10 => (duration / 10).max(1),
        _ => 1,
    };
    let output = Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error", "-ss"])
        .arg(seek_seconds.to_string())
        .arg("-i")
        .arg(path)
        .args(["-frames:v", "1", "-vf", "scale=640:-1"])
        .arg(&thumbnail_path)
        .output()
        .ok()?;

    if output.status.success() && thumbnail_path.exists() {
        Some(thumbnail_path.to_string_lossy().into_owned())
    } else {
        let _ = fs::remove_file(&thumbnail_path);
        None
    }
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

struct TagWriteGuard<'a> {
    state: &'a Mutex<TagWriteState>,
    path: String,
}

impl<'a> TagWriteGuard<'a> {
    fn new(state: &'a Mutex<TagWriteState>, path: String) -> Result<Self, String> {
        let mut writes = state
            .lock()
            .map_err(|_| "Tag writer state is unavailable.".to_owned())?;

        if !writes.active_paths.insert(path.clone()) {
            return Err("This track is already being edited.".to_owned());
        }

        Ok(Self { state, path })
    }
}

impl Drop for TagWriteGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut writes) = self.state.lock() {
            writes.active_paths.remove(&self.path);
        }
    }
}

fn validate_update_track_tags_request(request: &UpdateTrackTagsRequest) -> Result<(), String> {
    if request.track_id.trim().is_empty() {
        return Err("Track is not in the library cache.".to_owned());
    }

    if matches!(request.year, Some(0)) {
        return Err("Year must be blank or a positive integer.".to_owned());
    }

    if matches!(request.track_number, Some(0)) {
        return Err("Track number must be blank or an integer greater than or equal to 1.".to_owned());
    }

    if matches!(request.disc_number, Some(0)) {
        return Err("Disc number must be blank or an integer greater than or equal to 1.".to_owned());
    }

    Ok(())
}

fn validated_cached_track_path(track: &Track, root_path: &Path) -> Result<PathBuf, String> {
    let target_path = PathBuf::from(&track.file_path);

    if target_path.is_dir() {
        return Err("Selected track is a directory.".to_owned());
    }

    if !is_supported_audio_file(&target_path) {
        return Err("Tag editing is not currently supported for this file format.".to_owned());
    }

    let canonical_root = root_path
        .canonicalize()
        .map_err(|error| format!("Could not verify library folder: {error}"))?;
    let canonical_target = target_path
        .canonicalize()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                "Selected track no longer exists.".to_owned()
            } else {
                format!("Could not verify selected track: {error}")
            }
        })?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err("Selected track is outside the active library folder.".to_owned());
    }

    let metadata = fs::metadata(&canonical_target)
        .map_err(|error| format!("Could not read selected track: {error}"))?;

    if !metadata.is_file() {
        return Err("Selected track is not a file.".to_owned());
    }

    Ok(canonical_target)
}

fn read_tagged_file(path: &Path) -> Result<lofty::file::TaggedFile, String> {
    Probe::open(path)
        .and_then(|probe| probe.read())
        .map_err(|error| format!("Could not read audio tags: {error}"))
}

fn tag_editing_supported(tagged_file: &lofty::file::TaggedFile) -> bool {
    tagged_file
        .tag_support(tagged_file.primary_tag_type())
        .is_writable()
}

fn track_tag_values_from_file(tagged_file: &lofty::file::TaggedFile) -> TrackTagValues {
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    TrackTagValues {
        title: tag.and_then(|tag| clean_text(tag.title().map(|value| value.into_owned()))),
        artist: tag.and_then(|tag| clean_text(tag.artist().map(|value| value.into_owned()))),
        album: tag.and_then(|tag| clean_text(tag.album().map(|value| value.into_owned()))),
        album_artist: tag.and_then(album_artist),
        genre: raw_genres_for_editor(tagged_file),
        year: tag.and_then(|tag| tag.date().map(|date| date.year)),
        track_number: tag.and_then(Accessor::track),
        disc_number: tag.and_then(Accessor::disk),
    }
}

fn raw_genres_for_editor(tagged_file: &lofty::file::TaggedFile) -> Option<String> {
    let genres = tagged_file
        .primary_tag()
        .into_iter()
        .chain(tagged_file.tags().iter())
        .flat_map(|tag| tag.get_strings(ItemKey::Genre))
        .flat_map(split_genres)
        .collect::<Vec<_>>();
    let genres = normalize_genres_without_unknown(genres);

    if genres.is_empty() {
        None
    } else {
        Some(genres.join("; "))
    }
}

fn normalize_genres_without_unknown(genres: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for genre in genres {
        let Some(genre) = clean_text(Some(genre)) else {
            continue;
        };
        let key = genre.to_lowercase();

        if seen.insert(key) {
            normalized.push(genre);
        }
    }

    normalized
}

fn genre_override_active_for_values(
    values: &TrackTagValues,
    track: &Track,
    assignments: &GenreAssignmentMaps,
) -> bool {
    let raw_track = Track {
        genres: values
            .genre
            .as_deref()
            .map(split_genres)
            .unwrap_or_default(),
        album: values.album.clone(),
        album_artist: values.album_artist.clone(),
        artist: values.artist.clone(),
        ..track.clone()
    };

    assignments.albums.contains_key(&album_key_for_track(&raw_track))
        || assignments.artists.contains_key(&artist_key_for_track(&raw_track))
}

fn safe_update_track_tags(path: &Path, request: &UpdateTrackTagsRequest) -> Result<(), String> {
    let original_metadata =
        fs::metadata(path).map_err(|error| format!("Could not read selected track: {error}"))?;

    if original_metadata.permissions().readonly() {
        return Err("Selected track is read-only.".to_owned());
    }

    let temp_path = unique_sidecar_path(path, "tmp")?;
    let backup_path = unique_sidecar_path(path, "backup")?;

    if let Err(error) = fs::copy(path, &temp_path) {
        cleanup_file(&temp_path);
        return Err(format!("Could not create safe editing copy: {error}"));
    }

    if let Err(error) = fs::set_permissions(&temp_path, original_metadata.permissions()) {
        cleanup_file(&temp_path);
        return Err(format!("Could not preserve file permissions: {error}"));
    }

    if let Err(error) = write_tags_to_temp_file(&temp_path, request) {
        cleanup_file(&temp_path);
        return Err(error);
    }

    if let Err(error) = sync_file(&temp_path) {
        cleanup_file(&temp_path);
        return Err(format!("Could not flush edited tags: {error}"));
    }

    if let Err(error) = verify_tag_values(&temp_path, request) {
        cleanup_file(&temp_path);
        return Err(error);
    }

    replace_original_with_verified_temp(path, &temp_path, &backup_path, request)
}

fn write_tags_to_temp_file(path: &Path, request: &UpdateTrackTagsRequest) -> Result<(), String> {
    let mut tagged_file = read_tagged_file(path)?;

    if !tag_editing_supported(&tagged_file) {
        return Err("Tag editing is not currently supported for this file format.".to_owned());
    }

    let primary_tag_type = tagged_file.primary_tag_type();
    if tagged_file.primary_tag().is_none() {
        tagged_file.insert_tag(Tag::new(primary_tag_type));
    }

    let tag = tagged_file
        .primary_tag_mut()
        .ok_or_else(|| "Could not prepare writable tag container.".to_owned())?;
    apply_tag_update_request(tag, request);
    tagged_file
        .save_to_path(path, WriteOptions::default())
        .map_err(|error| format!("Could not write tags: {error}"))
}

fn apply_tag_update_request(tag: &mut Tag, request: &UpdateTrackTagsRequest) {
    set_or_remove_text(tag, ItemKey::TrackTitle, request.title.as_deref());
    set_or_remove_text(tag, ItemKey::TrackArtist, request.artist.as_deref());
    set_or_remove_text(tag, ItemKey::AlbumTitle, request.album.as_deref());
    set_or_remove_text(tag, ItemKey::AlbumArtist, request.album_artist.as_deref());
    tag.remove_key(ItemKey::AlbumArtists);

    if let Some(genre) = normalized_request_text(request.genre.as_deref()) {
        tag.set_genre(genre);
    } else {
        tag.remove_genre();
    }

    if let Some(year) = request.year {
        tag.set_date(Timestamp {
            year,
            ..Timestamp::default()
        });
    } else {
        tag.remove_date();
    }

    if let Some(track_number) = request.track_number {
        tag.set_track(track_number);
    } else {
        tag.remove_track();
    }

    if let Some(disc_number) = request.disc_number {
        tag.set_disk(disc_number);
    } else {
        tag.remove_disk();
    }
}

fn set_or_remove_text(tag: &mut Tag, key: ItemKey, value: Option<&str>) {
    if let Some(value) = normalized_request_text(value) {
        tag.insert_text(key, value);
    } else {
        tag.remove_key(key);
    }
}

fn normalized_request_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn verify_tag_values(path: &Path, request: &UpdateTrackTagsRequest) -> Result<(), String> {
    let tagged_file = read_tagged_file(path)?;
    let actual = track_tag_values_from_file(&tagged_file);
    let expected = TrackTagValues {
        title: normalized_request_text(request.title.as_deref()),
        artist: normalized_request_text(request.artist.as_deref()),
        album: normalized_request_text(request.album.as_deref()),
        album_artist: normalized_request_text(request.album_artist.as_deref()),
        genre: normalized_request_text(request.genre.as_deref()),
        year: request.year,
        track_number: request.track_number,
        disc_number: request.disc_number,
    };

    if tag_values_match(&actual, &expected) {
        Ok(())
    } else {
        Err("Tag verification failed after writing.".to_owned())
    }
}

fn tag_values_match(actual: &TrackTagValues, expected: &TrackTagValues) -> bool {
    normalize_optional_text(actual.title.as_deref()) == normalize_optional_text(expected.title.as_deref())
        && normalize_optional_text(actual.artist.as_deref()) == normalize_optional_text(expected.artist.as_deref())
        && normalize_optional_text(actual.album.as_deref()) == normalize_optional_text(expected.album.as_deref())
        && normalize_optional_text(actual.album_artist.as_deref()) == normalize_optional_text(expected.album_artist.as_deref())
        && normalize_optional_genre(actual.genre.as_deref()) == normalize_optional_genre(expected.genre.as_deref())
        && actual.year == expected.year
        && actual.track_number == expected.track_number
        && actual.disc_number == expected.disc_number
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    normalized_request_text(value)
}

fn normalize_optional_genre(value: Option<&str>) -> Option<String> {
    let genres = value
        .map(split_genres)
        .map(normalize_genres_without_unknown)
        .unwrap_or_default();

    if genres.is_empty() {
        None
    } else {
        Some(genres.join("; "))
    }
}

fn sync_file(path: &Path) -> io::Result<()> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?
        .sync_all()
}

fn replace_original_with_verified_temp(
    original_path: &Path,
    temp_path: &Path,
    backup_path: &Path,
    request: &UpdateTrackTagsRequest,
) -> Result<(), String> {
    if let Err(error) = fs::rename(original_path, backup_path) {
        cleanup_file(temp_path);
        return Err(format!("Could not prepare safe replacement: {error}"));
    }

    if let Err(error) = fs::rename(temp_path, original_path) {
        let restore_result = fs::rename(backup_path, original_path);
        cleanup_file(temp_path);
        return match restore_result {
            Ok(()) => Err(format!("Could not replace original file; original was restored: {error}")),
            Err(restore_error) => Err(format!(
                "Could not replace original file and automatic restore failed: {error}; restore error: {restore_error}"
            )),
        };
    }

    if let Err(error) = verify_tag_values(original_path, request) {
        let _ = fs::remove_file(original_path);
        return match fs::rename(backup_path, original_path) {
            Ok(()) => Err(format!("Final tag verification failed; original was restored: {error}")),
            Err(restore_error) => Err(format!(
                "Final tag verification failed and automatic restore failed: {error}; restore error: {restore_error}"
            )),
        };
    }

    if let Err(error) = fs::remove_file(backup_path) {
        let _ = fs::remove_file(original_path);
        return match fs::rename(backup_path, original_path) {
            Ok(()) => Err(format!("Could not remove temporary backup; original was restored: {error}")),
            Err(restore_error) => Err(format!(
                "Could not remove temporary backup and automatic restore failed: {error}; restore error: {restore_error}"
            )),
        };
    }

    Ok(())
}

fn unique_sidecar_path(original_path: &Path, kind: &str) -> Result<PathBuf, String> {
    let parent = original_path
        .parent()
        .ok_or_else(|| "Selected track has no parent folder.".to_owned())?;
    let file_name = original_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Selected track filename is invalid.".to_owned())?;
    let extension = original_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("audio");
    let nonce = unique_timestamp_nanos();

    for attempt in 0..1000 {
        let candidate = parent.join(format!(
            ".{file_name}.cassette-{nonce}-{attempt}.{kind}.{extension}"
        ));

        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Could not allocate a safe temporary filename.".to_owned())
}

fn cleanup_file(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

fn rescan_single_track_after_tag_write(
    path: &Path,
    scanned_at: i64,
    cover_art_dir: Option<PathBuf>,
    previous_cover_art_path: Option<String>,
) -> Result<Track, String> {
    let Some((mut track, embedded_cover_art)) = track_from_path(path.to_path_buf(), scanned_at) else {
        return Err("Tags were written, but Cassette could not rescan the updated file.".to_owned());
    };
    let album_art_key = album_art_key_for_track(&track, path);
    track.cover_art_path = cover_art_candidate(
        cover_art_dir.as_deref(),
        &album_art_key,
        path,
        embedded_cover_art.as_ref(),
    )
    .map(|cover_art| cover_art.path)
    .or(previous_cover_art_path);

    Ok(track)
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
            source: "local",
            fetched_at: None,
        }),
        "plain" => Some(LyricsFile {
            path,
            kind: "plain",
            source: "local",
            fetched_at: None,
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
            return Some(LyricsFile {
                path,
                kind,
                source: "local",
                fetched_at: None,
            });
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

fn read_lyrics_file(lyrics_file: LyricsFile, offset_seconds: f64) -> Option<TrackLyrics> {
    let text = fs::read_to_string(&lyrics_file.path).ok()?;
    let metadata = if lyrics_file.source == "lrclib" {
        lyrics_metadata_path(&lyrics_file.path)
            .and_then(|metadata_path| fs::read_to_string(metadata_path).ok())
            .and_then(|value| serde_json::from_str::<LyricsCacheMetadata>(&value).ok())
    } else {
        None
    };
    let track_path = metadata.as_ref().map(|metadata| metadata.track_path.clone());

    Some(TrackLyrics {
        path: lyrics_file.path.to_string_lossy().into_owned(),
        kind: lyrics_file.kind.to_owned(),
        text,
        source: lyrics_file.source.to_owned(),
        fetched_at: lyrics_file.fetched_at,
        track_path,
        selected_track_name: metadata
            .as_ref()
            .and_then(|metadata| metadata.selected_track_name.clone()),
        selected_artist_name: metadata
            .as_ref()
            .and_then(|metadata| metadata.selected_artist_name.clone()),
        selected_album_name: metadata
            .as_ref()
            .and_then(|metadata| metadata.selected_album_name.clone()),
        offset_seconds,
    })
}

fn app_lyrics_file_for_track(app: &AppHandle, track: &Track) -> Option<LyricsFile> {
    let lyrics_dir = app.path().app_data_dir().ok()?.join("lyrics");
    let cache_key = format!("{:016x}", stable_hash(&track.file_path));
    let synced_path = lyrics_dir.join(format!("{cache_key}.lrc"));
    let plain_path = lyrics_dir.join(format!("{cache_key}.txt"));
    let metadata = cached_lyrics_metadata(&lyrics_dir, &cache_key);
    let fetched_at = metadata.as_ref().map(|metadata| metadata.fetched_at);

    if is_valid_lyrics_file(&synced_path) {
        return Some(LyricsFile {
            path: synced_path,
            kind: "synced",
            source: "lrclib",
            fetched_at,
        });
    }

    if is_valid_lyrics_file(&plain_path) {
        return Some(LyricsFile {
            path: plain_path,
            kind: "plain",
            source: "lrclib",
            fetched_at,
        });
    }

    None
}

fn cached_lyrics_metadata(lyrics_dir: &Path, cache_key: &str) -> Option<LyricsCacheMetadata> {
    let metadata_path = lyrics_dir.join(format!("{cache_key}.json"));
    let value = fs::read_to_string(metadata_path).ok()?;

    serde_json::from_str(&value).ok()
}

fn lyrics_metadata_path(lyrics_path: &Path) -> Option<PathBuf> {
    let stem = lyrics_path.file_stem()?.to_string_lossy();
    Some(lyrics_path.with_file_name(format!("{stem}.json")))
}

async fn search_lrclib_lyrics_results(track: &Track) -> Result<Vec<LrclibLyricsResult>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("Cassette/0.1.0 local lyrics lookup")
        .build()
        .map_err(|error| format!("Could not prepare lyrics lookup: {error}"))?;
    let query = lrclib_query(track);

    if query.len() < 2 {
        return Ok(Vec::new());
    }

    let url = reqwest::Url::parse_with_params("https://lrclib.net/api/search", &query)
        .map_err(|error| format!("Could not prepare LRCLIB request: {error}"))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "Offline/network error: could not reach LRCLIB. Check your connection.".to_owned())?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !response.status().is_success() {
        return Err("Offline/network error: LRCLIB search failed. Try again later.".to_owned());
    }

    let mut results = response
        .json::<Vec<LrclibLyrics>>()
        .await
        .map_err(|_| "LRCLIB returned an unreadable search response.".to_owned())?
        .into_iter()
        .filter_map(|lyrics| lrclib_result_for_picker(lyrics, track))
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        lrclib_picker_score(right, track).cmp(&lrclib_picker_score(left, track))
    });

    Ok(results)
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

fn text_matches(left: Option<&str>, right: &str) -> bool {
    left.map(|left| left.trim().eq_ignore_ascii_case(right.trim()))
        .unwrap_or(false)
}

fn lrclib_result_for_picker(lyrics: LrclibLyrics, track: &Track) -> Option<LrclibLyricsResult> {
    let synced_lyrics = clean_lyrics_text(lyrics.synced_lyrics);
    let plain_lyrics = clean_lyrics_text(lyrics.plain_lyrics);

    if synced_lyrics.is_none() && plain_lyrics.is_none() {
        return None;
    }

    let track_name = lyrics
        .track_name
        .as_deref()
        .and_then(|value| clean_text(Some(value.to_owned())))
        .unwrap_or_else(|| "Unknown title".to_owned());
    let artist_name = lyrics
        .artist_name
        .as_deref()
        .and_then(|value| clean_text(Some(value.to_owned())))
        .unwrap_or_else(|| "Unknown artist".to_owned());
    let title_match = match_label(Some(&track_name), &track.title);
    let artist_match = track
        .artist
        .as_ref()
        .or(track.album_artist.as_ref())
        .map(|artist| match_label(Some(&artist_name), artist))
        .unwrap_or_else(|| "No library artist".to_owned());
    let duration_difference_seconds = lyrics.duration.and_then(|duration| {
        track
            .duration_seconds
            .map(|track_duration| (duration - f64::from(track_duration)).round() as i64)
    });

    Some(LrclibLyricsResult {
        track_name,
        artist_name,
        album_name: lyrics.album_name.and_then(|value| clean_text(Some(value))),
        duration_seconds: lyrics.duration,
        has_synced_lyrics: synced_lyrics.is_some(),
        has_plain_lyrics: plain_lyrics.is_some(),
        synced_lyrics,
        plain_lyrics,
        title_match,
        artist_match,
        duration_difference_seconds,
        source: "LRCLIB".to_owned(),
    })
}

fn clean_lyrics_text(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn match_label(candidate: Option<&str>, expected: &str) -> String {
    let Some(candidate) = candidate else {
        return "No match data".to_owned();
    };
    let candidate = normalize_lrclib_match_text(candidate);
    let expected = normalize_lrclib_match_text(expected);

    if candidate.is_empty() || expected.is_empty() {
        return "No match data".to_owned();
    }

    if candidate == expected {
        return "Exact match".to_owned();
    }

    if candidate.contains(&expected) || expected.contains(&candidate) {
        return "Close match".to_owned();
    }

    let candidate_words = candidate.split_whitespace().collect::<HashSet<_>>();
    let expected_words = expected.split_whitespace().collect::<HashSet<_>>();
    let shared_words = candidate_words.intersection(&expected_words).count();

    if shared_words > 0 && shared_words * 2 >= expected_words.len().max(1) {
        "Close match".to_owned()
    } else {
        "Different".to_owned()
    }
}

fn normalize_lrclib_match_text(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| character.to_lowercase())
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn lrclib_picker_score(result: &LrclibLyricsResult, track: &Track) -> i64 {
    let mut score = 0;

    if result.has_synced_lyrics {
        score += 10_000;
    }

    score += match result.title_match.as_str() {
        "Exact match" => 1_000,
        "Close match" => 450,
        _ => 0,
    };
    score += match result.artist_match.as_str() {
        "Exact match" => 500,
        "Close match" => 225,
        _ => 0,
    };

    if let Some(album) = track.album.as_ref() {
        if text_matches(result.album_name.as_deref(), album) {
            score += 250;
        }
    }

    if let Some(difference) = result.duration_difference_seconds {
        score += 100_i64.saturating_sub(difference.abs().min(100));
    }

    score
}

fn found_lyrics_from_result(result: &LrclibLyricsResult) -> Option<FoundLyrics> {
    if let Some(text) = result.synced_lyrics.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
        return Some(FoundLyrics {
            kind: "synced",
            text: text.to_owned(),
        });
    }

    result
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
    selected_result: Option<&LrclibLyricsResult>,
    replace_cached: bool,
    offset_seconds: f64,
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
    let opposite_extension = if lyrics.kind == "synced" { "txt" } else { "lrc" };
    let opposite_path = lyrics_dir.join(format!("{cache_key}.{opposite_extension}"));
    let metadata_path = lyrics_dir.join(format!("{cache_key}.json"));

    if path.exists() && !replace_cached {
        return Err("Lyrics are already cached for this track.".to_owned());
    }

    let text = lyrics.text;
    fs::write(&path, &text)
        .map_err(|error| format!("Could not save lyrics cache: {error}"))?;
    if replace_cached && opposite_path.exists() {
        let _ = fs::remove_file(opposite_path);
    }

    let fetched_at = unix_timestamp();
    let metadata = LyricsCacheMetadata {
        track_path: track.file_path.clone(),
        lyrics_kind: lyrics.kind.to_owned(),
        source: "LRCLIB".to_owned(),
        fetched_at,
        selected_track_name: selected_result.map(|result| result.track_name.clone()),
        selected_artist_name: selected_result.map(|result| result.artist_name.clone()),
        selected_album_name: selected_result.and_then(|result| result.album_name.clone()),
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|error| format!("Could not prepare lyrics cache metadata: {error}"))?;
    fs::write(&metadata_path, metadata_json)
        .map_err(|error| format!("Could not save lyrics cache metadata: {error}"))?;

    Ok(TrackLyrics {
        path: path.to_string_lossy().into_owned(),
        kind: lyrics.kind.to_owned(),
        text,
        source: "lrclib".to_owned(),
        fetched_at: Some(fetched_at),
        track_path: Some(track.file_path.clone()),
        selected_track_name: selected_result.map(|result| result.track_name.clone()),
        selected_artist_name: selected_result.map(|result| result.artist_name.clone()),
        selected_album_name: selected_result.and_then(|result| result.album_name.clone()),
        offset_seconds,
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

fn video_title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.replace(['_', '-'], " "))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "Untitled Video".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(PlaybackState::default()))
        .manage(Mutex::new(VideoPlaybackState::default()))
        .manage(Mutex::new(TagWriteState::default()))
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
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                cleanup_playback_for_exit(window.app_handle());
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_library_cache,
            send_linux_notification,
            get_video_library,
            scan_video_folder,
            update_video_info,
            update_video_progress,
            play_video,
            pause_video,
            resume_video,
            stop_video,
            seek_video,
            set_video_volume,
            get_video_position,
            get_video_state,
            bring_video_window_to_front,
            fullscreen_video_window,
            close_video_window,
            get_video_codec_info,
            detect_dvd,
            scan_dvd_titles,
            import_dvd_title,
            scan_library,
            get_track_tag_editor_data,
            update_track_tags,
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
            search_track_lyrics_results,
            save_track_lyrics_result,
            remove_cached_track_lyrics,
            set_track_lyrics_offset,
            detect_audio_cd,
            lookup_cd_metadata,
            lookup_cd_cover,
            inspect_cover_image,
            rip_cd_to_flac,
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
