use super::listening_time::ListeningTimeStats;
use super::portable_history::CoverageSource;
use super::portable_history::PortableRef;
use super::{LibraryDatabase, Track};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrackPeriodCount {
    track_id: String,
    plays: i64,
    last_played_at: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListeningStats {
    track_counts: Vec<TrackPeriodCount>,
    recent_listens: Vec<RecentListen>,
    listening_time: ListeningTimeStats,
    daily_plays: Vec<i64>,
    total_plays: i64,
    dated_plays: i64,
    undated_legacy_plays: i64,
    detailed_history_started_at_utc: Option<i64>,
    pending_tracks: Vec<PendingTrack>,
    coverage_sources: Vec<CoverageSource>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentListen {
    event_id: String,
    track_id: String,
    played_at_utc: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingTrack {
    track_id: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    genres: Vec<String>,
}

impl LibraryDatabase {
    fn listening_stats(
        &self,
        start_utc: Option<i64>,
        end_utc: Option<i64>,
        day_boundaries_utc: &[i64],
        recent_limit: u32,
    ) -> Result<ListeningStats, String> {
        if start_utc.is_some() != end_utc.is_some()
            || start_utc
                .zip(end_utc)
                .is_some_and(|(start, end)| start >= end)
        {
            return Err("Choose a valid statistics period.".to_owned());
        }
        if day_boundaries_utc.len() > 36626
            || day_boundaries_utc.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("Invalid daily chart boundaries.".to_owned());
        }

        let mut track_counts = Vec::new();
        if let (Some(start), Some(end)) = (start_utc, end_utc) {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT track_id, COUNT(*), MAX(played_at_utc) \
                 FROM track_play_events \
                 WHERE played_at_utc >= ?1 AND played_at_utc < ?2 \
                   AND track_id NOT IN (SELECT track_id FROM ignored_tracks) \
                 GROUP BY track_id ORDER BY COUNT(*) DESC, track_id",
                )
                .map_err(|error| format!("Could not query playback history: {error}"))?;
            let rows = statement
                .query_map(params![start, end], |row| {
                    Ok(TrackPeriodCount {
                        track_id: row.get(0)?,
                        plays: row.get(1)?,
                        last_played_at: row.get(2)?,
                    })
                })
                .map_err(|error| format!("Could not query playback history: {error}"))?;
            for row in rows {
                track_counts.push(
                    row.map_err(|error| format!("Could not read playback history: {error}"))?,
                );
            }
        } else {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT id, play_count, last_played_at FROM tracks \
                 WHERE play_count > 0 AND id NOT IN (SELECT track_id FROM ignored_tracks) \
                 ORDER BY play_count DESC, id",
                )
                .map_err(|error| format!("Could not query all-time totals: {error}"))?;
            let rows = statement
                .query_map([], |row| {
                    Ok(TrackPeriodCount {
                        track_id: row.get(0)?,
                        plays: row.get(1)?,
                        last_played_at: row.get(2)?,
                    })
                })
                .map_err(|error| format!("Could not query all-time totals: {error}"))?;
            for row in rows {
                track_counts
                    .push(row.map_err(|error| format!("Could not read all-time totals: {error}"))?);
            }
            let mut statement = self.connection.prepare(
                "SELECT p.track_id, (SELECT COUNT(*) FROM track_play_events e WHERE e.track_id = p.track_id) \
                 + (SELECT COALESCE(SUM(undated_plays), 0) FROM history_legacy_baselines b WHERE b.track_id = p.track_id), \
                 (SELECT MAX(played_at_utc) FROM track_play_events e WHERE e.track_id = p.track_id) FROM history_pending_tracks p \
                 WHERE p.track_id NOT IN (SELECT track_id FROM ignored_tracks)",
            ).map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok(TrackPeriodCount {
                        track_id: row.get(0)?,
                        plays: row.get(1)?,
                        last_played_at: row.get(2)?,
                    })
                })
                .map_err(|error| error.to_string())?;
            for row in rows {
                track_counts.push(row.map_err(|error| error.to_string())?);
            }
        }

        let total_plays = track_counts.iter().map(|item| item.plays).sum();
        let dated_plays: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM track_play_events WHERE track_id NOT IN (SELECT track_id FROM ignored_tracks)", [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("Could not count dated plays: {error}"))?;
        let authoritative_total: i64 = self.connection.query_row(
            "SELECT (SELECT COALESCE(SUM(play_count), 0) FROM tracks WHERE id NOT IN (SELECT track_id FROM ignored_tracks)) \
             + (SELECT COUNT(*) FROM track_play_events e JOIN history_pending_tracks p ON p.track_id = e.track_id \
                WHERE p.track_id NOT IN (SELECT track_id FROM ignored_tracks)) \
             + (SELECT COALESCE(SUM(undated_plays), 0) FROM history_legacy_baselines b JOIN history_pending_tracks p ON p.track_id = b.track_id \
                WHERE p.track_id NOT IN (SELECT track_id FROM ignored_tracks))", [], |row| row.get(0),
        ).map_err(|error| format!("Could not count all-time plays: {error}"))?;

        let mut daily_plays = vec![0_i64; day_boundaries_utc.len().saturating_sub(1)];
        if day_boundaries_utc.len() > 1 {
            let mut statement = self.connection.prepare(
                "SELECT COUNT(*) FROM track_play_events WHERE played_at_utc >= ?1 AND played_at_utc < ?2 \
                 AND track_id NOT IN (SELECT track_id FROM ignored_tracks)",
            ).map_err(|error| format!("Could not query daily plays: {error}"))?;
            for (index, pair) in day_boundaries_utc.windows(2).enumerate() {
                daily_plays[index] = statement
                    .query_row(params![pair[0], pair[1]], |row| row.get(0))
                    .map_err(|error| format!("Could not query daily plays: {error}"))?;
            }
        }

        let recent_listens = if recent_limit == 0 {
            Vec::new()
        } else {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT event_id, track_id, played_at_utc FROM track_play_events \
                 WHERE played_at_utc >= ?1 AND played_at_utc < ?2 \
                   AND track_id NOT IN (SELECT track_id FROM ignored_tracks) \
                 ORDER BY played_at_utc DESC, event_id DESC LIMIT ?3",
                )
                .map_err(|error| format!("Could not query recent listens: {error}"))?;
            let rows = statement
                .query_map(
                    params![
                        start_utc.unwrap_or(i64::MIN),
                        end_utc.unwrap_or(i64::MAX),
                        recent_limit
                    ],
                    |row| {
                        Ok(RecentListen {
                            event_id: row.get(0)?,
                            track_id: row.get(1)?,
                            played_at_utc: row.get(2)?,
                        })
                    },
                )
                .map_err(|error| format!("Could not query recent listens: {error}"))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| format!("Could not read recent listens: {error}"))?
        };

        let mut pending_tracks = Vec::new();
        let mut statement = self
            .connection
            .prepare(
                "SELECT track_id, reference_json FROM history_pending_tracks ORDER BY track_id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (track_id, json) = row.map_err(|error| error.to_string())?;
            let reference: PortableRef =
                serde_json::from_str(&json).map_err(|error| error.to_string())?;
            pending_tracks.push(PendingTrack {
                track_id,
                title: reference.title,
                artist: reference.artist,
                album: reference.album,
                album_artist: reference.album_artist,
                genres: reference.genres,
            });
        }

        let detailed_history_started_at_utc = self
            .meta_value("detailed_play_history_started_at_utc")?
            .and_then(|value| value.parse().ok());
        let coverage_sources = {
            let mut statement = self
                .connection
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
        Ok(ListeningStats {
            track_counts,
            recent_listens,
            listening_time: self.listening_time_stats(
                start_utc,
                end_utc,
                day_boundaries_utc,
                recent_limit,
            )?,
            daily_plays,
            total_plays,
            dated_plays,
            undated_legacy_plays: (authoritative_total - dated_plays).max(0),
            detailed_history_started_at_utc,
            pending_tracks,
            coverage_sources,
        })
    }

    fn remove_listening_event(&mut self, event_id: &str) -> Result<Option<RemovedListen>, String> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let qualified_track: Option<String> = transaction
            .query_row(
                "DELETE FROM track_play_events WHERE event_id = ?1 RETURNING track_id",
                [event_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("Could not remove the listen: {error}"))?;
        let timed_track: Option<String> = transaction
            .query_row(
                "SELECT track_id FROM track_listening_minutes WHERE session_id = ?1 LIMIT 1",
                [event_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("Could not find listening time: {error}"))?;
        let Some(track_id) = qualified_track.as_ref().or(timed_track.as_ref()) else {
            return Ok(None);
        };
        if qualified_track.is_some() {
            transaction.execute(
                "UPDATE tracks SET play_count = MAX(0, play_count - 1), \
                 last_played_at = (SELECT MAX(played_at_utc) FROM track_play_events WHERE track_id = ?1) \
                 WHERE id = ?1",
                [track_id],
            ).map_err(|error| format!("Could not update playback totals: {error}"))?;
        }
        transaction
            .execute(
                "DELETE FROM track_listening_minutes WHERE session_id = ?1",
                [event_id],
            )
            .map_err(|error| format!("Could not remove listening time: {error}"))?;
        transaction
            .execute(
                "DELETE FROM counted_listening_sessions WHERE session_id = ?1",
                [event_id],
            )
            .map_err(|error| format!("Could not remove the listening session: {error}"))?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO removed_listening_sessions VALUES (?1)",
                [event_id],
            )
            .map_err(|error| format!("Could not keep the listen removed: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("Could not save listening history: {error}"))?;
        Ok(Some(RemovedListen {
            track: self.track_by_id(track_id)?,
            listened_ms: self.listening_time_for_track(track_id)?,
            track_id: track_id.clone(),
        }))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovedListen {
    track: Option<Track>,
    track_id: String,
    listened_ms: i64,
}

#[tauri::command(async)]
pub(crate) fn get_listening_stats(
    start_utc: Option<i64>,
    end_utc: Option<i64>,
    day_boundaries_utc: Vec<i64>,
    recent_limit: u32,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<ListeningStats, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.listening_stats(start_utc, end_utc, &day_boundaries_utc, recent_limit)
}

#[tauri::command(async)]
pub(crate) fn remove_listening_event(
    event_id: String,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<Option<RemovedListen>, String> {
    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.remove_listening_event(&event_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> LibraryDatabase {
        let database =
            LibraryDatabase::initialize(rusqlite::Connection::open_in_memory().unwrap()).unwrap();
        database.connection.execute_batch(
            "INSERT INTO tracks (id, title, file_path, file_name, extension, scanned_at, play_count) \
             VALUES ('a', 'A', '/isolated/a.wav', 'a.wav', 'wav', 1, 9), \
             ('b', 'B', '/isolated/b.wav', 'b.wav', 'wav', 1, 3);",
        ).unwrap();
        for (id, track_id, time) in [
            ("event-at-start", "a", 100),
            ("event-before-end", "a", 199),
            ("event-at-end", "b", 200),
        ] {
            database
                .connection
                .execute(
                    "INSERT INTO track_play_events VALUES (?1, ?2, ?3, ?3, 'qualified_play')",
                    params![id, track_id, time],
                )
                .unwrap();
        }
        database
    }

    #[test]
    fn stats_periods_are_half_open_and_all_time_includes_undated_legacy_totals() {
        let database = database();
        let selected = database
            .listening_stats(Some(100), Some(200), &[100, 150, 200], 50)
            .unwrap();
        assert_eq!(selected.total_plays, 2);
        assert_eq!(selected.track_counts.len(), 1);
        assert_eq!(selected.daily_plays, vec![1, 1]);
        assert_eq!(selected.undated_legacy_plays, 9);
        assert_eq!(selected.recent_listens[0].event_id, "event-before-end");
        assert_eq!(selected.recent_listens[1].event_id, "event-at-start");
        let next = database
            .listening_stats(Some(200), Some(300), &[200, 300], 50)
            .unwrap();
        assert_eq!(next.total_plays, 1);
        let all = database
            .listening_stats(None, None, &[100, 200, 300], 50)
            .unwrap();
        assert_eq!(all.total_plays, 12);
        assert_eq!(all.dated_plays, 3);
        assert_eq!(all.daily_plays, vec![2, 1]);
        assert_eq!(all.track_counts[0].plays, 9);
    }

    #[test]
    fn ignored_tracks_hide_legacy_and_dated_plays_and_keep_original_history() {
        let mut database = database();
        database
            .connection
            .execute("INSERT INTO ignored_tracks VALUES ('a')", [])
            .unwrap();
        database
            .record_play_at("a", "00000000-0000-4000-8000-000000000001", 175)
            .unwrap();
        let stats = database
            .listening_stats(None, None, &[100, 200, 300], 50)
            .unwrap();
        assert_eq!(stats.total_plays, 3);
        assert_eq!(stats.daily_plays, vec![0, 1]);
        assert_eq!(stats.dated_plays, 1);
        assert_eq!(stats.recent_listens.len(), 1);
        assert_eq!(stats.recent_listens[0].track_id, "b");
        let stats = database
            .listening_stats(Some(100), Some(200), &[], 50)
            .unwrap();
        assert_eq!(stats.total_plays, 0);
        database
            .connection
            .execute("DELETE FROM ignored_tracks", [])
            .unwrap();
        let restored = database
            .listening_stats(None, None, &[100, 200, 300], 50)
            .unwrap();
        assert_eq!(restored.total_plays, 12);
        assert_eq!(restored.daily_plays, vec![2, 1]);
    }

    #[test]
    fn removing_one_listen_preserves_other_events_and_undated_totals() {
        let mut database = database();
        let duplicate = "00000000-0000-4000-8000-000000000003";
        database.record_play_at("a", duplicate, 199).unwrap();
        let updated = database
            .remove_listening_event("event-before-end")
            .unwrap()
            .unwrap()
            .track
            .unwrap();
        assert_eq!(updated.play_count, 9);
        assert_eq!(updated.last_played_at, Some(199));
        assert!(database
            .remove_listening_event("event-before-end")
            .unwrap()
            .is_none());

        let latest = database
            .listening_stats(Some(100), Some(200), &[100, 150, 200], 1)
            .unwrap();
        assert_eq!(latest.total_plays, 2);
        assert_eq!(latest.daily_plays, vec![1, 1]);
        assert_eq!(latest.recent_listens.len(), 1);
        assert_eq!(latest.recent_listens[0].event_id, duplicate);
        assert_eq!(latest.recent_listens[0].played_at_utc, 199);

        let updated = database
            .remove_listening_event(duplicate)
            .unwrap()
            .unwrap()
            .track
            .unwrap();
        assert_eq!(updated.play_count, 8);
        assert_eq!(updated.last_played_at, Some(100));
        let updated = database
            .remove_listening_event("event-at-start")
            .unwrap()
            .unwrap()
            .track
            .unwrap();
        assert_eq!(updated.play_count, 7);
        assert_eq!(updated.last_played_at, None);
        let all = database.listening_stats(None, None, &[], 50).unwrap();
        assert_eq!(all.total_plays, 10);
        assert_eq!(all.undated_legacy_plays, 9);
        assert_eq!(all.dated_plays, 1);
        assert_eq!(all.recent_listens[0].event_id, "event-at-end");
    }

    #[test]
    fn listening_time_keeps_partial_plays_and_counts_each_minute_once() {
        use crate::listening_time::ListeningMinute;
        let mut database = database();
        let session = "time-fixture-a";
        let sample = |minute, listened| ListeningMinute {
            session_id: session.into(),
            track_id: "a".into(),
            minute_utc: minute,
            listened_ms: listened,
            last_sample_utc_ms: minute * 1000 + listened,
        };
        database
            .record_listening_minutes(vec![sample(60, 2400), sample(120, 2499)])
            .unwrap();
        let before = database
            .listening_stats(None, None, &[0, 120, 180], 10)
            .unwrap();
        assert!(before.listening_time.track_times.is_empty());
        assert_eq!(before.listening_time.session_count, 0);
        assert_eq!(before.listening_time.daily_listened_ms, [0, 0]);
        let totals = database
            .record_listening_minutes(vec![sample(60, 800), sample(120, 2600)])
            .unwrap();
        assert_eq!(totals["a"], 5000);
        let selected = database
            .listening_stats(Some(120), Some(180), &[120, 180], 10)
            .unwrap();
        assert_eq!(selected.listening_time.track_times["a"], 2600);
        assert_eq!(selected.total_plays, 0);
        assert_eq!(
            database
                .remove_listening_event(session)
                .unwrap()
                .unwrap()
                .listened_ms,
            0
        );
        let totals = database
            .record_listening_minutes(vec![sample(120, 2000)])
            .unwrap();
        assert_eq!(totals["a"], 0);
        assert_eq!(
            database
                .record_play_at("a", session, 180)
                .unwrap()
                .play_count,
            9
        );
    }

    #[test]
    fn upgrading_filters_existing_short_sessions_without_erasing_samples() {
        let mut database = database();
        database
            .connection
            .execute_batch(
                "DELETE FROM library_meta WHERE key = 'counted_listening_sessions_backfilled';
             INSERT INTO track_listening_minutes VALUES
                ('old-short-a', 'a', 60, 3000, 63000),
                ('old-short-b', 'a', 60, 3000, 66000),
                ('old-long', 'b', 60, 2000, 120000),
                ('old-long', 'b', 120, 3000, 123000);",
            )
            .unwrap();
        database.migrate_listening_time().unwrap();
        assert_eq!(database.listening_time_for_track("a").unwrap(), 0);
        assert_eq!(database.listening_time_for_track("b").unwrap(), 5000);
        assert_eq!(
            database
                .connection
                .query_row("SELECT COUNT(*) FROM track_listening_minutes", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            4
        );
    }

    #[test]
    fn stats_invalid_empty_ranges_and_dst_length_days() {
        let database = database();
        assert!(database.listening_stats(Some(100), None, &[], 50).is_err());
        assert!(database
            .listening_stats(Some(200), Some(100), &[], 50)
            .is_err());
        assert!(database
            .listening_stats(None, None, &[100, 100], 50)
            .is_err());
        let empty = database
            .listening_stats(Some(300), Some(400), &[300, 400], 50)
            .unwrap();
        assert_eq!(empty.total_plays, 0);
        assert_eq!(empty.daily_plays, vec![0]);
        let dst = database
            .listening_stats(Some(0), Some(172800), &[0, 23 * 3600, 48 * 3600], 50)
            .unwrap();
        assert_eq!(dst.daily_plays, vec![3, 0]);
    }

    #[test]
    fn stats_date_queries_use_existing_history_indexes_and_cover_the_full_list() {
        let database = database();
        let plan: String = database.connection.query_row(
            "EXPLAIN QUERY PLAN SELECT COUNT(*) FROM track_play_events WHERE played_at_utc >= 100 AND played_at_utc < 200",
            [], |row| row.get(3),
        ).unwrap();
        assert!(plan.contains("idx_track_play_events_played_at_utc"));
        for index in 0..125 {
            database.connection.execute("INSERT INTO tracks (id, title, file_path, file_name, extension, scanned_at, play_count) VALUES (?1, ?1, ?1, ?1, 'wav', 1, 1)", [format!("synthetic-{index:03}")]).unwrap();
        }
        let all = database.listening_stats(None, None, &[], 50).unwrap();
        assert_eq!(all.track_counts.len(), 127);
        assert_eq!(all.total_plays, 137);
    }
}
