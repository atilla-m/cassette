use super::*;
use std::collections::BTreeMap;
use tauri::ipc::Channel;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FolderScanReport {
    path: String,
    scanned_at: Option<i64>,
    added: usize,
    updated: usize,
    skipped: usize,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScanProgress {
    path: String,
    checked: usize,
}

#[derive(Default, Deserialize, Serialize)]
pub(super) struct ScanSnapshot {
    pub(super) folders: Vec<FolderScanReport>,
    pub(super) unavailable_track_ids: HashSet<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScanResult {
    tracks: Vec<Track>,
    folders: Vec<FolderScanReport>,
    unavailable_track_ids: HashSet<String>,
    primary_folder: String,
    scanned_at: i64,
}

impl ScanResult {
    pub(super) fn into_tracks(self) -> Vec<Track> {
        self.tracks
    }
}

impl LibraryDatabase {
    pub(super) fn scan_snapshot(&self) -> Result<ScanSnapshot, String> {
        let mut snapshot: ScanSnapshot = self
            .meta_value("library_scan_snapshot")?
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_default();
        if snapshot.folders.is_empty() {
            if let Some(primary) = self.meta_value("last_scanned_folder")?.map(PathBuf::from) {
                snapshot.folders = std::iter::once(primary.clone())
                    .chain(self.imported_library_folders(&primary)?)
                    .map(|path| FolderScanReport {
                        path: path.to_string_lossy().into_owned(),
                        ..Default::default()
                    })
                    .collect();
            }
        }
        Ok(snapshot)
    }
}

#[tauri::command]
pub(super) async fn scan_library_folders(
    root: String,
    append: bool,
    only_folder: Option<String>,
    progress: Channel<ScanProgress>,
    app: AppHandle,
) -> Result<ScanResult, String> {
    run_scan(root, append, only_folder, Some(progress), app).await
}

pub(super) async fn run_scan(
    root: String,
    append: bool,
    only_folder: Option<String>,
    progress: Option<Channel<ScanProgress>>,
    app: AppHandle,
) -> Result<ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scan_lock = app.state::<LibraryScanLock>();
        let _scan = scan_lock
            .0
            .lock()
            .map_err(|_| "Library scanning is unavailable.")?;
        let cache = app.state::<Mutex<LibraryDatabase>>();
        let selected = PathBuf::from(root);
        let (primary, roots, added, previous, mut snapshot, same_library) = {
            let library = cache.lock().map_err(|_| "Library cache is unavailable.")?;
            let (primary, extras) = library.scan_roots(&selected, append)?;
            let (extras, added) = plan_import_folders(&selected, append, &primary, extras)?;
            let roots: Vec<_> = std::iter::once(primary.clone()).chain(extras).collect();
            let same_library =
                library.meta_value("last_scanned_folder")?.as_deref() == primary.to_str();
            let (previous, snapshot) = if same_library {
                (library.cached_tracks()?, library.scan_snapshot()?)
            } else {
                (Vec::new(), ScanSnapshot::default())
            };
            (primary, roots, added, previous, snapshot, same_library)
        };
        if let Some(folder) = only_folder.as_deref() {
            if !same_library || !roots.iter().any(|root| root == Path::new(folder)) {
                return Err("Select a registered library folder to rescan.".into());
            }
        }
        let cover_art_dir = app
            .path()
            .app_data_dir()
            .ok()
            .map(|path| path.join("cover-art"));
        let scanned_at = unix_timestamp();
        let previous_by_path: HashMap<_, _> =
            previous.iter().map(|track| (&track.id, track)).collect();
        let mut by_path = BTreeMap::new();
        let mut folder_results = Vec::new();
        let mut rescanned = Vec::new();
        let mut unavailable_roots = Vec::new();
        for folder in &roots {
            let path = folder.to_string_lossy().into_owned();
            if only_folder.as_deref().is_some_and(|only| only != path) {
                let report = snapshot
                    .folders
                    .iter()
                    .find(|report| report.path == path)
                    .cloned();
                folder_results.push(report.unwrap_or(FolderScanReport {
                    path,
                    ..Default::default()
                }));
                continue;
            }
            let mut report = FolderScanReport {
                path,
                scanned_at: Some(scanned_at),
                ..Default::default()
            };
            let mut checked = 0;
            let mut send_progress = |skipped: bool| {
                checked += 1;
                report.skipped += usize::from(skipped);
                if checked == 1 || checked % 16 == 0 {
                    if let Some(progress) = &progress {
                        let _ = progress.send(ScanProgress {
                            path: report.path.clone(),
                            checked,
                        });
                    }
                }
            };
            if let Some(progress) = &progress {
                let _ = progress.send(ScanProgress {
                    path: report.path.clone(),
                    checked: 0,
                });
            }
            let mut scanned = Vec::new();
            match scan_directory(
                folder,
                scanned_at,
                cover_art_dir.as_deref(),
                &mut HashMap::new(),
                &mut scanned,
                &mut send_progress,
            ) {
                Ok(()) => {
                    rescanned.push(folder);
                    for track in scanned {
                        match previous_by_path.get(&track.id) {
                            None => report.added += 1,
                            Some(previous)
                                if previous.file_size != track.file_size
                                    || previous.modified_time != track.modified_time
                                    || previous.title != track.title
                                    || previous.lyrics_path != track.lyrics_path
                                    || previous.cover_art_path != track.cover_art_path =>
                            {
                                report.updated += 1
                            }
                            _ => {}
                        }
                        snapshot.unavailable_track_ids.remove(&track.id);
                        by_path.insert(track.id.clone(), track);
                    }
                }
                Err(error) => {
                    if !same_library || added.as_ref() == Some(folder) {
                        return Err(error);
                    }
                    unavailable_roots.push(folder);
                    report.error = Some(error);
                }
            }
            folder_results.push(report);
        }
        for track in previous {
            if by_path.contains_key(&track.id) {
                continue;
            }
            let track_path = Path::new(&track.file_path);
            if unavailable_roots
                .iter()
                .any(|root| track_path.starts_with(root))
            {
                snapshot.unavailable_track_ids.insert(track.id.clone());
            } else if rescanned.iter().any(|root| track_path.starts_with(root)) {
                snapshot.unavailable_track_ids.remove(&track.id);
                continue;
            }
            by_path.insert(track.id.clone(), track);
        }
        snapshot.folders = folder_results;
        let mut tracks: Vec<_> = by_path.into_values().collect();
        let history_paths = cache
            .lock()
            .map_err(|_| "Library cache is unavailable.")?
            .history_paths_for_scan(&tracks)?;
        let fingerprints = portable_history::fingerprint_scan_history(history_paths);
        cache
            .lock()
            .map_err(|_| "Library cache is unavailable.")?
            .replace_library_with_import(
                &primary,
                &mut tracks,
                scanned_at,
                added.as_deref(),
                &fingerprints,
                Some(&snapshot),
            )?;
        Ok(ScanResult {
            tracks,
            folders: snapshot.folders,
            unavailable_track_ids: snapshot.unavailable_track_ids,
            primary_folder: primary.to_string_lossy().into_owned(),
            scanned_at,
        })
    })
    .await
    .map_err(|error| format!("Could not scan library: {error}"))?
}
