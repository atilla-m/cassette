use super::portable_history::CoverageSource;
use super::portable_history::PortableRef;
use super::LibraryDatabase;
use rusqlite::params;
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
                 WHERE play_count > 0 ORDER BY play_count DESC, id",
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
                 (SELECT MAX(played_at_utc) FROM track_play_events e WHERE e.track_id = p.track_id) FROM history_pending_tracks p",
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
            .query_row("SELECT COUNT(*) FROM track_play_events", [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("Could not count dated plays: {error}"))?;
        let authoritative_total: i64 = self.connection.query_row(
            "SELECT (SELECT COALESCE(SUM(play_count), 0) FROM tracks) \
             + (SELECT COUNT(*) FROM track_play_events e JOIN history_pending_tracks p ON p.track_id = e.track_id) \
             + (SELECT COALESCE(SUM(undated_plays), 0) FROM history_legacy_baselines b JOIN history_pending_tracks p ON p.track_id = b.track_id)", [], |row| row.get(0),
        ).map_err(|error| format!("Could not count all-time plays: {error}"))?;

        let mut daily_plays = vec![0_i64; day_boundaries_utc.len().saturating_sub(1)];
        if day_boundaries_utc.len() > 1 {
            let mut statement = self.connection.prepare(
                "SELECT COUNT(*) FROM track_play_events WHERE played_at_utc >= ?1 AND played_at_utc < ?2",
            ).map_err(|error| format!("Could not query daily plays: {error}"))?;
            for (index, pair) in day_boundaries_utc.windows(2).enumerate() {
                daily_plays[index] = statement
                    .query_row(params![pair[0], pair[1]], |row| row.get(0))
                    .map_err(|error| format!("Could not query daily plays: {error}"))?;
            }
        }

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
            daily_plays,
            total_plays,
            dated_plays,
            undated_legacy_plays: (authoritative_total - dated_plays).max(0),
            detailed_history_started_at_utc,
            pending_tracks,
            coverage_sources,
        })
    }
}

#[tauri::command(async)]
pub(crate) fn get_listening_stats(
    start_utc: Option<i64>,
    end_utc: Option<i64>,
    day_boundaries_utc: Vec<i64>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<ListeningStats, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.".to_owned())?;
    library.listening_stats(start_utc, end_utc, &day_boundaries_utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> LibraryDatabase {
        let mut database = LibraryDatabase {
            connection: rusqlite::Connection::open_in_memory().unwrap(),
        };
        database.migrate().unwrap();
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
            .listening_stats(Some(100), Some(200), &[100, 150, 200])
            .unwrap();
        assert_eq!(selected.total_plays, 2);
        assert_eq!(selected.track_counts.len(), 1);
        assert_eq!(selected.daily_plays, vec![1, 1]);
        assert_eq!(selected.undated_legacy_plays, 9);
        let next = database
            .listening_stats(Some(200), Some(300), &[200, 300])
            .unwrap();
        assert_eq!(next.total_plays, 1);
        let all = database
            .listening_stats(None, None, &[100, 200, 300])
            .unwrap();
        assert_eq!(all.total_plays, 12);
        assert_eq!(all.dated_plays, 3);
        assert_eq!(all.daily_plays, vec![2, 1]);
        assert_eq!(all.track_counts[0].plays, 9);
    }

    #[test]
    fn stats_invalid_empty_ranges_and_dst_length_days() {
        let database = database();
        assert!(database.listening_stats(Some(100), None, &[]).is_err());
        assert!(database.listening_stats(Some(200), Some(100), &[]).is_err());
        assert!(database.listening_stats(None, None, &[100, 100]).is_err());
        let empty = database
            .listening_stats(Some(300), Some(400), &[300, 400])
            .unwrap();
        assert_eq!(empty.total_plays, 0);
        assert_eq!(empty.daily_plays, vec![0]);
        let dst = database
            .listening_stats(Some(0), Some(172800), &[0, 23 * 3600, 48 * 3600])
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
        let all = database.listening_stats(None, None, &[]).unwrap();
        assert_eq!(all.track_counts.len(), 127);
        assert_eq!(all.total_plays, 137);
    }
}
