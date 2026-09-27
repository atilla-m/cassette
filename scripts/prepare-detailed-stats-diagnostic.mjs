import { chmodSync, copyFileSync, existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// This prepares disposable runtime data, never an installed application or release.
const repository = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = process.argv[2];
if (!root || !isAbsolute(root) || resolve(root) === "/" || existsSync(root)) {
  throw new Error("Choose a new, absolute diagnostic directory. Existing destinations are never overwritten.");
}
const binary = join(repository, "src-tauri/target/release/cassette");
if (!existsSync(binary)) throw new Error("First build: npm run tauri -- build --no-bundle");
if (spawnSync("sqlite3", ["--version"], { encoding: "utf8" }).status !== 0) {
  throw new Error("The diagnostic generator needs the existing sqlite3 command.");
}
mkdirSync(root);
copyFileSync(binary, join(root, "cassette-diagnostic"));
chmodSync(join(root, "cassette-diagnostic"), 0o755);

const now = new Date();
const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
const dated = {
  today: Math.floor(new Date(today.getFullYear(), today.getMonth(), today.getDate(), 0, 1).getTime() / 1000),
  yesterday: Math.floor(new Date(today.getFullYear(), today.getMonth(), today.getDate() - 1, 12).getTime() / 1000),
  previousMonth: Math.floor(new Date(today.getFullYear(), today.getMonth() - 1, 15, 12).getTime() / 1000),
  previousYear: Math.floor(new Date(today.getFullYear() - 1, 5, 15, 12).getTime() / 1000),
};
const quote = (value) => `'${String(value).replaceAll("'", "''")}'`;
const epoch = (date) => Math.floor(date.getTime() / 1000);
const pad = (value) => String(value).padStart(3, "0");
const synchsafe = (value) => Buffer.from([(value >>> 21) & 127, (value >>> 14) & 127, (value >>> 7) & 127, value & 127]);

function wav(index, tags) {
  const rate = 16000;
  const seconds = 12;
  const pcm = Buffer.alloc(rate * seconds * 2);
  for (let sample = 0; sample < rate * seconds; sample++) {
    const envelope = Math.min(1, sample / 160, (rate * seconds - sample) / 160);
    pcm.writeInt16LE(Math.round(1800 * envelope * Math.sin(2 * Math.PI * (180 + index * 3) * sample / rate)), sample * 2);
  }
  const frames = Object.entries(tags).map(([id, value]) => {
    const text = Buffer.concat([Buffer.from([3]), Buffer.from(value, "utf8")]);
    return Buffer.concat([Buffer.from(id), synchsafe(text.length), Buffer.alloc(2), text]);
  });
  const body = Buffer.concat(frames);
  const id3 = Buffer.concat([Buffer.from("ID3"), Buffer.from([4, 0, 0]), synchsafe(body.length), body]);
  const header = Buffer.alloc(44);
  header.write("RIFF");
  header.writeUInt32LE(36 + pcm.length + 8 + id3.length + (id3.length % 2), 4);
  header.write("WAVEfmt ", 8);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20);
  header.writeUInt16LE(1, 22);
  header.writeUInt32LE(rate, 24);
  header.writeUInt32LE(rate * 2, 28);
  header.writeUInt16LE(2, 32);
  header.writeUInt16LE(16, 34);
  header.write("data", 36);
  header.writeUInt32LE(pcm.length, 40);
  const chunk = Buffer.alloc(8);
  chunk.write("id3 ");
  chunk.writeUInt32LE(id3.length, 4);
  return Buffer.concat([header, pcm, chunk, id3, Buffer.alloc(id3.length % 2)]);
}

const schema = `
CREATE TABLE tracks (
 id TEXT PRIMARY KEY NOT NULL, title TEXT NOT NULL, artist TEXT, album TEXT, album_artist TEXT,
 genres TEXT NOT NULL DEFAULT '[]', track_number INTEGER, disc_number INTEGER, year INTEGER,
 duration_seconds INTEGER, file_path TEXT NOT NULL, file_name TEXT NOT NULL, extension TEXT NOT NULL,
 modified_time INTEGER, file_size INTEGER, scanned_at INTEGER NOT NULL, cover_art_path TEXT,
 lyrics_path TEXT, lyrics_kind TEXT, is_favorite INTEGER NOT NULL DEFAULT 0,
 play_count INTEGER NOT NULL DEFAULT 0, last_played_at INTEGER);
CREATE TABLE library_meta (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
CREATE TABLE track_play_events (event_id TEXT PRIMARY KEY NOT NULL, track_id TEXT NOT NULL,
 played_at_utc INTEGER NOT NULL, recorded_at_utc INTEGER NOT NULL,
 source TEXT NOT NULL CHECK(source IN ('qualified_play', 'legacy_last_played')));
CREATE TABLE playlists (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE playlist_tracks (playlist_id TEXT NOT NULL, track_id TEXT NOT NULL, position INTEGER NOT NULL,
 added_at INTEGER NOT NULL, PRIMARY KEY (playlist_id, track_id), FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE);
`;
let datedEvents = 0;
let undatedPlays = 0;
for (const profile of ["source", "destination"]) {
  const media = join(root, profile === "source" ? "source-music" : "relocated-music");
  const profileRoot = join(root, profile);
  for (const directory of [media, ...["data", "config", "cache", "state"].map((name) => join(profileRoot, name))]) mkdirSync(directory, { recursive: true });
  const data = join(profileRoot, "data/io.github.atilla.cassette");
  mkdirSync(data, { recursive: true });
  const statements = ["BEGIN;", schema,
    `INSERT INTO library_meta VALUES ('last_scanned_folder', ${quote(media)}), ('last_scanned_at', ${quote(epoch(now))}), ('detailed_play_history_started_at_utc', ${quote(epoch(new Date(today.getFullYear(), today.getMonth(), today.getDate() - 7)))});`,
  ];
  for (let index = 1; index <= 120; index++) {
    const fileName = `${profile === "source" ? "track" : "moved"}-${pad(index)}.wav`;
    const path = join(media, fileName);
    const title = `Synthetic Listening Track ${pad(index)} — 音`;
    const artist = `Synthetic Artist ${pad((index - 1) % 64 + 1)}`;
    const album = `Synthetic Album ${pad((index - 1) % 80 + 1)}`;
    const albumArtist = "Cassette Synthetic Ensemble";
    const genre = `Synthetic Genre ${pad((index - 1) % 60 + 1)}`;
    writeFileSync(path, wav(index, { TIT2: title, TPE1: artist, TALB: album, TPE2: albumArtist, TCON: genre, TRCK: String(index), TDRC: "2026" }), { flag: "wx" });
    const lyrics = path.replace(/\.wav$/, ".lrc");
    writeFileSync(lyrics, "[00:00.00]Disposable listening-statistics fixture\n[00:06.00]This play should increment exactly once\n", { flag: "wx" });
    const events = [];
    if (profile === "source") {
      if (index <= 100) events.push([dated.today, "qualified_play"]);
      if (index % 2 === 0) events.push([dated.yesterday, "qualified_play"]);
      events.push([dated.previousMonth, "legacy_last_played"]);
      if (index % 5 === 0) events.push([dated.previousYear, "legacy_last_played"]);
      datedEvents += events.length;
      undatedPlays += index % 4;
    }
    const plays = events.length + (profile === "source" ? index % 4 : 0);
    const modified = statSync(path);
    statements.push(`INSERT INTO tracks VALUES (${quote(path)}, ${quote(title)}, ${quote(artist)}, ${quote(album)}, ${quote(albumArtist)}, ${quote(JSON.stringify([genre]))}, ${index}, 1, 2026, 12, ${quote(path)}, ${quote(fileName)}, 'wav', ${Math.floor(modified.mtimeMs / 1000)}, ${modified.size}, ${epoch(now)}, NULL, ${quote(lyrics)}, 'synced', ${index === 1 ? 1 : 0}, ${plays}, ${events.length ? Math.max(...events.map(([date]) => date)) : "NULL"});`);
    events.forEach(([timestamp, source], eventIndex) => statements.push(`INSERT INTO track_play_events VALUES ('diagnostic-event-${pad(index)}-${eventIndex}', ${quote(path)}, ${timestamp}, ${timestamp}, '${source}');`));
    if (index <= 3) statements.push(`INSERT INTO playlist_tracks VALUES ('diagnostic-playlist', ${quote(path)}, ${index - 1}, ${epoch(now)});`);
  }
  statements.push(`INSERT INTO playlists VALUES ('diagnostic-playlist', 'Disposable diagnostic playlist', ${epoch(now)}, ${epoch(now)});`, "COMMIT;");
  const result = spawnSync("sqlite3", [join(data, "library.sqlite3")], { input: statements.join("\n"), encoding: "utf8" });
  if (result.status !== 0) throw new Error(result.stderr || "Could not seed isolated database.");
}
writeFileSync(join(root, "EXPECTED.json"), JSON.stringify({
  generatedAt: now.toISOString(), timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  tracks: 120, artists: 64, albums: 80, genres: 60,
  source: { allTime: datedEvents + undatedPlays, datedEvents, undatedPlays, today: 100, yesterday: 60, previousMonth: 120, previousYear: 24 },
  destinationBeforeImport: { allTime: 0 }, dates: dated,
}, null, 2) + "\n", { flag: "wx" });
writeFileSync(join(root, "launch.sh"), `#!/usr/bin/env bash
set -euo pipefail
test_root="$(cd -- "$(dirname -- "\${BASH_SOURCE[0]}")" && pwd)"
profile="\${1:-source}"
case "$profile" in source|destination) ;; *) echo 'Use source or destination' >&2; exit 2 ;; esac
exec env -u APPIMAGE -u APPDIR \\
  XDG_DATA_HOME="$test_root/$profile/data" \\
  XDG_CONFIG_HOME="$test_root/$profile/config" \\
  XDG_CACHE_HOME="$test_root/$profile/cache" \\
  XDG_STATE_HOME="$test_root/$profile/state" \\
  "$test_root/cassette-diagnostic"
`, { flag: "wx", mode: 0o755 });
writeFileSync(join(root, "MANUAL.md"), `# Disposable detailed-stats diagnostic

Run ./launch.sh source (or omit source), then close it and run ./launch.sh destination.
Both use persistent isolated XDG data. No music import is necessary: 120 generated WAVs and LRCs are already in each library.
The destination uses different folder paths and filenames but exactly the same encoded audio.

Initial source All time: ${datedEvents + undatedPlays} plays, 120 tracks, 64 artists; Today: 100 plays.
Initial destination: zero plays. EXPECTED.json records all dates and totals.
Playing a full 12-second tone adds one play under the existing qualification rule.

1. Source → Stats: periods, chart, exact values, View all/load more. Export while Today is selected to a NEW filename here (for example history.json).
2. Close Source. Launch Destination, import that file, cancel the preview first: zero plays remain.
3. Preview/import again: 120 matches; All time should equal Source. Restore all dated periods too.
4. Reimport: zero new events. Play a tone, then reimport the older backup: the new play must remain. Reopen to check persistence.
5. Check keyboard operation and refresh after a play. Do not mark these manual checks passed until you report them.

This is a local native diagnostic, not the installed or published beta.4 AppImage. It has no configured development updater activation.
`, { flag: "wx" });
console.log(`Prepared diagnostic: ${join(root, "launch.sh")}`);
