use super::{validate_play_event_id, LibraryDatabase};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::State;

const MINIMUM_LISTENING_MS: i64 = 5000;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListeningMinute {
    pub(super) session_id: String,
    pub(super) track_id: String,
    pub(super) minute_utc: i64,
    pub(super) listened_ms: i64,
    pub(super) last_sample_utc_ms: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListeningTimeStats {
    pub(super) track_times: HashMap<String, i64>,
    pub(super) daily_listened_ms: Vec<i64>,
    total_listened_ms: i64,
    recent_sessions: Vec<ListeningSession>,
    pub(super) session_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListeningSession {
    event_id: String,
    track_id: String,
    played_at_utc: i64,
    listened_ms: i64,
}

impl LibraryDatabase {
    pub(super) fn migrate_listening_time(&mut self) -> rusqlite::Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS track_listening_minutes (
                session_id TEXT NOT NULL,
                track_id TEXT NOT NULL,
                minute_utc INTEGER NOT NULL,
                listened_ms INTEGER NOT NULL CHECK(listened_ms BETWEEN 1 AND 60000),
                last_sample_utc_ms INTEGER NOT NULL,
                PRIMARY KEY (session_id, minute_utc)
            );
            CREATE INDEX IF NOT EXISTS listening_minutes_track ON track_listening_minutes (track_id);
            CREATE INDEX IF NOT EXISTS listening_minutes_date ON track_listening_minutes (minute_utc);
            CREATE TABLE IF NOT EXISTS removed_listening_sessions (session_id TEXT PRIMARY KEY NOT NULL);
            CREATE TABLE IF NOT EXISTS counted_listening_sessions (session_id TEXT PRIMARY KEY NOT NULL);
            CREATE VIEW IF NOT EXISTS counted_listening_minutes AS
                SELECT minutes.* FROM track_listening_minutes AS minutes
                JOIN counted_listening_sessions USING (session_id);",
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO counted_listening_sessions (session_id)
             SELECT session_id FROM track_listening_minutes
             WHERE NOT EXISTS (
                 SELECT 1 FROM library_meta WHERE key = 'counted_listening_sessions_backfilled'
             )
             GROUP BY session_id HAVING SUM(listened_ms) >= ?1",
            [MINIMUM_LISTENING_MS],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO library_meta (key, value)
             VALUES ('counted_listening_sessions_backfilled', '1')",
            [],
        )?;
        transaction.commit()
    }

    pub(super) fn record_listening_minutes(
        &mut self,
        minutes: Vec<ListeningMinute>,
    ) -> Result<HashMap<String, i64>, String> {
        if minutes.len() > 512 {
            return Err("Too many listening samples.".into());
        }
        let mut ids = HashSet::new();
        for minute in &minutes {
            validate_play_event_id(&minute.session_id)?;
            if !(0..=253_402_300_740).contains(&minute.minute_utc)
                || minute.minute_utc % 60 != 0
                || !(1..=60_000).contains(&minute.listened_ms)
                || !(minute.minute_utc * 1000..=(minute.minute_utc + 60) * 1000)
                    .contains(&minute.last_sample_utc_ms)
            {
                return Err("Invalid listening time sample.".into());
            }
            ids.insert(minute.track_id.as_str());
        }
        for id in &ids {
            self.cache_history_reference(id);
        }
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| error.to_string())?;
        {
            let mut write = transaction
                .prepare(
                    "INSERT INTO track_listening_minutes
                    (session_id, track_id, minute_utc, listened_ms, last_sample_utc_ms)
                 SELECT ?1, ?2, ?3, ?4, ?5
                 WHERE EXISTS (SELECT 1 FROM tracks WHERE id = ?2)
                   AND NOT EXISTS (SELECT 1 FROM ignored_tracks WHERE track_id = ?2)
                   AND NOT EXISTS (SELECT 1 FROM removed_listening_sessions WHERE session_id = ?1)
                 ON CONFLICT (session_id, minute_utc) DO UPDATE SET
                   listened_ms = MAX(listened_ms, excluded.listened_ms),
                   last_sample_utc_ms = MAX(last_sample_utc_ms, excluded.last_sample_utc_ms)
                 WHERE track_listening_minutes.track_id = excluded.track_id",
                )
                .map_err(|error| error.to_string())?;
            for minute in &minutes {
                write
                    .execute(params![
                        minute.session_id,
                        minute.track_id,
                        minute.minute_utc,
                        minute.listened_ms,
                        minute.last_sample_utc_ms,
                    ])
                    .map_err(|error| format!("Could not save listening time: {error}"))?;
            }
        }
        {
            let sessions: HashSet<_> = minutes.iter().map(|minute| &minute.session_id).collect();
            let mut qualify = transaction.prepare(
                "INSERT OR IGNORE INTO counted_listening_sessions (session_id)
                 SELECT ?1
                 WHERE NOT EXISTS (SELECT 1 FROM counted_listening_sessions WHERE session_id = ?1)
                   AND (SELECT SUM(listened_ms) FROM track_listening_minutes WHERE session_id = ?1) >= ?2",
            ).map_err(|error| error.to_string())?;
            for session in sessions {
                qualify
                    .execute(params![session, MINIMUM_LISTENING_MS])
                    .map_err(|error| format!("Could not count the listening session: {error}"))?;
            }
        }
        transaction.commit().map_err(|error| error.to_string())?;
        ids.into_iter()
            .map(|id| {
                self.listening_time_for_track(id)
                    .map(|time| (id.into(), time))
            })
            .collect()
    }

    pub(super) fn listening_time_for_track(&self, id: &str) -> Result<i64, String> {
        self.connection.query_row(
            "SELECT COALESCE(SUM(listened_ms), 0) FROM counted_listening_minutes WHERE track_id = ?1",
            [id], |row| row.get(0),
        ).map_err(|error| format!("Could not read listening time: {error}"))
    }

    pub(super) fn listening_time_by_track(
        &self,
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<HashMap<String, i64>, String> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT track_id, SUM(listened_ms) FROM counted_listening_minutes
             WHERE minute_utc >= ?1 AND minute_utc < ?2
             GROUP BY track_id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(
                params![start.unwrap_or(i64::MIN), end.unwrap_or(i64::MAX)],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| error.to_string())
    }

    pub(super) fn listening_time_stats(
        &self,
        start: Option<i64>,
        end: Option<i64>,
        boundaries: &[i64],
        recent_limit: u32,
    ) -> Result<ListeningTimeStats, String> {
        let mut track_times = self.listening_time_by_track(start, end)?;
        for id in self.ignored_track_ids()? {
            track_times.remove(&id);
        }
        let total_listened_ms = track_times.values().sum();
        let mut daily_listened_ms = vec![0; boundaries.len().saturating_sub(1)];
        if let (Some(&first), Some(&last)) = (boundaries.first(), boundaries.last()) {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT minute_utc, SUM(listened_ms) FROM counted_listening_minutes
                 WHERE minute_utc >= ?1 AND minute_utc < ?2
                   AND track_id NOT IN (SELECT track_id FROM ignored_tracks)
                 GROUP BY minute_utc",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map(params![first, last], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|error| error.to_string())?;
            for row in rows {
                let (minute, listened_ms) = row.map_err(|error| error.to_string())?;
                let index = boundaries.partition_point(|boundary| *boundary <= minute) - 1;
                daily_listened_ms[index] += listened_ms;
            }
        }

        let start = start.unwrap_or(i64::MIN);
        let end = end.unwrap_or(i64::MAX);
        let session_count = self
            .connection
            .query_row(
                "SELECT COUNT(DISTINCT session_id) FROM counted_listening_minutes
             WHERE minute_utc >= ?1 AND minute_utc < ?2
               AND track_id NOT IN (SELECT track_id FROM ignored_tracks)",
                params![start, end],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let mut statement = self.connection.prepare(
            "SELECT session_id, track_id, MAX(last_sample_utc_ms) / 1000, SUM(listened_ms)
             FROM counted_listening_minutes
             WHERE minute_utc >= ?1 AND minute_utc < ?2
               AND track_id NOT IN (SELECT track_id FROM ignored_tracks)
             GROUP BY session_id, track_id ORDER BY MAX(last_sample_utc_ms) DESC, session_id DESC LIMIT ?3",
        ).map_err(|error| error.to_string())?;
        let recent_sessions = statement
            .query_map(params![start, end, recent_limit], |row| {
                Ok(ListeningSession {
                    event_id: row.get(0)?,
                    track_id: row.get(1)?,
                    played_at_utc: row.get(2)?,
                    listened_ms: row.get(3)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;

        Ok(ListeningTimeStats {
            track_times,
            daily_listened_ms,
            total_listened_ms,
            recent_sessions,
            session_count,
        })
    }
}

#[tauri::command(async)]
pub(crate) fn save_listening_minutes(
    minutes: Vec<ListeningMinute>,
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<HashMap<String, i64>, String> {
    let mut library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.")?;
    library.record_listening_minutes(minutes)
}

#[tauri::command(async)]
pub(crate) fn get_listening_time_by_track(
    library: State<'_, Mutex<LibraryDatabase>>,
) -> Result<HashMap<String, i64>, String> {
    let library = library
        .lock()
        .map_err(|_| "Library cache is unavailable.")?;
    library.listening_time_by_track(None, None)
}
