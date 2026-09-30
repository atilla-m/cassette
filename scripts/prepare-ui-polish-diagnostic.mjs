import { copyFileSync, cpSync, existsSync, lstatSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Reuse the disposable, path-relocated listening fixture. It refuses existing
// destinations and copies the freshly built native binary; no installed app is touched.
const repository = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = process.argv[2];
const runtime = process.argv[3];
if (!root || !isAbsolute(root)) throw new Error("Pass a new absolute diagnostic directory.");
if (runtime && (!isAbsolute(runtime) || !existsSync(join(runtime, "AppRun")) || !existsSync(join(runtime, "usr/bin/cassette")) || !lstatSync(join(runtime, "usr/bin/cassette")).isFile())) throw new Error("Optional runtime must be an existing extracted Cassette AppDir with a regular executable.");
const generated = spawnSync(process.execPath, [join(repository, "scripts/prepare-detailed-stats-diagnostic.mjs"), root, "--browse-group"], { encoding: "utf8" });
if (generated.status !== 0) throw new Error(generated.stderr || generated.stdout || "Could not prepare disposable profile.");

const colors = ["#268e86", "#647bbd", "#9a604b", "#a47a39", "#8855a8"];
const quote = (value) => `'${value.replaceAll("'", "''")}'`;
for (const profile of ["source", "destination"]) {
  // Artwork uses the same native asset-protocol scope as real cached covers.
  const artworkDirectory = join(root, profile, "data/io.github.atilla.cassette/cover-art");
  mkdirSync(artworkDirectory);
  const paths = colors.map((color, index) => {
    const path = join(artworkDirectory, `synthetic-cover-${index + 1}.svg`);
    writeFileSync(path, `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 600"><rect width="600" height="600" fill="#111720"/><circle cx="300" cy="300" r="230" fill="${color}"/><text x="300" y="330" text-anchor="middle" fill="white" font-family="sans-serif" font-size="80" font-weight="bold">TEST ${index + 1}</text></svg>`, { flag: "wx" });
    return path;
  });
  const database = join(root, profile, "data/io.github.atilla.cassette/library.sqlite3");
  const sql = `UPDATE tracks SET cover_art_path = CASE ((track_number - 1) % 5) ${paths.map((path, index) => `WHEN ${index} THEN ${quote(path)}`).join(" ")} END; UPDATE tracks SET is_favorite = 1 WHERE track_number <= 3;`;
  const update = spawnSync("sqlite3", [database, sql], { encoding: "utf8" });
  if (update.status !== 0) throw new Error(update.stderr || `Could not add artwork to ${profile}.`);
}
const expectedPath = join(root, "EXPECTED.json");
const expected = JSON.parse(readFileSync(expectedPath, "utf8"));
writeFileSync(expectedPath, `${JSON.stringify({ ...expected, favorites: 3, syntheticArtwork: colors.length }, null, 2)}\n`);
if (runtime) {
  const appdir = join(root, "runtime-appdir");
  cpSync(runtime, appdir, { recursive: true, verbatimSymlinks: true });
  copyFileSync(join(root, "cassette-diagnostic"), join(appdir, "usr/bin/cassette"));
  writeFileSync(join(root, "launch.sh"), `#!/usr/bin/env bash
set -euo pipefail
test_root="$(cd -- "$(dirname -- "\${BASH_SOURCE[0]}")" && pwd)"
profile="\${1:-source}"
case "$profile" in source|destination) ;; *) echo 'Use source or destination' >&2; exit 2 ;; esac
exec env -u APPIMAGE APPDIR="$test_root/runtime-appdir" \\
  XDG_DATA_HOME="$test_root/$profile/data" XDG_CONFIG_HOME="$test_root/$profile/config" \\
  XDG_CACHE_HOME="$test_root/$profile/cache" XDG_STATE_HOME="$test_root/$profile/state" \\
  "$test_root/runtime-appdir/AppRun"
`, { mode: 0o755 });
}
writeFileSync(join(root, "MANUAL.md"), `# Isolated UI-polish diagnostic

Run: ${join(root, "launch.sh")} source

The persistent source and destination profiles contain disposable synthetic media at different paths. The source starts with populated listening history, three favorites, artwork, lyrics and a custom playlist. Test playback can add plays normally. This is a diagnostic binary, not an installed or signed release.

1. Play a song, then switch between browsing, Settings, Stats and Lyrics at 1920×1080 and 1280×720. Check the shared bar, metadata links, queue and seek position; blank bar space must not open Lyrics.
2. Visit every Settings section, try the three retained themes and visibility preferences, then leave and return. The selected section should remain selected.
3. Sort Songs and album/artist/genre lists. Field and direction changes stay open; Escape, Close and outside click dismiss. Check Songs search/sort persistence, list positions and the unchanged current queue.
4. Check a same-day custom Stats range and a reversed range. Expand the counting explanation; library totals are separate from period totals.
5. Export from Settings → Listening history, close the source, and run ${join(root, "launch.sh")} destination. Import that JSON, check the completion result, change sections and return; repeat the import to see the no-new-history result. Cancel and invalid-file attempts must not show new success.
6. Check Liked Songs/custom playlist actions and Mix Builder artwork, selection summary and Start Mix while scrolling.

Do not select real music or use updater/applications-menu actions during this diagnostic. User-confirmed manual results are still pending.
`);
console.log(`Ready: ${join(root, "launch.sh")} source`);
