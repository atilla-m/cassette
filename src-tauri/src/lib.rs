use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

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
    tracks.sort_by(|left, right| left.file_path.cmp(&right.file_path));

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
    })
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_library])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
