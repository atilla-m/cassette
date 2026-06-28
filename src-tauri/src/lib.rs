use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "ogg", "opus", "wav", "m4a"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Track {
    id: String,
    path: String,
    file_name: String,
    extension: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_seconds: Option<u32>,
}

#[derive(Debug, Default)]
struct TrackMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_seconds: Option<u32>,
}

#[tauri::command]
fn scan_library(root: String) -> Result<Vec<Track>, String> {
    let root_path = PathBuf::from(root);

    if !root_path.exists() {
        return Err("Selected folder does not exist.".into());
    }

    if !root_path.is_dir() {
        return Err("Selected path is not a folder.".into());
    }

    let mut tracks = Vec::new();
    scan_directory(&root_path, &mut tracks)?;
    tracks.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(tracks)
}

fn scan_directory(directory: &Path, tracks: &mut Vec<Track>) -> Result<(), String> {
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
            scan_directory(&path, tracks)?;
        } else if file_type.is_file() && is_supported_audio_file(&path) {
            if let Some(track) = track_from_path(path) {
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

fn track_from_path(path: PathBuf) -> Option<Track> {
    let metadata = read_track_metadata(&path);
    let path_string = path.to_string_lossy().into_owned();
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let title = metadata.title.unwrap_or_else(|| title_from_path(&path));

    Some(Track {
        id: path_string.clone(),
        path: path_string,
        file_name,
        extension,
        title,
        artist: metadata.artist,
        album: metadata.album,
        duration_seconds: metadata.duration_seconds,
    })
}

fn read_track_metadata(_path: &Path) -> TrackMetadata {
    TrackMetadata::default()
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_library])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
