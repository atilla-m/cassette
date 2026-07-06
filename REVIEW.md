# Cassette Codebase Review

Tiny fixes made during this review: none. This pass only updates `REVIEW.md`.

Validation run:

- `npm run build`: passed
- `npm run check`: passed, 0 errors/warnings
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed
- `git diff --check`: passed
- Automated tests: none found in `src` or `src-tauri`; no test script is defined in `package.json`

## 1. Executive Summary

Overall health: Cassette has a large amount of product behavior wired end-to-end, but the implementation is concentrated in two very large files: `src/routes/+page.svelte` at 11,454 lines and `src-tauri/src/lib.rs` at 6,370 lines. The app builds cleanly today, and TypeScript/Svelte diagnostics are clean, but the current structure makes future changes risky.

Biggest risks:

- `src/routes/+page.svelte` owns routing, page state, playback orchestration, queueing, lyrics, CD ripping, DVD import, video playback controls, settings, modals, and most derived data.
- `src-tauri/src/lib.rs` mixes command handlers, SQLite schema/query code, scanning, GStreamer playback, mpv IPC, lyrics, CD ripping, DVD import, MusicBrainz/Cover Art Archive, and utility parsing.
- Audio scanning is synchronous in `scan_library`, unlike the video/CD/DVD paths that use `spawn_blocking`.
- App shutdown does not explicitly stop GStreamer/mpv or save active video progress.
- Tauri CSP is disabled.
- There are no automated tests around the highest-risk queue, parser, schema, lyrics, or process-control logic.

What is surprisingly good:

- External commands are spawned with `Command::new` and explicit args, not shell strings.
- CD/DVD/mpv dependency failures generally produce user-facing errors.
- DVD import explicitly treats unsupported/encrypted media as unsupported and does not add DRM bypass logic.
- Large video/CD/DVD operations are mostly moved off the async runtime with `spawn_blocking`.
- Frontend global listeners and intervals are cleaned up in `onMount` teardown.
- Keyboard shortcuts ignore inputs, textareas, selects, and contenteditable elements.
- LRCLIB lookup sends track metadata only; it does not send local file paths.

What should not be touched yet:

- Do not rewrite audio playback, video playback, CD ripping, and DVD import at the same time.
- Do not change track/video IDs away from file paths without a migration for playlists, favorites, play counts, lyrics offsets/cache, and video progress.
- Do not silently delete stale playlist entries unless preserving missing tracks is no longer a product requirement.
- Do not replace mpv or GStreamer while doing UI cleanup.
- Do not change schema behavior without a versioned migration plan and restart/rescan tests.

## 2. Critical Issues

### C1. App shutdown does not explicitly stop playback/video or save active video progress

Files/functions:

- `src-tauri/src/lib.rs:6297` `run`
- `src-tauri/src/lib.rs:4208` `VideoPlaybackState`
- `src-tauri/src/lib.rs:4526` `VideoPlaybackState::stop_process`
- `src-tauri/src/lib.rs:4606` `save_active_video_progress`

The Tauri builder registers state and commands, but there is no app/window close handler or `Drop` cleanup for the playback states. Normal stop/close commands save progress and stop mpv, but closing the app while media is active can skip that logic.

Impact: mpv can outlive the app on some exit paths, video progress can be lost, and GStreamer resources may not be released cleanly.

Recommended fix: add a close/exit cleanup path that saves active video progress, stops mpv, and sets GStreamer playback to `Null`.

### C2. Tauri CSP is disabled

File:

- `src-tauri/tauri.conf.json:20`

`"csp": null` removes an important defense-in-depth layer. The current UI does not appear to use `{@html}`, `innerHTML`, `eval`, or dynamic script injection, so this is not an immediate exploit by itself. But the app handles untrusted metadata, lyrics, local paths, remote MusicBrainz/Cover Art Archive/LRCLIB responses, and local images.

Impact: a future rendering bug could become a more serious desktop-app security issue than necessary.

Recommended fix: add a restrictive CSP compatible with Tauri assets, local images, and dev mode.

## 3. High-Priority Issues

### H1. `+page.svelte` is too large to safely extend

File:

- `src/routes/+page.svelte`

The page contains almost all user workflows. Examples include playback polling and shortcuts around `src/routes/+page.svelte:591`, lyrics selection around `src/routes/+page.svelte:786`, video controls around `src/routes/+page.svelte:1060`, queue logic around `src/routes/+page.svelte:2896`, and CD/DVD UI later in the same file.

Impact: unrelated features share too much state. Subtle regressions are likely as new work continues.

Recommended fix: split by workflow first: `NowPlayingPage`, `LyricsPanel`, `VideosPage`, `CdRipperPage`, `PlaylistsPage`, `SettingsPage`, and extracted queue/lyrics helpers.

### H2. `lib.rs` mixes unrelated backend domains

File:

- `src-tauri/src/lib.rs`

This file contains schema, command handlers, scans, lyrics, CD ripping, DVD import, playback, mpv IPC, and parsers.

Impact: lock boundaries, process cleanup, and migration behavior are harder to audit.

Recommended split: `db.rs`, `audio.rs`, `video.rs`, `lyrics.rs`, `cd.rs`, `dvd.rs`, and command registration.

### H3. Audio library scanning runs synchronously

File/function:

- `src-tauri/src/lib.rs:803` `scan_library`

`scan_library` validates the path, then directly traverses files and reads metadata/art/lyrics before locking SQLite for replacement. Video scanning uses `spawn_blocking` at `src-tauri/src/lib.rs:548`; audio scanning should follow that pattern.

Impact: large libraries can make the app feel frozen and can starve other commands.

Recommended fix: perform traversal and metadata extraction in `tauri::async_runtime::spawn_blocking`, then lock SQLite only for `replace_library`.

### H4. Failed scans clear the in-memory library before success

File/function:

- `src/routes/+page.svelte:903` `scanFolderIntoLibrary`

The frontend sets `tracks = []`, clears selected views, clears queue state, and updates `scannedFolder` before `await scanLibrary(folder)` succeeds. If scanning fails, the database cache is not necessarily lost, but the current UI can appear empty until reload/cache load.

Impact: a bad folder choice or backend scan error can make the user think the library disappeared.

Recommended fix: scan into a temporary result, then replace visible state only after success. Keep a separate pending scan state for progress.

### H5. LRCLIB selection is not persisted as the preferred lyrics source

Files/functions:

- `src/routes/+page.svelte:822` `handleSelectLyricsResult`
- `src-tauri/src/lib.rs:973` `read_track_lyrics`
- `src-tauri/src/lib.rs:995` local lyrics priority
- `src-tauri/src/lib.rs:5991` `save_app_lyrics`

Selecting an LRCLIB result writes Cassette's cache and updates current UI state, but `read_track_lyrics` still prefers scanned/adjacent local lyrics over app-cached LRCLIB lyrics. If a local `.lrc`/`.txt` exists, the explicit online choice can be hidden after restart or track reload.

Impact: the result picker appears to work, then silently reverts later.

Recommended fix: persist a per-track preferred lyrics source such as `local` or `cached_lrclib` and honor it in `read_track_lyrics`.

### H6. Video thumbnails are generated outside the configured asset scope

Files:

- `src-tauri/tauri.conf.json:22`
- `src-tauri/src/lib.rs:546`
- `src-tauri/src/lib.rs:2180`
- `src-tauri/src/lib.rs:5329`
- `src/lib/utils/localImage.ts:3`

The asset protocol scope allows `$APPDATA/cover-art/**`, but generated video thumbnails are under `$APPDATA/video-thumbnails`. The UI uses `convertFileSrc`, so production thumbnail display can fail depending on Tauri asset-scope enforcement.

Impact: video thumbnails can be broken even when generation succeeds.

Recommended fix: add `$APPDATA/video-thumbnails/**` to the asset protocol scope.

### H7. Shuffle actions create shuffled queues while showing shuffle as off

Files/functions:

- `src/routes/+page.svelte:2297`, `src/routes/+page.svelte:2311`, `src/routes/+page.svelte:2324`, `src/routes/+page.svelte:2337`
- `src/routes/+page.svelte:3240` `playTrackSet`
- `src/routes/+page.svelte:513` `playbackOrder`

`playTrackSet(libraryTracks, true)` shuffles the queue, but then sets `isShuffleEnabled = false` and clears `shuffledQueueOrder`. The queue will play in the shuffled static order, while the player reports shuffle off. Genre and mix shuffle paths set shuffle on, so behavior is inconsistent.

Impact: normal users get misleading playback state and repeat/queue behavior is harder to reason about.

Recommended fix: decide whether these are one-shot randomized play actions or true shuffle mode, then make the indicator and queue order match.

### H8. No automated tests exist for high-risk logic

Search found no app test files and no Rust `#[test]` functions.

Best places to add tests later:

- queue order/repeat/shuffle helpers after extraction from `+page.svelte`
- LRC parsing, offset, and active-line selection
- CD/DVD parser functions
- path/filename sanitization
- LRCLIB match scoring/labels
- SQLite migrations and playlist missing-track behavior

## 4. Medium-Priority Issues

### M1. Schema lacks versioned migrations and useful indexes

File/function:

- `src-tauri/src/lib.rs:2642` `LibraryDatabase::migrate`

Migration uses `CREATE TABLE IF NOT EXISTS` plus column checks. There is no `PRAGMA user_version` migration chain, no migration transaction, and no explicit indexes for common playlist operations.

Recommended indexes:

- `playlist_tracks(playlist_id, position)`
- `playlist_tracks(track_id)`
- consider `tracks(file_path)` if `id` and `file_path` ever diverge

### M2. Playlist tracks intentionally do not reference `tracks`, but the behavior needs a repair path

File:

- `src-tauri/src/lib.rs:2696`

`playlist_tracks` references playlists but not tracks. This preserves missing tracks across rescans, which appears intentional because the UI counts unavailable entries at `src/routes/+page.svelte:3799`. It also means stale rows can accumulate forever.

Recommended fix: document the choice and add a Library Health cleanup/repair action later.

### M3. CD rip FLAC failure leaves temporary WAV files

File/function:

- `src-tauri/src/lib.rs:1925` `rip_single_track_to_flac`

The WAV is removed when `cdparanoia` fails and after successful FLAC encoding, but if `flac` returns a non-success status at `src-tauri/src/lib.rs:1956`, the WAV remains.

Impact: failed rips can leave large temporary files in the output folder.

Recommended fix: choose a policy. Either delete the WAV on encode failure or explicitly report that it was kept.

### M4. DVD import failure can leave partial output

Files/functions:

- `src-tauri/src/lib.rs:2101` creates import folder
- `src-tauri/src/lib.rs:2137` stream-copy attempt
- `src-tauri/src/lib.rs:2164` encode fallback

Failed imports can leave partial MKV files and empty/partial folders.

Recommended fix: write to a temporary filename and rename on success, or clean partial output on failure.

### M5. MusicBrainz/Cover Art Archive rate-limit handling is minimal

Files/functions:

- `src-tauri/src/lib.rs:36` `MUSICBRAINZ_USER_AGENT`
- `src-tauri/src/lib.rs:1259` `lookup_musicbrainz_disc`
- `src-tauri/src/lib.rs:1493` `lookup_cover_art_archive`

The code sets a User-Agent, which is necessary. It does not handle `Retry-After`, throttle repeated lookups, or cache Disc ID responses.

Recommended fix: handle HTTP 429/503 and `Retry-After`; consider local response caching.

### M6. Polling loops can overlap slow backend calls

File:

- `src/routes/+page.svelte:614`
- `src/routes/+page.svelte:645`

Audio and video status polling use fixed intervals. If a backend call stalls, the next tick can start before the previous one completes.

Recommended fix: add `isRefreshingPlaybackStatus` and `isRefreshingVideoStatus` guards.

### M7. Context menu accessibility is incomplete

File:

- `src/lib/components/ContextMenu.svelte`

The menu has `role="menu"` and closes on Escape/outside click. It does not focus itself on open, does not focus the first enabled item, and does not implement ArrowUp/ArrowDown/Home/End navigation.

Recommended fix: add focus management and roving keyboard navigation.

### M8. Track rows use `role="button"` with nested buttons

File:

- `src/lib/components/TrackList.svelte:129`

The row is an interactive `div role="button"` containing nested buttons for artist, album, favorite, remove, and move controls.

Impact: screen-reader and keyboard interaction can be awkward.

Recommended fix: later, separate the row selection control from secondary controls or use a semantic list/table pattern.

### M9. Noncritical media failures are silent

Examples:

- `src-tauri/src/lib.rs:5334` thumbnail directory creation failure collapses to `None`
- `src-tauri/src/lib.rs:5360` ffmpeg thumbnail failure collapses to `None`
- `src-tauri/src/lib.rs:5511` and `src-tauri/src/lib.rs:5531` cover cache writes use `.ok()?`

Recommended fix: keep nonfatal behavior but surface warnings in scan/import results or Library Health diagnostics.

### M10. `window.prompt` remains for playlist create/rename from context menus

File:

- `src/routes/+page.svelte:2614`
- `src/routes/+page.svelte:2724`

This is acceptable for now but inconsistent with the custom delete confirmation dialog.

Recommended fix: replace with existing modal/panel patterns when playlist UI is split.

## 5. Low-Priority Issues

- `package.json` description is empty and `Cargo.toml` still has placeholder metadata.
- Main window title is lowercase `"cassette"` in `src-tauri/tauri.conf.json:15`.
- `src/lib/types/library.ts` is already broad; split it when backend domains split.
- `src/lib/api/library.ts` is a clean wrapper, but it is becoming a kitchen sink for all non-playback commands.
- `src-tauri/src/mpris.rs` reports static loop/shuffle properties; it does not reflect app repeat/shuffle state.
- Local image error handlers are repeated across components/page markup.
- Many pure helper functions are embedded in `+page.svelte` and cannot be tested in place.

## 6. Suggested Next 5 Fixes

1. Add app/window shutdown cleanup for active GStreamer and mpv playback, including video progress save.
2. Move `scan_library` traversal/metadata work into `spawn_blocking` and avoid clearing UI state until a scan succeeds.
3. Add a restrictive Tauri CSP and expand asset protocol scope for video thumbnails.
4. Persist per-track lyrics source preference so selected LRCLIB lyrics survive restart even when local lyrics exist.
5. Extract and test queue ordering, LRC parsing/offset, path sanitization, CD/DVD parsers, and LRCLIB match scoring.

## 7. Suggested Test Plan

Fresh install/no library:

- Launch with no SQLite database.
- Verify empty Home/Songs/Albums/Artists/Genres states.
- Verify Now Playing and bottom player do not show active playback.
- Open Settings and Library Health without scanning.

Audio library scanning:

- Scan a folder with FLAC/MP3/OGG/OPUS/WAV/M4A.
- Include nested folders, bad/unreadable files, missing tags, embedded art, folder art, local `.lrc`, local `.txt`.
- Try a scan that fails and verify existing visible library state is preserved after the fix.
- Rescan after deleting/moving a file.
- Scan a different root and confirm expected replacement behavior.

Audio playback:

- Play from Songs, Albums, Artists, Genres, Liked Songs, custom playlist, and Mix Builder.
- Test play/pause/resume/seek/volume.
- Test missing file playback error.
- Test previous/next with one item, multiple items, and queue end.
- Test repeat off/all/one.
- Test shuffle on/off before and during playback.
- Test Shuffle Album/Artist/Genre/Playlist and confirm UI indicator matches behavior.
- Confirm play count increments once after threshold.
- Confirm MPRIS play/pause/next/previous/seek works.

Queue/playlists:

- Add next, add to end, clear queue.
- Reorder playlist tracks.
- Remove tracks from playlist.
- Delete playlist and cancel delete.
- Rescan library with playlist tracks missing and confirm missing count/UI.

Lyrics:

- Track with local `.lrc`: load, highlight, click-to-seek.
- Track with local `.txt`: display plain lyrics.
- Track with no local lyrics: Auto Find, picker opens, select synced result.
- Restart app and verify cached LRCLIB reloads.
- Track with both local lyrics and selected LRCLIB cache: verify chosen source persists after the preference fix.
- Find Again and choose a different LRCLIB result.
- Remove cached lyrics and verify local files are not deleted.
- Set offset `+0.5s`, verify highlight delay, click seek behavior, and restart persistence.
- Test LRCLIB offline/network failure.
- Confirm no local file path is sent to LRCLIB.

CD ripping:

- Detect with no drive, empty drive, audio CD.
- Metadata lookup with libdiscid missing and installed.
- MusicBrainz no-result and multi-result cases.
- Cover lookup success/failure.
- Rip without metadata.
- Rip with edited metadata, cover, genre, disc number.
- Confirm FLAC tags and embedded cover.
- Test missing `cdparanoia` and missing `flac`.
- Interrupt/failed rip and inspect partial files/warnings.

Video scanning:

- Scan folder with MKV/MP4/WEBM/MOV/M4V/AVI.
- Test missing `ffprobe` and missing `ffmpeg`.
- Verify thumbnails render in production build.
- Edit metadata and rescan; confirm edits persist.
- Delete/move video and rescan.

DVD import:

- Detect no DVD drive, unreadable drive, readable DVD.
- Scan `VIDEO_TS` folder.
- Import main title with default metadata.
- Import with edited metadata/output filename.
- Test encrypted/unreadable DVD and confirm unsupported/no-DRM-bypass message.
- Test missing `lsdvd` and missing `ffmpeg`.
- Confirm failed imports do not leave confusing partial output after cleanup policy is implemented.

mpv playback:

- Missing `mpv` error.
- Play DVD-imported MKV.
- Confirm music pauses and bottom player hides.
- Pause/resume/seek/volume.
- Bring to front/fullscreen.
- Close mpv window externally and verify progress saves.
- Stop video and verify music player returns.
- Restart app and resume from saved video position.

Missing files:

- Delete current audio file and attempt play.
- Delete current video file and attempt play.
- Delete cached cover/thumb files and verify graceful fallback.

App restart persistence:

- Library cache.
- Favorites/play counts.
- Playlists and missing playlist tracks.
- Genre assignments.
- Cached lyrics and offsets.
- Video metadata/progress.
- CD rip output files/tags.

## 8. Files/Modules Needing Attention

- `src/routes/+page.svelte`: too much state and behavior; split by feature/page.
- `src-tauri/src/lib.rs`: too many backend domains; split into modules before adding more features.
- `src/lib/types/library.ts`: central type file is acceptable now but will become unwieldy.
- `src/lib/api/library.ts`: clean command wrappers, but it covers too many domains.
- `src/lib/api/playback.ts`: keep command names and argument types synchronized with backend playback modules.
- `src/lib/components/TrackList.svelte`: reusable but semantically awkward due to row-as-button with nested controls.
- `src/lib/components/ContextMenu.svelte`: needs focus management and keyboard navigation.
- `src/lib/components/NowPlayingBar.svelte`: self-contained; avoid moving page-wide playback logic into it.
- `src-tauri/src/mpris.rs`: isolated well; keep it separate from general playback code.
- `src-tauri/tauri.conf.json`: CSP and asset protocol scope need attention.

## 9. Dependency/Tooling Notes

Required runtime/system tools or libraries:

- GStreamer runtime and plugins for audio codec support.
- `cdparanoia` for audio CD detection/ripping.
- `flac` for FLAC encoding.
- `libdiscid` for MusicBrainz Disc ID lookup.
- `ffmpeg` for DVD import and video thumbnails.
- `ffprobe` for video duration/codec metadata.
- `lsdvd` for DVD title scanning.
- `mpv` for Cassette-controlled native video playback.

Current handling:

- Missing `cdparanoia`, `flac`, `lsdvd`, `ffmpeg`, `ffprobe`, `libdiscid`, and `mpv` generally produce friendly errors.
- GStreamer plugin/codec failures should be tested more; playback errors currently surface as backend errors.
- MusicBrainz, Cover Art Archive, and LRCLIB are online dependencies. Network failures are handled, but caching/rate-limit behavior is minimal.

## 10. Do-Not-Change Warnings

- Do not change track/video IDs away from file paths without migrating dependent tables/cache files.
- Do not delete stale playlist entries automatically unless product behavior changes.
- Do not weaken the no-DRM-bypass behavior in DVD import.
- Do not replace mpv with webview playback as a cleanup shortcut.
- Do not force DVD MKV transcoding unless stream copy is proven unsafe for the target playback path.
- Do not make LRCLIB overwrite adjacent local `.lrc`/`.txt` files.
- Do not hold SQLite locks across network awaits.
- Do not add shell-based command execution; keep explicit `Command::new` args.
- Do not combine audio and video queues unless deliberately redesigning playback.

