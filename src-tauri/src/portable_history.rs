use super::{audio_payload_fingerprint_with_hasher, unix_timestamp, LibraryDatabase, Track};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::hash::Hasher;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

const BACKUP_VERSION: u32 = 1;
const MAX_EXACT_PLAYS: i64 = 9_007_199_254_740_991;
const MAX_BACKUP_BYTES: u64 = 256 * 1024 * 1024;
const MAX_BACKUP_EVENTS: usize = 2_000_000;

#[derive(Default, Clone)]
struct Sha256Writer(Sha256);

impl Hasher for Sha256Writer {
    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
    fn finish(&self) -> u64 {
        let digest = self.0.clone().finalize();
        u64::from_le_bytes(digest[..8].try_into().unwrap())
    }
    fn write_u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }
    fn write_u32(&mut self, value: u32) {
        self.write(&value.to_le_bytes());
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_text(value: &str) -> String {
    digest_hex(&Sha256::digest(value.as_bytes()))
}

fn audio_key(path: &Path) -> Result<String, String> {
    let mut hasher = Sha256Writer::default();
    let size = audio_payload_fingerprint_with_hasher(path, &mut hasher, true)?;
    Ok(format!(
        "sha256-audio-v1:{size}:{}",
        digest_hex(&hasher.0.finalize())
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PortableRef {
    pub(super) reference_id: String,
    pub(super) audio_key: String,
    pub(super) title: String,
    pub(super) artist: Option<String>,
    pub(super) album: Option<String>,
    pub(super) album_artist: Option<String>,
    pub(super) genres: Vec<String>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    duration_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortableEvent {
    event_id: String,
    played_at_utc: i64,
    recorded_at_utc: i64,
    source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortableBaseline {
    source_id: String,
    origin_reference_id: String,
    undated_plays: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortableTrack {
    reference: PortableRef,
    all_time_total: i64,
    baselines: Vec<PortableBaseline>,
    events: Vec<PortableEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortableBackup {
    format: String,
    version: u32,
    source_id: String,
    exported_at_utc: i64,
    detailed_tracking_started_at_utc: Option<i64>,
    historical_coverage: String,
    coverage_sources: Vec<CoverageSource>,
    tracks: Vec<PortableTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CoverageSource {
    pub(super) source_id: String,
    pub(super) detailed_tracking_started_at_utc: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportResult {
    track_count: usize,
    event_count: usize,
    undated_plays: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportTrackPreview {
    reference_id: String,
    pending_track_id: String,
    title: String,
    artist: Option<String>,
    status: String,
    matched_track_id: Option<String>,
    candidates: Vec<String>,
    new_events: usize,
    duplicate_events: usize,
    undated_plays: i64,
    conflict: Option<String>,
    retained: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportPreview {
    approval_token: String,
    tracks: Vec<ImportTrackPreview>,
    matched_tracks: usize,
    unmatched_tracks: usize,
    ambiguous_tracks: usize,
    new_events: usize,
    duplicate_events: usize,
    undated_plays: i64,
    conflicts: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportResult {
    imported_events: usize,
    pending_tracks: usize,
    matched_tracks: usize,
    imported_undated_plays: i64,
}

impl LibraryDatabase {
    pub(super) fn cache_history_reference(&self, id: &str) {
        let cached = self
            .connection
            .query_row(
                "SELECT 1 FROM history_track_references WHERE track_id = ?1",
                [id],
                |_| Ok(()),
            )
            .optional();
        if matches!(cached, Ok(Some(()))) {
            return;
        }
        if let Ok(Some(track)) = self.track_by_id(id) {
            if let Ok(source) = self.source_id() {
                if let Ok(reference) = track_reference(&source, &track) {
                    if let Ok(json) = serde_json::to_string(&reference) {
                        let _ = self.connection.execute("INSERT OR IGNORE INTO history_track_references (track_id, reference_json) VALUES (?1, ?2)", params![id, json]);
                    }
                }
            }
        }
    }

    pub(super) fn removed_history_references(
        &self,
        new_tracks: &[Track],
    ) -> Result<Vec<(String, String)>, String> {
        let retained: HashSet<&str> = new_tracks.iter().map(|track| track.id.as_str()).collect();
        let source = self.source_id()?;
        let mut removed = Vec::new();
        for track in self.load_cache()?.tracks {
            if track.play_count == 0 || retained.contains(track.id.as_str()) {
                continue;
            }
            let mut reference = track_reference(&source, &track).or_else(|_| {
                self.connection
                    .query_row(
                        "SELECT reference_json FROM history_track_references WHERE track_id = ?1",
                        [&track.id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
                    .map(|json| serde_json::from_str(&json).map_err(|error| error.to_string()))
                    .unwrap_or_else(|| {
                        Ok(PortableRef {
                            reference_id: sha256_text(&format!("{source}\0{}", track.id)),
                            audio_key: format!("unavailable:{}", sha256_text(&track.id)),
                            title: track.title.clone(),
                            artist: track.artist.clone(),
                            album: track.album.clone(),
                            album_artist: track.album_artist.clone(),
                            genres: track.genres.clone(),
                            track_number: track.track_number,
                            disc_number: track.disc_number,
                            duration_seconds: track.duration_seconds,
                        })
                    })
            })?;
            if let Ok(Some(json)) = self
                .connection
                .query_row(
                    "SELECT reference_json FROM history_track_references WHERE track_id = ?1",
                    [&track.id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
            {
                if let Ok(cached) = serde_json::from_str::<PortableRef>(&json) {
                    reference.reference_id = cached.reference_id;
                }
            }
            removed.push((
                track.id,
                serde_json::to_string(&reference).map_err(|error| error.to_string())?,
            ));
        }
        Ok(removed)
    }

    pub(super) fn migrate_portable_history(&mut self) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS history_legacy_baselines (
                source_id TEXT NOT NULL,
                origin_reference_id TEXT NOT NULL,
                track_id TEXT NOT NULL,
                undated_plays INTEGER NOT NULL CHECK (undated_plays >= 0),
                PRIMARY KEY (source_id, origin_reference_id)
            );
            CREATE INDEX IF NOT EXISTS idx_history_baselines_track_id
                ON history_legacy_baselines (track_id);
            CREATE TABLE IF NOT EXISTS history_pending_tracks (
                track_id TEXT PRIMARY KEY NOT NULL,
                reference_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS history_track_references (
                track_id TEXT PRIMARY KEY NOT NULL,
                reference_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS history_associations (
                reference_id TEXT PRIMARY KEY NOT NULL,
                track_id TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS history_coverage (
                source_id TEXT PRIMARY KEY NOT NULL,
                started_at_utc INTEGER
            );",
        )?;
        let source_id: Option<String> = transaction
            .query_row(
                "SELECT value FROM library_meta WHERE key = 'portable_history_source_id'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if source_id.is_none() {
            transaction.execute(
                "INSERT INTO library_meta (key, value) VALUES ('portable_history_source_id', ?1)",
                [Uuid::new_v4().to_string()],
            )?;
        }

        // Preserve every original event identity, including beta.3's legacy IDs.
        // Baselines are provenance-labelled, not invented dated events.
        transaction.execute(
            "INSERT OR IGNORE INTO history_coverage (source_id, started_at_utc) \
             SELECT (SELECT value FROM library_meta WHERE key = 'portable_history_source_id'), \
             CAST(value AS INTEGER) FROM library_meta WHERE key = 'detailed_play_history_started_at_utc'",
            [],
        )?;
        let orphan_ids = {
            let mut statement = transaction.prepare("SELECT DISTINCT track_id FROM track_play_events WHERE track_id NOT IN (SELECT id FROM tracks) AND track_id NOT IN (SELECT track_id FROM history_pending_tracks)")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let source: String = transaction.query_row(
            "SELECT value FROM library_meta WHERE key = 'portable_history_source_id'",
            [],
            |row| row.get(0),
        )?;
        for orphan in orphan_ids {
            let reference_id = sha256_text(&format!("{source}\0{orphan}"));
            let reference = PortableRef {
                reference_id: reference_id.clone(),
                audio_key: format!("unavailable:{reference_id}"),
                title: "Unavailable legacy track".to_owned(),
                artist: None,
                album: None,
                album_artist: None,
                genres: Vec::new(),
                track_number: None,
                disc_number: None,
                duration_seconds: None,
            };
            let pending = pending_id(&source, &reference_id);
            transaction.execute(
                "INSERT INTO history_pending_tracks (track_id, reference_json) VALUES (?1, ?2)",
                params![pending, serde_json::to_string(&reference).unwrap()],
            )?;
            transaction.execute(
                "UPDATE track_play_events SET track_id = ?2 WHERE track_id = ?1",
                params![orphan, pending],
            )?;
            transaction.execute(
                "UPDATE history_legacy_baselines SET track_id = ?2 WHERE track_id = ?1",
                params![orphan, pending],
            )?;
        }
        transaction.commit()
    }

    fn source_id(&self) -> Result<String, String> {
        self.meta_value("portable_history_source_id")?
            .ok_or_else(|| "Portable history identity is missing.".to_owned())
    }

    fn export_backup(&mut self) -> Result<(PortableBackup, ExportResult), String> {
        let source_id = self.source_id()?;
        let genre_assignments = self.genre_assignments()?;
        let started = self
            .meta_value("detailed_play_history_started_at_utc")?
            .and_then(|value| value.parse().ok());
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| format!("Could not snapshot listening history: {error}"))?;
        freeze_local_baselines(&transaction, &source_id)?;

        let mut tracks = Vec::new();
        let cached =
            {
                let mut statement = transaction.prepare(
                "SELECT id, title, artist, album, album_artist, track_number, disc_number, \
                 duration_seconds, file_path, play_count, genres FROM tracks \
                 WHERE play_count > 0 ORDER BY id",
            ).map_err(|error| format!("Could not read track history: {error}"))?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<u32>>(5)?,
                            row.get::<_, Option<u32>>(6)?,
                            row.get::<_, Option<u32>>(7)?,
                            row.get::<_, String>(8)?,
                            row.get::<_, i64>(9)?,
                            row.get::<_, String>(10)?,
                        ))
                    })
                    .map_err(|error| format!("Could not read track history: {error}"))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|error| format!("Could not read track history: {error}"))?
            };
        for (
            id,
            title,
            artist,
            album,
            album_artist,
            track_number,
            disc_number,
            duration_seconds,
            file_path,
            total,
            genres_json,
        ) in cached
        {
            let key = audio_key(Path::new(&file_path)).or_else(|_| {
                let cached: Option<String> = transaction.query_row("SELECT reference_json FROM history_track_references WHERE track_id = ?1", [&id], |row| row.get(0))
                    .optional().map_err(|error| error.to_string())?;
                cached.and_then(|json| serde_json::from_str::<PortableRef>(&json).ok()).map(|reference| reference.audio_key)
                    .ok_or_else(|| "No verified portable reference is available.".to_owned())
            }).unwrap_or_else(|_| format!("unavailable:{}", sha256_text(&format!("{source_id}\0{id}"))));
            let mut reference = PortableRef {
                reference_id: sha256_text(&format!("{source_id}\0{id}")),
                audio_key: key,
                title,
                artist,
                album,
                album_artist,
                genres: serde_json::from_str(&genres_json).unwrap_or_default(),
                track_number,
                disc_number,
                duration_seconds,
            };
            let cached: Option<String> = transaction
                .query_row(
                    "SELECT reference_json FROM history_track_references WHERE track_id = ?1",
                    [&id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?;
            if let Some(cached) =
                cached.and_then(|json| serde_json::from_str::<PortableRef>(&json).ok())
            {
                reference.reference_id = cached.reference_id;
            }
            let album_artist_name = reference
                .album_artist
                .as_deref()
                .or(reference.artist.as_deref())
                .unwrap_or("Unknown Artist");
            let album_key = format!(
                "{}\0{}",
                album_artist_name.to_lowercase(),
                reference
                    .album
                    .as_deref()
                    .unwrap_or("Unknown Album")
                    .to_lowercase()
            );
            let artist_key = reference
                .artist
                .as_deref()
                .or(reference.album_artist.as_deref())
                .unwrap_or("Unknown Artist")
                .trim()
                .to_lowercase();
            if let Some(assigned) = genre_assignments
                .albums
                .get(&album_key)
                .or_else(|| genre_assignments.artists.get(&artist_key))
            {
                reference.genres = assigned.clone();
            }
            transaction.execute(
                "INSERT INTO history_track_references (track_id, reference_json) VALUES (?1, ?2) \
                 ON CONFLICT(track_id) DO UPDATE SET reference_json = excluded.reference_json",
                params![id, serde_json::to_string(&reference).map_err(|error| error.to_string())?],
            ).map_err(|error| format!("Could not cache portable track reference: {error}"))?;
            tracks.push(export_track(
                &transaction,
                &source_id,
                &id,
                reference,
                total,
            )?);
        }
        let pending =
            {
                let mut statement = transaction.prepare(
                "SELECT track_id, reference_json FROM history_pending_tracks ORDER BY track_id",
            ).map_err(|error| format!("Could not read pending history: {error}"))?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|error| format!("Could not read pending history: {error}"))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|error| format!("Could not read pending history: {error}"))?
            };
        for (id, reference_json) in pending {
            let reference: PortableRef = serde_json::from_str(&reference_json)
                .map_err(|error| format!("Stored pending history is invalid: {error}"))?;
            let dated: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM track_play_events WHERE track_id = ?1",
                    [&id],
                    |row| row.get(0),
                )
                .map_err(|error| format!("Could not count pending history: {error}"))?;
            let baseline: i64 = transaction.query_row(
                "SELECT COALESCE(SUM(undated_plays), 0) FROM history_legacy_baselines WHERE track_id = ?1",
                [&id], |row| row.get(0),
            ).map_err(|error| format!("Could not count pending legacy plays: {error}"))?;
            tracks.push(export_track(
                &transaction,
                &source_id,
                &id,
                reference,
                dated + baseline,
            )?);
        }
        let coverage_sources = {
            let mut statement = transaction
                .prepare(
                    "SELECT source_id, started_at_utc FROM history_coverage ORDER BY source_id",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok(CoverageSource {
                        source_id: row.get(0)?,
                        detailed_tracking_started_at_utc: row.get(1)?,
                    })
                })
                .map_err(|error| error.to_string())?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| error.to_string())?
        };
        transaction
            .commit()
            .map_err(|error| format!("Could not finish history snapshot: {error}"))?;

        let result = ExportResult {
            track_count: tracks.len(),
            event_count: tracks.iter().map(|track| track.events.len()).sum(),
            undated_plays: tracks
                .iter()
                .flat_map(|track| &track.baselines)
                .map(|baseline| baseline.undated_plays)
                .sum(),
        };
        Ok((PortableBackup {
            format: "cassette-listening-history".to_owned(), version: BACKUP_VERSION,
            source_id, exported_at_utc: unix_timestamp(),
            detailed_tracking_started_at_utc: started,
            historical_coverage: "Dated history may be partial before detailed tracking began; undated legacy plays contribute only to all-time totals. Current tags are not historical metadata snapshots.".to_owned(),
            coverage_sources,
            tracks,
        }, result))
    }
}

fn freeze_local_baselines(transaction: &Transaction<'_>, source_id: &str) -> Result<(), String> {
    let residuals = {
        let mut statement = transaction.prepare(
            "SELECT id, play_count - (SELECT COUNT(*) FROM track_play_events e WHERE e.track_id = t.id) \
             - (SELECT COALESCE(SUM(undated_plays), 0) FROM history_legacy_baselines b WHERE b.track_id = t.id) \
             FROM tracks t",
        ).map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    };
    for (id, residual) in residuals {
        if residual < 0 {
            return Err(
                "All-time totals disagree with recorded history; no export/import was performed."
                    .to_owned(),
            );
        }
        if residual == 0 {
            continue;
        }
        let reference_id = sha256_text(&format!("{source_id}\0{id}"));
        transaction.execute(
            "INSERT INTO history_legacy_baselines (source_id, origin_reference_id, track_id, undated_plays) \
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT(source_id, origin_reference_id) DO UPDATE \
             SET undated_plays = undated_plays + excluded.undated_plays",
            params![source_id, reference_id, id, residual],
        ).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) fn archive_removed_history(
    transaction: &Transaction<'_>,
    removed: &[(String, String)],
) -> Result<(), String> {
    let source: String = transaction
        .query_row(
            "SELECT value FROM library_meta WHERE key = 'portable_history_source_id'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    freeze_local_baselines(transaction, &source)?;
    for (old_id, json) in removed {
        let reference: PortableRef =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        let pending = pending_id(&source, &reference.reference_id);
        transaction.execute("INSERT OR REPLACE INTO history_pending_tracks (track_id, reference_json) VALUES (?1, ?2)", params![pending, json])
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE track_play_events SET track_id = ?2 WHERE track_id = ?1",
                params![old_id, pending],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE history_legacy_baselines SET track_id = ?2 WHERE track_id = ?1",
                params![old_id, pending],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) fn restore_readded_history(
    transaction: &Transaction<'_>,
    tracks: &mut [Track],
) -> Result<(), String> {
    let source: String = transaction
        .query_row(
            "SELECT value FROM library_meta WHERE key = 'portable_history_source_id'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    for track in tracks {
        let cached: Option<String> = transaction
            .query_row(
                "SELECT reference_json FROM history_track_references WHERE track_id = ?1",
                [&track.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let reference_id = cached
            .and_then(|json| serde_json::from_str::<PortableRef>(&json).ok())
            .map(|reference| reference.reference_id)
            .unwrap_or_else(|| sha256_text(&format!("{source}\0{}", track.id)));
        let pending = pending_id(&source, &reference_id);
        let json: Option<String> = transaction
            .query_row(
                "SELECT reference_json FROM history_pending_tracks WHERE track_id = ?1",
                [&pending],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if let Some(json) = json {
            let reference: PortableRef =
                serde_json::from_str(&json).map_err(|error| error.to_string())?;
            if audio_key(Path::new(&track.file_path)).is_ok_and(|key| key == reference.audio_key) {
                promote_pending(transaction, &pending, &track.id)?;
                let (count, date): (i64, Option<i64>) = transaction
                    .query_row(
                        "SELECT play_count, last_played_at FROM tracks WHERE id = ?1",
                        [&track.id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| error.to_string())?;
                track.play_count = count;
                track.last_played_at = date;
            }
        }
    }
    Ok(())
}

fn export_track(
    transaction: &Transaction<'_>,
    source_id: &str,
    track_id: &str,
    reference: PortableRef,
    all_time_total: i64,
) -> Result<PortableTrack, String> {
    let events = {
        let mut statement = transaction
            .prepare(
                "SELECT event_id, played_at_utc, recorded_at_utc, source \
             FROM track_play_events WHERE track_id = ?1 ORDER BY played_at_utc, event_id",
            )
            .map_err(|error| format!("Could not read playback events: {error}"))?;
        let rows = statement
            .query_map([track_id], |row| {
                Ok(PortableEvent {
                    event_id: row.get(0)?,
                    played_at_utc: row.get(1)?,
                    recorded_at_utc: row.get(2)?,
                    source: row.get(3)?,
                })
            })
            .map_err(|error| format!("Could not read playback events: {error}"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read playback events: {error}"))?
    };
    let mut baselines = {
        let mut statement = transaction.prepare(
            "SELECT source_id, origin_reference_id, undated_plays FROM history_legacy_baselines \
             WHERE track_id = ?1 ORDER BY source_id, origin_reference_id",
        ).map_err(|error| format!("Could not read legacy totals: {error}"))?;
        let rows = statement
            .query_map([track_id], |row| {
                Ok(PortableBaseline {
                    source_id: row.get(0)?,
                    origin_reference_id: row.get(1)?,
                    undated_plays: row.get(2)?,
                })
            })
            .map_err(|error| format!("Could not read legacy totals: {error}"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| format!("Could not read legacy totals: {error}"))?
    };
    let accounted =
        events.len() as i64 + baselines.iter().map(|item| item.undated_plays).sum::<i64>();
    if accounted > all_time_total {
        return Err(format!(
            "All-time total for {} is less than its detailed history.",
            reference.title
        ));
    }
    let local_legacy = all_time_total - accounted;
    if local_legacy > 0 && !track_id.starts_with("pending:") {
        baselines.push(PortableBaseline {
            source_id: source_id.to_owned(),
            origin_reference_id: reference.reference_id.clone(),
            undated_plays: local_legacy,
        });
    }
    Ok(PortableTrack {
        reference,
        all_time_total,
        baselines,
        events,
    })
}

fn read_backup(path: &Path) -> Result<PortableBackup, String> {
    let size = fs::metadata(path)
        .map_err(|error| format!("Could not open history backup: {error}"))?
        .len();
    if size > MAX_BACKUP_BYTES {
        return Err("History backup exceeds the 256 MiB safety limit.".to_owned());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(MAX_BACKUP_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read history backup: {error}"))?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err("History backup exceeds the 256 MiB safety limit.".to_owned());
    }
    let backup: PortableBackup = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid history backup JSON: {error}"))?;
    validate_backup(&backup)?;
    Ok(backup)
}

fn validate_backup(backup: &PortableBackup) -> Result<(), String> {
    if backup.format != "cassette-listening-history" || backup.version != BACKUP_VERSION {
        return Err("Unsupported listening-history backup format or version.".to_owned());
    }
    Uuid::parse_str(&backup.source_id).map_err(|_| "Invalid backup source identity.".to_owned())?;
    let mut sources = HashSet::new();
    if backup.coverage_sources.iter().any(|source| {
        Uuid::parse_str(&source.source_id).is_err()
            || source
                .detailed_tracking_started_at_utc
                .is_some_and(|timestamp| timestamp < 0)
            || !sources.insert(&source.source_id)
    }) {
        return Err("Backup contains invalid coverage provenance.".to_owned());
    }
    if !sources.contains(&backup.source_id)
        || backup.exported_at_utc < 0
        || backup
            .coverage_sources
            .iter()
            .find(|source| source.source_id == backup.source_id)
            .is_none_or(|source| {
                source.detailed_tracking_started_at_utc != backup.detailed_tracking_started_at_utc
            })
    {
        return Err("Backup source coverage is inconsistent.".to_owned());
    }
    let mut references = HashSet::new();
    let mut event_ids = HashSet::new();
    let mut global_baseline_keys = HashSet::new();
    let mut event_count = 0_usize;
    let mut all_time_total = 0_i64;
    for track in &backup.tracks {
        let reference = &track.reference;
        if reference.reference_id.len() != 64
            || !reference
                .reference_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || !valid_audio_key(&reference.audio_key)
            || reference.title.trim().is_empty()
            || !references.insert(&reference.reference_id)
        {
            return Err("Backup contains an invalid or repeated track reference.".to_owned());
        }
        let mut baseline_keys = HashSet::new();
        let baseline_total = track.baselines.iter().try_fold(0_i64, |sum, baseline| {
            if Uuid::parse_str(&baseline.source_id).is_err()
                || !sources.contains(&baseline.source_id)
                || baseline.origin_reference_id.len() != 64
                || !baseline
                    .origin_reference_id
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || baseline.undated_plays < 0
                || !baseline_keys.insert((
                    baseline.source_id.as_str(),
                    baseline.origin_reference_id.as_str(),
                ))
                || !global_baseline_keys.insert((
                    baseline.source_id.as_str(),
                    baseline.origin_reference_id.as_str(),
                ))
            {
                return Err("Backup contains an invalid legacy baseline.".to_owned());
            }
            sum.checked_add(baseline.undated_plays)
                .ok_or_else(|| "Backup legacy total overflowed.".to_owned())
        })?;
        if baseline_total > 0
            && track
                .baselines
                .iter()
                .filter(|item| item.undated_plays > 0)
                .count()
                > 1
        {
            return Err("Backup has overlapping undated legacy baselines.".to_owned());
        }
        event_count = event_count
            .checked_add(track.events.len())
            .ok_or_else(|| "Backup event count overflowed.".to_owned())?;
        if event_count > MAX_BACKUP_EVENTS {
            return Err("Backup contains too many events.".to_owned());
        }
        for event in &track.events {
            if !(8..=8192).contains(&event.event_id.len())
                || !event
                    .event_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
                || !event_ids.insert(&event.event_id)
                || event.played_at_utc < 0
                || event.recorded_at_utc < 0
                || !matches!(
                    event.source.as_str(),
                    "qualified_play" | "legacy_last_played"
                )
            {
                return Err("Backup contains an invalid or repeated playback event.".to_owned());
            }
        }
        if baseline_total.checked_add(track.events.len() as i64) != Some(track.all_time_total) {
            return Err(
                "Backup all-time total disagrees with events and undated plays.".to_owned(),
            );
        }
        all_time_total = all_time_total
            .checked_add(track.all_time_total)
            .ok_or_else(|| "Backup all-time total overflowed.".to_owned())?;
        if all_time_total > MAX_EXACT_PLAYS {
            return Err("Backup totals exceed the exact JSON integer range.".to_owned());
        }
    }
    Ok(())
}

fn track_reference(source_id: &str, track: &Track) -> Result<PortableRef, String> {
    Ok(PortableRef {
        reference_id: sha256_text(&format!("{source_id}\0{}", track.id)),
        audio_key: audio_key(Path::new(&track.file_path))?,
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        album_artist: track.album_artist.clone(),
        genres: track.genres.clone(),
        track_number: track.track_number,
        disc_number: track.disc_number,
        duration_seconds: track.duration_seconds,
    })
}

fn reference_metadata_matches(source: &PortableRef, candidate: &PortableRef) -> bool {
    source.title.eq_ignore_ascii_case(&candidate.title)
        && source.artist.as_ref().map(|s| s.to_lowercase())
            == candidate.artist.as_ref().map(|s| s.to_lowercase())
        && source.album.as_ref().map(|s| s.to_lowercase())
            == candidate.album.as_ref().map(|s| s.to_lowercase())
        && source.album_artist.as_ref().map(|s| s.to_lowercase())
            == candidate.album_artist.as_ref().map(|s| s.to_lowercase())
        && source.track_number == candidate.track_number
        && source.disc_number == candidate.disc_number
}

fn candidate_tracks(
    database: &LibraryDatabase,
) -> Result<HashMap<String, Vec<(String, PortableRef)>>, String> {
    let source_id = database.source_id()?;
    let mut result: HashMap<String, Vec<(String, PortableRef)>> = HashMap::new();
    for track in database.load_cache()?.tracks {
        let Ok(reference) = track_reference(&source_id, &track) else {
            continue;
        };
        result
            .entry(reference.audio_key.clone())
            .or_default()
            .push((track.id, reference));
    }
    Ok(result)
}

fn pending_id(_source_id: &str, reference_id: &str) -> String {
    format!("pending:{reference_id}")
}

fn valid_audio_key(key: &str) -> bool {
    if let Some(id) = key.strip_prefix("unavailable:") {
        return id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    let parts: Vec<_> = key.split(':').collect();
    parts.len() == 3
        && parts[0] == "sha256-audio-v1"
        && parts[1].parse::<u64>().is_ok()
        && parts[2].len() == 64
        && parts[2].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn resolve_target(
    reference: &PortableRef,
    candidates: &HashMap<String, Vec<(String, PortableRef)>>,
) -> (Option<String>, Vec<String>, String) {
    let Some(possible) = candidates.get(&reference.audio_key) else {
        return (None, Vec::new(), "unmatched".to_owned());
    };
    if possible.len() == 1 {
        return (
            Some(possible[0].0.clone()),
            vec![possible[0].0.clone()],
            "matched".to_owned(),
        );
    }
    let exact = possible
        .iter()
        .filter(|(_, candidate)| reference_metadata_matches(reference, candidate))
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return (
            Some(exact[0].0.clone()),
            vec![exact[0].0.clone()],
            "matched".to_owned(),
        );
    }
    (
        None,
        possible.iter().map(|(id, _)| id.clone()).collect(),
        "ambiguous".to_owned(),
    )
}

fn preview_backup(
    database: &LibraryDatabase,
    backup: &PortableBackup,
) -> Result<ImportPreview, String> {
    let candidates = candidate_tracks(database)?;
    let local_source = database.source_id()?;
    let mut tracks = Vec::new();
    let mut conflicts = Vec::new();
    for coverage in &backup.coverage_sources {
        let existing: Option<Option<i64>> = database
            .connection
            .query_row(
                "SELECT started_at_utc FROM history_coverage WHERE source_id = ?1",
                [&coverage.source_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if existing.is_some_and(|date| date != coverage.detailed_tracking_started_at_utc) {
            conflicts.push("Coverage provenance disagrees with an earlier import.".to_owned());
        }
    }
    let mut targets_seen = HashSet::new();
    for item in &backup.tracks {
        let (mut target, candidate_ids, mut status) = resolve_target(&item.reference, &candidates);
        let association: Option<String> = database
            .connection
            .query_row(
                "SELECT track_id FROM history_associations WHERE reference_id = ?1",
                [&item.reference.reference_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if let Some(associated) = association {
            if candidates
                .get(&item.reference.audio_key)
                .is_some_and(|items| items.iter().any(|(id, _)| id == &associated))
            {
                target = Some(associated);
                status = "matched".to_owned();
            }
        }
        let pending = pending_id(&backup.source_id, &item.reference.reference_id);
        let stored_track = target.as_deref().unwrap_or(&pending);
        let mut new_events = 0;
        let mut duplicate_events = 0;
        let mut conflict = None;
        let pending_reference: Option<String> = database
            .connection
            .query_row(
                "SELECT reference_json FROM history_pending_tracks WHERE track_id = ?1",
                [&pending],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if let Some(json) = pending_reference {
            let previous: PortableRef =
                serde_json::from_str(&json).map_err(|error| error.to_string())?;
            if previous.audio_key.starts_with("sha256-audio-v1:")
                && item.reference.audio_key.starts_with("sha256-audio-v1:")
                && previous.audio_key != item.reference.audio_key
            {
                conflict = Some(
                    "A retained track reference now identifies different audio content.".to_owned(),
                );
            }
        }
        if target
            .as_ref()
            .is_some_and(|id| !targets_seen.insert(id.clone()))
        {
            conflict = Some("Several source tracks match one destination track; refusing to merge their totals.".to_owned());
        }
        for event in &item.events {
            let existing: Option<(String, i64, i64, String)> = database.connection.query_row(
                "SELECT track_id, played_at_utc, recorded_at_utc, source FROM track_play_events WHERE event_id = ?1",
                [&event.event_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ).optional().map_err(|error| format!("Could not check existing events: {error}"))?;
            match existing {
                None => new_events += 1,
                Some((old_track, old_time, old_recorded, old_source))
                    if old_time == event.played_at_utc
                        && old_recorded == event.recorded_at_utc
                        && old_source == event.source
                        && (old_track == stored_track || old_track == pending) =>
                {
                    duplicate_events += 1
                }
                Some(_) => {
                    conflict = Some("An event ID already belongs to different history.".to_owned())
                }
            }
        }
        let incoming_baseline: i64 = item
            .baselines
            .iter()
            .map(|baseline| baseline.undated_plays)
            .sum();
        for baseline in &item.baselines {
            let owner: Option<String> = database.connection.query_row(
                "SELECT track_id FROM history_legacy_baselines WHERE source_id = ?1 AND origin_reference_id = ?2",
                params![baseline.source_id, baseline.origin_reference_id], |row| row.get(0),
            ).optional().map_err(|error| error.to_string())?;
            if owner.is_some_and(|id| id != stored_track && id != pending) {
                conflict = Some("Legacy baseline already belongs to a different track.".to_owned());
            }
        }
        if let Some(target_id) = &target {
            let (total, dated): (i64, i64) = database.connection.query_row(
                "SELECT t.play_count, (SELECT COUNT(*) FROM track_play_events e WHERE e.track_id = t.id) \
                 FROM tracks t WHERE t.id = ?1",
                [target_id], |row| Ok((row.get(0)?, row.get(1)?)),
            ).map_err(|error| format!("Could not inspect existing totals: {error}"))?;
            let stored_baselines: i64 = database.connection.query_row(
                "SELECT COALESCE(SUM(undated_plays), 0) FROM history_legacy_baselines WHERE track_id = ?1",
                [target_id], |row| row.get(0),
            ).map_err(|error| format!("Could not inspect existing baselines: {error}"))?;
            let local_undated = total - dated - stored_baselines;
            if local_undated < 0 {
                conflict = Some(
                    "Existing all-time total is smaller than its recorded history.".to_owned(),
                );
            } else if incoming_baseline > 0
                && local_undated > 0
                && !item.baselines.iter().any(|baseline| {
                    baseline.source_id == local_source
                        && baseline.origin_reference_id
                            == sha256_text(&format!("{local_source}\0{target_id}"))
                })
            {
                conflict = Some(
                    "Both devices have undated legacy totals; their overlap cannot be determined."
                        .to_owned(),
                );
            }
            let retained_events: i64 = database
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM track_play_events WHERE track_id = ?1",
                    [&pending],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if incoming_baseline == 0 && (new_events > 0 || retained_events > 0) && total > dated {
                // A dated-only backup has no baseline lineage with which to
                // prove that its events aren't already included in these totals.
                conflict = Some("Incoming dated events may overlap existing undated plays; no shared legacy provenance establishes a safe reconciliation.".to_owned());
            }
            if incoming_baseline > 0 {
                let known: Vec<(String, String)> = {
                    let mut statement = database
                        .connection
                        .prepare(
                            "SELECT source_id, origin_reference_id FROM history_legacy_baselines \
                         WHERE track_id = ?1 AND undated_plays > 0",
                        )
                        .map_err(|error| format!("Could not check legacy provenance: {error}"))?;
                    let keys = statement
                        .query_map([target_id], |row| Ok((row.get(0)?, row.get(1)?)))
                        .map_err(|error| format!("Could not check legacy provenance: {error}"))?
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(|error| format!("Could not check legacy provenance: {error}"))?;
                    keys
                };
                if known.iter().any(|key| {
                    !item.baselines.iter().any(|incoming| {
                        &incoming.source_id == &key.0 && &incoming.origin_reference_id == &key.1
                    })
                }) {
                    conflict = Some("This track has a different undated legacy baseline; overlap is unresolved.".to_owned());
                }
            }
        }
        if let Some(message) = &conflict {
            conflicts.push(format!("{}: {message}", item.reference.title));
        }
        tracks.push(ImportTrackPreview {
            reference_id: item.reference.reference_id.clone(),
            pending_track_id: pending.clone(),
            title: item.reference.title.clone(),
            artist: item.reference.artist.clone(),
            status,
            matched_track_id: target,
            candidates: candidate_ids,
            retained: database
                .connection
                .query_row(
                    "SELECT 1 FROM history_pending_tracks WHERE track_id = ?1",
                    [&pending],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .is_some(),
            new_events,
            duplicate_events,
            undated_plays: incoming_baseline,
            conflict,
        });
    }
    let mut preview = ImportPreview {
        approval_token: String::new(),
        matched_tracks: tracks
            .iter()
            .filter(|item| item.status == "matched")
            .count(),
        unmatched_tracks: tracks
            .iter()
            .filter(|item| item.status == "unmatched")
            .count(),
        ambiguous_tracks: tracks
            .iter()
            .filter(|item| item.status == "ambiguous")
            .count(),
        new_events: tracks.iter().map(|item| item.new_events).sum(),
        duplicate_events: tracks.iter().map(|item| item.duplicate_events).sum(),
        undated_plays: tracks.iter().map(|item| item.undated_plays).sum(),
        conflicts,
        tracks,
    };
    // Bind approval to the exact backup and resolution. A changed file/library
    // requires another preview, not a silent application of a different plan.
    preview.approval_token = sha256_text(&format!(
        "{}\n{}",
        serde_json::to_string(backup).map_err(|error| error.to_string())?,
        serde_json::to_string(&preview).map_err(|error| error.to_string())?
    ));
    Ok(preview)
}

#[cfg(test)]
fn import_backup(
    database: &mut LibraryDatabase,
    backup: &PortableBackup,
) -> Result<ImportResult, String> {
    validate_backup(backup)?;
    let preview = preview_backup(database, backup)?;
    apply_backup(database, backup, preview)
}

fn apply_backup(
    database: &mut LibraryDatabase,
    backup: &PortableBackup,
    preview: ImportPreview,
) -> Result<ImportResult, String> {
    if !preview.conflicts.is_empty() {
        return Err(format!(
            "Import has unresolved conflicts: {}",
            preview.conflicts.join("; ")
        ));
    }
    let transaction = database
        .connection
        .transaction()
        .map_err(|error| format!("Could not start history import: {error}"))?;
    let source: String = transaction
        .query_row(
            "SELECT value FROM library_meta WHERE key = 'portable_history_source_id'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    freeze_local_baselines(&transaction, &source)?;
    for coverage in &backup.coverage_sources {
        let existing: Option<Option<i64>> = transaction
            .query_row(
                "SELECT started_at_utc FROM history_coverage WHERE source_id = ?1",
                [&coverage.source_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if existing.is_some_and(|date| date != coverage.detailed_tracking_started_at_utc) {
            return Err("Coverage provenance disagrees with an earlier import.".to_owned());
        }
        transaction.execute("INSERT OR IGNORE INTO history_coverage (source_id, started_at_utc) VALUES (?1, ?2)", params![coverage.source_id, coverage.detailed_tracking_started_at_utc])
            .map_err(|error| error.to_string())?;
    }
    let mut imported_events = 0;
    let mut imported_undated_plays = 0;
    for (item, checked) in backup.tracks.iter().zip(&preview.tracks) {
        let pending = pending_id(&backup.source_id, &item.reference.reference_id);
        let target = checked.matched_track_id.as_deref().unwrap_or(&pending);
        if checked.matched_track_id.is_none() {
            transaction.execute(
                "INSERT INTO history_pending_tracks (track_id, reference_json) VALUES (?1, ?2) \
                 ON CONFLICT(track_id) DO UPDATE SET reference_json = CASE \
                 WHEN json_extract(history_pending_tracks.reference_json, '$.audioKey') LIKE 'sha256-audio-v1:%' \
                 AND json_extract(excluded.reference_json, '$.audioKey') LIKE 'unavailable:%' \
                 THEN history_pending_tracks.reference_json ELSE excluded.reference_json END",
                params![target, serde_json::to_string(&item.reference).map_err(|error| error.to_string())?],
            ).map_err(|error| format!("Could not retain unmatched history: {error}"))?;
        } else {
            promote_pending(&transaction, &pending, target)?;
            transaction.execute("INSERT INTO history_associations (reference_id, track_id) VALUES (?1, ?2) ON CONFLICT(reference_id) DO UPDATE SET track_id = excluded.track_id", params![item.reference.reference_id, target])
                .map_err(|error| error.to_string())?;
            transaction.execute("INSERT INTO history_track_references (track_id, reference_json) VALUES (?1, ?2) ON CONFLICT(track_id) DO UPDATE SET reference_json = excluded.reference_json", params![target, serde_json::to_string(&item.reference).map_err(|error| error.to_string())?])
                .map_err(|error| error.to_string())?;
        }
        for event in &item.events {
            let inserted = transaction
                .execute(
                    "INSERT OR IGNORE INTO track_play_events \
                 (event_id, track_id, played_at_utc, recorded_at_utc, source) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        event.event_id,
                        target,
                        event.played_at_utc,
                        event.recorded_at_utc,
                        event.source
                    ],
                )
                .map_err(|error| format!("Could not import playback event: {error}"))?;
            if inserted != 0 {
                imported_events += 1;
                if checked.matched_track_id.is_some() {
                    transaction
                        .execute(
                            "UPDATE tracks SET play_count = play_count + 1, \
                         last_played_at = MAX(COALESCE(last_played_at, 0), ?2) WHERE id = ?1",
                            params![target, event.played_at_utc],
                        )
                        .map_err(|error| {
                            format!("Could not update imported play count: {error}")
                        })?;
                }
            }
        }
        for baseline in &item.baselines {
            let old: Option<(String, i64)> = transaction
                .query_row(
                    "SELECT track_id, undated_plays FROM history_legacy_baselines \
                 WHERE source_id = ?1 AND origin_reference_id = ?2",
                    params![baseline.source_id, baseline.origin_reference_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|error| format!("Could not check imported legacy total: {error}"))?;
            let old_count = old.as_ref().map(|(_, count)| *count).unwrap_or(0);
            if old
                .as_ref()
                .is_some_and(|(old_track, _)| old_track != target)
            {
                return Err("Legacy baseline already belongs to a different track.".to_owned());
            }
            if baseline.undated_plays > old_count {
                let delta = baseline.undated_plays - old_count;
                transaction.execute(
                    "INSERT INTO history_legacy_baselines \
                     (source_id, origin_reference_id, track_id, undated_plays) VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT(source_id, origin_reference_id) DO UPDATE SET undated_plays = excluded.undated_plays",
                    params![baseline.source_id, baseline.origin_reference_id, target, baseline.undated_plays],
                ).map_err(|error| format!("Could not save imported legacy total: {error}"))?;
                imported_undated_plays += delta;
                if checked.matched_track_id.is_some() {
                    transaction
                        .execute(
                            "UPDATE tracks SET play_count = play_count + ?2 WHERE id = ?1",
                            params![target, delta],
                        )
                        .map_err(|error| {
                            format!("Could not update imported legacy count: {error}")
                        })?;
                }
            }
        }
    }
    // The source's start date is provenance, not proof of full earlier coverage.
    transaction
        .commit()
        .map_err(|error| format!("Could not commit history import: {error}"))?;
    Ok(ImportResult {
        imported_events,
        imported_undated_plays,
        matched_tracks: preview.matched_tracks,
        pending_tracks: preview.unmatched_tracks + preview.ambiguous_tracks,
    })
}

fn promote_pending(
    transaction: &Transaction<'_>,
    pending: &str,
    target: &str,
) -> Result<(), String> {
    let pending_events: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM track_play_events WHERE track_id = ?1",
            [pending],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not inspect pending events: {error}"))?;
    let pending_baselines: i64 = transaction.query_row(
        "SELECT COALESCE(SUM(undated_plays), 0) FROM history_legacy_baselines WHERE track_id = ?1",
        [pending], |row| row.get(0),
    ).map_err(|error| format!("Could not inspect pending totals: {error}"))?;
    if pending_events + pending_baselines == 0 {
        return Ok(());
    }
    if pending_events + pending_baselines > 0 {
        let target_undated: i64 = transaction.query_row(
            "SELECT play_count - (SELECT COUNT(*) FROM track_play_events WHERE track_id = ?1) FROM tracks WHERE id = ?1",
            [target], |row| row.get(0),
        ).map_err(|error| error.to_string())?;
        if target_undated > 0 {
            return Err("Selected track already has undated legacy plays; their overlap with pending history is unresolved.".to_owned());
        }
    }
    let latest: Option<i64> = transaction
        .query_row(
            "SELECT MAX(played_at_utc) FROM track_play_events WHERE track_id = ?1",
            [pending],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not inspect pending dates: {error}"))?;
    transaction
        .execute(
            "UPDATE track_play_events SET track_id = ?2 WHERE track_id = ?1",
            params![pending, target],
        )
        .map_err(|error| format!("Could not associate pending events: {error}"))?;
    transaction
        .execute(
            "UPDATE history_legacy_baselines SET track_id = ?2 WHERE track_id = ?1",
            params![pending, target],
        )
        .map_err(|error| format!("Could not associate pending totals: {error}"))?;
    transaction
        .execute(
            "UPDATE tracks SET play_count = play_count + ?2, \
         last_played_at = CASE WHEN ?3 IS NULL THEN last_played_at \
         ELSE MAX(COALESCE(last_played_at, 0), ?3) END WHERE id = ?1",
            params![target, pending_events + pending_baselines, latest],
        )
        .map_err(|error| format!("Could not update associated play count: {error}"))?;
    transaction
        .execute(
            "DELETE FROM history_pending_tracks WHERE track_id = ?1",
            [pending],
        )
        .map_err(|error| format!("Could not clear associated pending history: {error}"))?;
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn export_listening_history(
    path: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<ExportResult, String> {
    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    let (backup, result) = library.export_backup()?;
    validate_backup(&backup)?;
    let bytes = serde_json::to_vec_pretty(&backup)
        .map_err(|error| format!("Could not encode history backup: {error}"))?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err("History backup exceeds the 256 MiB safety limit.".to_owned());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            format!(
                "Could not create history backup (existing files are never overwritten): {error}"
            )
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Could not finish writing history backup: {error}"))?;
    Ok(result)
}

#[tauri::command(async)]
pub(crate) fn preview_listening_history_import(
    path: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<ImportPreview, String> {
    let backup = read_backup(Path::new(&path))?;
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    preview_backup(&library, &backup)
}

#[tauri::command(async)]
pub(crate) fn import_listening_history(
    path: String,
    approval_token: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<ImportResult, String> {
    let backup = read_backup(Path::new(&path))?;
    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    let preview = preview_backup(&library, &backup)?;
    if preview.approval_token != approval_token {
        return Err(
            "The backup or import plan changed since preview. Preview it again before importing."
                .to_owned(),
        );
    }
    apply_backup(&mut library, &backup, preview)
}

#[tauri::command(async)]
pub(crate) fn associate_listening_history_track(
    pending_track_id: String,
    target_track_id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<(), String> {
    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    let reference_json: String = library
        .connection
        .query_row(
            "SELECT reference_json FROM history_pending_tracks WHERE track_id = ?1",
            [&pending_track_id],
            |row| row.get(0),
        )
        .map_err(|_| "Pending history reference was not found.".to_owned())?;
    let reference: PortableRef = serde_json::from_str(&reference_json)
        .map_err(|_| "Pending history reference is invalid.".to_owned())?;
    let track = library
        .track_by_id(&target_track_id)?
        .ok_or_else(|| "Selected library track was not found.".to_owned())?;
    if audio_key(Path::new(&track.file_path))? != reference.audio_key {
        return Err(
            "Selected track does not match the backup's verified audio content.".to_owned(),
        );
    }
    let transaction = library
        .connection
        .transaction()
        .map_err(|error| format!("Could not start history association: {error}"))?;
    promote_pending(&transaction, &pending_track_id, &target_track_id)?;
    transaction.execute("INSERT INTO history_associations (reference_id, track_id) VALUES (?1, ?2) ON CONFLICT(reference_id) DO UPDATE SET track_id = excluded.track_id", params![reference.reference_id, target_track_id])
        .map_err(|error| error.to_string())?;
    transaction.execute("INSERT INTO history_track_references (track_id, reference_json) VALUES (?1, ?2) ON CONFLICT(track_id) DO UPDATE SET reference_json = excluded.reference_json", params![target_track_id, reference_json])
        .map_err(|error| error.to_string())?;
    transaction
        .commit()
        .map_err(|error| format!("Could not save history association: {error}"))
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::{track_from_path, unique_timestamp_nanos};
    use super::*;

    pub(super) struct Fixture {
        directory: std::path::PathBuf,
    }
    impl Fixture {
        pub(super) fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "cassette-detailed-stats-{}-{}",
                std::process::id(),
                unique_timestamp_nanos()
            ));
            fs::create_dir(&directory).unwrap();
            Self { directory }
        }
        pub(super) fn track(&self, filename: &str, sample: i16) -> Track {
            let path = self.directory.join(filename);
            let mut bytes = Vec::new();
            bytes.extend_from_slice(b"RIFF");
            bytes.extend_from_slice(&196_u32.to_le_bytes());
            bytes.extend_from_slice(b"WAVEfmt ");
            bytes.extend_from_slice(&16_u32.to_le_bytes());
            bytes.extend_from_slice(&1_u16.to_le_bytes());
            bytes.extend_from_slice(&1_u16.to_le_bytes());
            bytes.extend_from_slice(&8000_u32.to_le_bytes());
            bytes.extend_from_slice(&16000_u32.to_le_bytes());
            bytes.extend_from_slice(&2_u16.to_le_bytes());
            bytes.extend_from_slice(&16_u16.to_le_bytes());
            bytes.extend_from_slice(b"data");
            bytes.extend_from_slice(&160_u32.to_le_bytes());
            for _ in 0..80 {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
            fs::write(&path, bytes).unwrap();
            let mut track = track_from_path(path, 1_700_000_000).unwrap().0;
            track.title = "Synthetic transfer track".to_owned();
            track.artist = Some("Artist α".to_owned());
            track.album = Some("Album A".to_owned());
            track.album_artist = Some("Album artist".to_owned());
            track.genres = vec!["Ambient".to_owned(), "Electronic".to_owned()];
            track
        }
        pub(super) fn database(&self) -> LibraryDatabase {
            LibraryDatabase::open(
                self.directory
                    .join(format!("library-{}.sqlite3", unique_timestamp_nanos())),
            )
            .unwrap()
        }
        pub(super) fn seed(&self, database: &mut LibraryDatabase, tracks: &[Track]) {
            database
                .replace_library(&self.directory, &mut tracks.to_vec(), 1_700_000_000)
                .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                fs::remove_dir_all(&self.directory).unwrap();
            }
        }
    }

    pub(super) fn event(database: &LibraryDatabase, track: &Track, id: &str, time: i64) {
        database
            .connection
            .execute(
                "INSERT INTO track_play_events VALUES (?1, ?2, ?3, ?3, 'qualified_play')",
                params![id, track.id, time],
            )
            .unwrap();
        database
            .connection
            .execute(
                "UPDATE tracks SET play_count = play_count + 1, last_played_at = ?2 WHERE id = ?1",
                params![track.id, time],
            )
            .unwrap();
    }
    fn total(database: &LibraryDatabase, id: &str) -> i64 {
        database.track_by_id(id).unwrap().unwrap().play_count
    }

    #[test]
    fn portable_round_trip_changed_paths_duplicate_and_older_imports_keep_new_plays() {
        let source_fixture = Fixture::new();
        let destination_fixture = Fixture::new();
        let source_track = source_fixture.track("source.wav", 42);
        let mut source = source_fixture.database();
        source_fixture.seed(&mut source, &[source_track.clone()]);
        source
            .connection
            .execute(
                "UPDATE tracks SET play_count = 7 WHERE id = ?1",
                [&source_track.id],
            )
            .unwrap();
        event(&source, &source_track, "original-event-one", 1_700_000_005);
        event(&source, &source_track, "original-event-two", 1_700_086_400);
        let (older, _) = source.export_backup().unwrap();
        validate_backup(&older).unwrap();
        assert_eq!(older.tracks[0].all_time_total, 9);
        assert_eq!(older.tracks[0].baselines[0].undated_plays, 7);
        assert_eq!(older.tracks[0].events[0].event_id, "original-event-one");
        let json = serde_json::to_string(&older).unwrap();
        assert!(!json.contains(source_fixture.directory.to_str().unwrap()));
        assert!(!json.contains("filePath"));

        let destination_track = destination_fixture.track("moved-folder.wav", 42);
        let mut destination = destination_fixture.database();
        destination_fixture.seed(&mut destination, &[destination_track.clone()]);
        destination.toggle_favorite(&destination_track.id).unwrap();
        let playlist = destination.create_playlist("Existing playlist").unwrap();
        destination
            .add_track_to_playlist(&playlist.id, &destination_track.id)
            .unwrap();
        let preview = preview_backup(&destination, &older).unwrap();
        assert_eq!(preview.matched_tracks, 1);
        assert_eq!(preview.new_events, 2);
        assert!(preview.conflicts.is_empty());
        import_backup(&mut destination, &older).unwrap();
        assert_eq!(total(&destination, &destination_track.id), 9);
        assert_eq!(
            import_backup(&mut destination, &older)
                .unwrap()
                .imported_events,
            0
        );
        assert_eq!(total(&destination, &destination_track.id), 9);
        destination
            .record_play(&destination_track.id, "destination-new-play")
            .unwrap();
        destination
            .record_play(&destination_track.id, "destination-new-play")
            .unwrap();
        event(&source, &source_track, "source-newer-event", 1_700_172_800);
        let (newer, _) = source.export_backup().unwrap();
        import_backup(&mut destination, &newer).unwrap();
        assert_eq!(total(&destination, &destination_track.id), 11);
        import_backup(&mut destination, &older).unwrap();
        assert_eq!(total(&destination, &destination_track.id), 11);
        assert!(
            destination
                .track_by_id(&destination_track.id)
                .unwrap()
                .unwrap()
                .is_favorite
        );
        assert_eq!(
            destination.playlist_track_ids(&playlist.id).unwrap(),
            vec![destination_track.id.clone()]
        );
        let database_path = std::path::PathBuf::from(destination.connection.path().unwrap());
        drop(destination);
        let mut destination = LibraryDatabase::open(database_path).unwrap();
        assert_eq!(total(&destination, &destination_track.id), 11);
        destination
            .record_play(&destination_track.id, "post-restart-play")
            .unwrap();
        assert_eq!(total(&destination, &destination_track.id), 12);
        let (reexported, _) = destination.export_backup().unwrap();
        assert_eq!(reexported.tracks[0].all_time_total, 12);
        assert_eq!(reexported.coverage_sources.len(), 2);
        import_backup(&mut source, &reexported).unwrap();
        assert_eq!(total(&source, &source_track.id), 12);
        import_backup(&mut source, &older).unwrap();
        assert_eq!(total(&source, &source_track.id), 12);
    }

    #[test]
    fn portable_unmatched_and_ambiguous_history_survives_reexport_and_later_matching() {
        let fixture = Fixture::new();
        let track = fixture.track("first.wav", 23);
        let mut source = fixture.database();
        fixture.seed(&mut source, &[track.clone()]);
        event(&source, &track, "unmatched-event-one", 1_700_000_000);
        source
            .connection
            .execute(
                "UPDATE tracks SET play_count = play_count + 3 WHERE id = ?1",
                [&track.id],
            )
            .unwrap();
        let (backup, _) = source.export_backup().unwrap();
        let other = Fixture::new();
        let mut destination = other.database();
        assert_eq!(
            preview_backup(&destination, &backup)
                .unwrap()
                .unmatched_tracks,
            1
        );
        assert_eq!(
            import_backup(&mut destination, &backup)
                .unwrap()
                .pending_tracks,
            1
        );
        assert_eq!(
            import_backup(&mut destination, &backup)
                .unwrap()
                .imported_events,
            0
        );
        let (pending, _) = destination.export_backup().unwrap();
        assert_eq!(pending.tracks[0].all_time_total, 4);
        let mut third = other.database();
        import_backup(&mut third, &pending).unwrap();
        import_backup(&mut third, &backup).unwrap();
        assert_eq!(third.export_backup().unwrap().0.tracks.len(), 1);
        let a = other.track("copy-a.wav", 23);
        let b = other.track("copy-b.wav", 23);
        other.seed(&mut destination, &[a.clone(), b.clone()]);
        let preview = preview_backup(&destination, &backup).unwrap();
        assert_eq!(preview.ambiguous_tracks, 1);
        assert_eq!(preview.tracks[0].candidates.len(), 2);
        other.seed(&mut destination, &[a.clone()]);
        import_backup(&mut destination, &backup).unwrap();
        assert_eq!(total(&destination, &a.id), 4);
        assert_eq!(destination.export_backup().unwrap().0.tracks.len(), 1);
    }

    #[test]
    fn portable_conflicting_legacy_totals_and_changed_event_identity_refuse_without_mutation() {
        let fixture = Fixture::new();
        let track = fixture.track("one.wav", 11);
        let mut source = fixture.database();
        fixture.seed(&mut source, &[track.clone()]);
        source
            .connection
            .execute(
                "UPDATE tracks SET play_count = 5 WHERE id = ?1",
                [&track.id],
            )
            .unwrap();
        event(&source, &track, "conflict-event-one", 1_700_000_000);
        let (backup, _) = source.export_backup().unwrap();
        let other = Fixture::new();
        let copy = other.track("copy.wav", 11);
        let mut destination = other.database();
        other.seed(&mut destination, &[copy.clone()]);
        destination
            .connection
            .execute("UPDATE tracks SET play_count = 2 WHERE id = ?1", [&copy.id])
            .unwrap();
        assert!(!preview_backup(&destination, &backup)
            .unwrap()
            .conflicts
            .is_empty());
        assert!(import_backup(&mut destination, &backup).is_err());
        assert_eq!(total(&destination, &copy.id), 2);
        destination
            .connection
            .execute("UPDATE tracks SET play_count = 0 WHERE id = ?1", [&copy.id])
            .unwrap();
        import_backup(&mut destination, &backup).unwrap();
        let mut corrupt = backup.clone();
        corrupt.tracks[0].events[0].played_at_utc += 1;
        assert!(import_backup(&mut destination, &corrupt).is_err());
        assert_eq!(total(&destination, &copy.id), 6);
        corrupt = backup.clone();
        corrupt.tracks[0].all_time_total += 1;
        assert!(import_backup(&mut destination, &corrupt).is_err());
    }

    #[test]
    fn portable_original_legacy_event_and_baseline_survive_migration_and_source_reimport() {
        let fixture = Fixture::new();
        let track = fixture.track("legacy.wav", 5);
        let mut database = fixture.database();
        fixture.seed(&mut database, &[track.clone()]);
        database
            .connection
            .execute(
                "UPDATE tracks SET play_count = 8, last_played_at = 1700000000 WHERE id = ?1",
                [&track.id],
            )
            .unwrap();
        database
            .connection
            .execute(
                "DELETE FROM library_meta WHERE key = 'detailed_play_history_started_at_utc'",
                [],
            )
            .unwrap();
        database.migrate().unwrap();
        let original: String = database
            .connection
            .query_row("SELECT event_id FROM track_play_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(original.starts_with("legacy-last-played:"));
        let (backup, _) = database.export_backup().unwrap();
        assert_eq!(backup.tracks[0].events[0].event_id, original);
        assert_eq!(backup.tracks[0].baselines[0].undated_plays, 7);
        import_backup(&mut database, &backup).unwrap();
        assert_eq!(total(&database, &track.id), 8);
        database.migrate().unwrap();
        assert_eq!(total(&database, &track.id), 8);
    }

    #[test]
    fn portable_dated_only_import_refuses_unknown_overlap_with_undated_totals() {
        let fixture = Fixture::new();
        let track = fixture.track("source.wav", 31);
        let mut source = fixture.database();
        fixture.seed(&mut source, &[track.clone()]);
        event(&source, &track, "dated-only-event", 1_700_000_000);
        let (backup, _) = source.export_backup().unwrap();
        assert!(backup.tracks[0].baselines.is_empty());
        let other = Fixture::new();
        let copy = other.track("copy.wav", 31);
        let mut destination = other.database();
        other.seed(&mut destination, &[copy.clone()]);
        destination
            .connection
            .execute("UPDATE tracks SET play_count = 7 WHERE id = ?1", [&copy.id])
            .unwrap();
        assert!(!preview_backup(&destination, &backup)
            .unwrap()
            .conflicts
            .is_empty());
        assert!(import_backup(&mut destination, &backup).is_err());
        assert_eq!(total(&destination, &copy.id), 7);
        // Freezing the local baseline for export does not make the overlap safe.
        destination.export_backup().unwrap();
        assert!(import_backup(&mut destination, &backup).is_err());
        assert_eq!(total(&destination, &copy.id), 7);

        // Retaining events while music is unavailable must not bypass the same
        // overlap guard when the track becomes available or is associated later.
        let retained_fixture = Fixture::new();
        let mut retained = retained_fixture.database();
        import_backup(&mut retained, &backup).unwrap();
        let readded = retained_fixture.track("readded.wav", 31);
        retained_fixture.seed(&mut retained, &[readded.clone()]);
        retained
            .connection
            .execute(
                "UPDATE tracks SET play_count = 7 WHERE id = ?1",
                [&readded.id],
            )
            .unwrap();
        let preview = preview_backup(&retained, &backup).unwrap();
        assert_eq!(preview.duplicate_events, 1);
        assert!(!preview.conflicts.is_empty());
        assert!(import_backup(&mut retained, &backup).is_err());
        let transaction = retained.connection.transaction().unwrap();
        assert!(promote_pending(
            &transaction,
            &pending_id(&backup.source_id, &backup.tracks[0].reference.reference_id),
            &readded.id
        )
        .is_err());
        drop(transaction);
        assert_eq!(total(&retained, &readded.id), 7);
        assert_eq!(retained.export_backup().unwrap().0.tracks.len(), 2);
    }

    #[test]
    fn portable_rescan_retains_removed_history_and_never_matches_title_alone() {
        let fixture = Fixture::new();
        let a = fixture.track("a.wav", 13);
        let b = fixture.track("b.wav", 14);
        let mut database = fixture.database();
        fixture.seed(&mut database, &[a.clone()]);
        event(&database, &a, "removed-event-one", 1_700_000_000);
        database
            .connection
            .execute("UPDATE tracks SET play_count = 6 WHERE id = ?1", [&a.id])
            .unwrap();
        fixture.seed(&mut database, &[b.clone()]);
        let (backup, _) = database.export_backup().unwrap();
        assert_eq!(backup.tracks[0].all_time_total, 6);
        assert_eq!(
            preview_backup(&database, &backup).unwrap().unmatched_tracks,
            1
        );
        assert_eq!(total(&database, &b.id), 0);
        fixture.seed(&mut database, &[a.clone(), b.clone()]);
        assert_eq!(total(&database, &a.id), 6);
        assert_eq!(database.export_backup().unwrap().0.tracks.len(), 1);
    }

    #[test]
    fn portable_invalid_backups_and_transaction_failure_leave_existing_data_unchanged() {
        let fixture = Fixture::new();
        let track = fixture.track("one.wav", 17);
        let mut source = fixture.database();
        fixture.seed(&mut source, &[track.clone()]);
        event(&source, &track, "atomic-event-one", 1_700_000_000);
        event(&source, &track, "atomic-event-two", 1_700_000_001);
        let (backup, _) = source.export_backup().unwrap();
        let other = Fixture::new();
        let copy = other.track("two.wav", 17);
        let mut destination = other.database();
        other.seed(&mut destination, &[copy.clone()]);
        destination.connection.execute_batch("CREATE TRIGGER controlled_failure BEFORE INSERT ON track_play_events WHEN NEW.event_id = 'atomic-event-two' BEGIN SELECT RAISE(ABORT, 'controlled failure'); END;").unwrap();
        assert!(import_backup(&mut destination, &backup).is_err());
        assert_eq!(total(&destination, &copy.id), 0);
        assert_eq!(
            destination
                .connection
                .query_row("SELECT COUNT(*) FROM track_play_events", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        for mutate in [0, 1, 2, 3] {
            let mut invalid = backup.clone();
            match mutate {
                0 => invalid.version += 1,
                1 => invalid.tracks[0].events[0].played_at_utc = -1,
                2 => {
                    let duplicate = invalid.tracks[0].events[0].clone();
                    invalid.tracks[0].events.push(duplicate);
                }
                _ => invalid.tracks[0].reference.audio_key = "sha256-audio-v1:invalid".to_owned(),
            }
            assert!(validate_backup(&invalid).is_err());
        }
        let mut enormous = backup.clone();
        let source_id = enormous.source_id.clone();
        enormous.tracks[0].events.clear();
        enormous.tracks[0].all_time_total = MAX_EXACT_PLAYS;
        enormous.tracks[0].baselines = vec![PortableBaseline {
            source_id,
            origin_reference_id: enormous.tracks[0].reference.reference_id.clone(),
            undated_plays: MAX_EXACT_PLAYS,
        }];
        validate_backup(&enormous).unwrap();
        let mut another = enormous.tracks[0].clone();
        another.reference.reference_id = sha256_text("another-enormous-track");
        another.baselines[0].origin_reference_id = another.reference.reference_id.clone();
        enormous.tracks.push(another);
        assert!(validate_backup(&enormous).is_err());
        assert!(import_backup(&mut destination, &enormous).is_err());
        assert_eq!(total(&destination, &copy.id), 0);
    }

    #[test]
    fn portable_six_format_audio_identities_survive_tag_edits_and_unicode() {
        use super::super::tag_editor_tests::{synthetic_format_bytes, synthetic_pcm_wav};
        use super::super::{
            safe_update_track_tags_masked_with_hook, TagFieldMask, UpdateTrackTagsRequest,
        };
        let fixture = Fixture::new();
        for format in ["flac", "mp3", "ogg", "opus", "wav", "m4a"] {
            let bytes = if format == "wav" {
                synthetic_pcm_wav()
            } else {
                synthetic_format_bytes(format)
            };
            let path = fixture.directory.join(format!("identity.{format}"));
            fs::write(&path, bytes).unwrap();
            let before = audio_key(&path).unwrap();
            let request = UpdateTrackTagsRequest {
                track_id: path.to_string_lossy().into_owned(),
                title: Some("東京 — Café".to_owned()),
                artist: Some("Different artist α".to_owned()),
                album: None,
                album_artist: None,
                genre: None,
                year: None,
                track_number: None,
                disc_number: None,
            };
            safe_update_track_tags_masked_with_hook(
                &path,
                &request,
                TagFieldMask {
                    title: true,
                    artist: true,
                    ..TagFieldMask::default()
                },
                &mut |_| Ok(()),
            )
            .unwrap();
            assert_eq!(audio_key(&path).unwrap(), before, "{format}");
        }
    }

    #[test]
    fn portable_preview_is_stable_but_changes_when_backup_or_matching_changes() {
        let fixture = Fixture::new();
        let track = fixture.track("one.wav", 2);
        let mut database = fixture.database();
        fixture.seed(&mut database, &[track.clone()]);
        event(&database, &track, "preview-event-one", 1_700_000_000);
        let (backup, _) = database.export_backup().unwrap();
        let first = preview_backup(&database, &backup).unwrap().approval_token;
        assert_eq!(
            first,
            preview_backup(&database, &backup).unwrap().approval_token
        );
        let mut changed = backup.clone();
        changed.tracks[0].events[0].played_at_utc += 1;
        assert_ne!(
            first,
            preview_backup(&database, &changed).unwrap().approval_token
        );
        let copy = fixture.track("copy.wav", 2);
        fixture.seed(&mut database, &[track.clone(), copy]);
        assert_ne!(
            first,
            preview_backup(&database, &backup).unwrap().approval_token
        );
    }
}
