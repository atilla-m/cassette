# Detailed listening statistics and portable history

This feature is development work on `feature/detailed-stats`, not a change to the published beta.4 packages. Lyrics editing and Windows release work remain separate.

## Periods and accuracy

Stats supports All time, Today, This week, This month, This year, a selected earlier month/year, and an inclusive custom date range. Weeks start **Monday**. Calendar boundaries are local midnights converted independently to UTC; queries use `start <= timestamp < end`. A daylight-saving day can contain 23 or 25 hours. Changing the device timezone changes calendar attribution, not the stored UTC timestamp.

All-time totals remain authoritative `tracks.play_count` values, plus retained history for tracks absent from the library. Dated periods and the daily chart use the existing indexed event ledger. All-time charts explicitly show the most recent 30 local days; they are not a dated reconstruction of undated plays. Lists load incrementally, and longer charts show 90 days at a time. Recently Played still means unique tracks ordered by their most recent qualifying play, not a list of individual events.

The qualifying-play threshold and duplicate-safe event recording are unchanged. Undated legacy plays appear only in all-time totals. Genuinely known legacy timestamps keep their original identities and dates. Coverage records retain each profile's actual detailed-tracking start; an earlier legacy last-played timestamp is **not** evidence that complete tracking started then. Imported sources can have different start dates. No dates are invented for old totals.

Track, album, artist and genre rankings reuse current library grouping and genre assignments. They describe **current metadata**, not historical snapshots. Retained tracks absent from the library use the backup's descriptive labels and cannot be played until associated.

## Export and restore

Use **Stats → Export listening history** or **Import listening history…**. Export always snapshots the entire database history, regardless of the displayed period. Version 1 JSON contains original event IDs, UTC timestamps, source coverage, portable track references and provenance-labelled undated baselines. It contains no audio, credentials or explicit absolute-path fields. Existing backup files are never overwritten. Original beta.3 legacy event IDs are opaque identifiers that historically encoded a path; preserving those IDs is necessary for deduplication. Treat backups as private listening-history data.

Portable matching uses a SHA-256 identity of encoded audio payload and codec properties, excluding editable tags. A copied or renamed file can match at a different folder path; database row IDs and titles alone cannot. Supported identity readers cover FLAC, MP3, Ogg/Vorbis, Opus, WAV and M4A/AAC. This is **not an acoustic fingerprint**: transcoded or re-encoded copies generally do not match. If identical audio occurs more than once, title/artist/album/track/disc metadata can disambiguate it; otherwise the preview reports ambiguity rather than guessing.

The preview shows matched, unmatched and ambiguous references, new/duplicate events, source legacy totals and conflicts. Canceling the preview changes no history. Applying an import revalidates the exact preview and uses one SQLite transaction. Original event identities deduplicate repeat imports; importing an older backup cannot erase newer plays or lower a known baseline. New qualifying plays continue normally after restoration. Multiple source tracks resolving to one destination track are refused rather than silently combined.

Unmatched or ambiguous history is retained, exported again and included in statistics. After adding the original music, reopen the same backup to resolve newly available matches. For ambiguity, import first, then choose an audio-identical library track and use **Associate retained history**. Audio identities are checked in native code. History with no verified audio reference cannot be safely associated automatically; keep the original backup. A rescan archives removed tracks' known history rather than truncating it; readding unchanged audio at its original identity restores the archived counts.

All-time totals are **not** blindly added. A known undated baseline is identified by its original source and reference; overlapping versions use the larger already-known baseline, not their sum. Unrelated undated baselines, inconsistent original IDs/timestamps, changed coverage provenance, or new dated events whose overlap with existing undated totals is unknown cause an explicit refusal. There is no “force import” that guesses those dates or totals. Restore into an empty disposable profile when independent devices' legacy totals cannot be reconciled.

Imports are limited to 256 MiB and two million events; custom chart ranges to 100 years. Backup totals must fit JSON's exact-integer range (at most 9,007,199,254,740,991), and overflow is refused rather than rounded. Audio-identity verification reads the library files and can take time for a large library. Queries and transfer commands run off the UI thread; transfer previews and ranking lists render incrementally. Qualifying-play times are captured before waiting for the library lock, and out-of-order writes cannot move the latest-played timestamp backward.

## Isolated manual diagnostic

Build the native executable with `npm run tauri -- build --no-bundle`, then run `node scripts/prepare-detailed-stats-diagnostic.mjs /absolute/new/test-directory`. The script refuses an existing destination, copies the diagnostic binary, generates 120 disposable WAV/LRC fixtures and seeds two independent profiles at different music paths. It never reads your library. The generated `launch.sh` accepts `source` (default) or `destination` and pins all four XDG directories. `EXPECTED.json` records initial totals and dates; the profiles remain persistent for restart checks.

Manual checks remain pending until reported by the tester:

1. Source: inspect periods, daily values/empty states, complete lists, keyboard controls and refresh after a qualifying play.
2. Export while Today is selected; confirm the preview/restoration still includes the full history.
3. Destination: preview the source backup, cancel, verify zero plays; then import and compare All time and dated periods with Source.
4. Import the same backup again: no new events/count increase. Play a track, then import the older backup: the new play remains. Reopen both profiles to verify persistence.
