import assert from "node:assert/strict";
import test from "node:test";
import { buildAlbums, buildArtists, buildGenres } from "../src/lib/data/libraryViews.ts";
import { listeningTotalLabel } from "../src/lib/utils/listeningStats.ts";
import { ListeningTimeBuffer, PlaybackListenProgress } from "../src/lib/utils/playbackListenProgress.ts";
import {
  nextStatsLimit,
  rankMostPlayedTracks,
  rankRecentlyPlayedTracks,
  sortAlbumsByActivity,
  sortArtistsByActivity,
  sortGenresByActivity,
  visibleStatsItems,
} from "../src/lib/utils/playStats.ts";

test("half of a two-minute track qualifies only after 60 seconds of media have played", () => {
  const progress = new PlaybackListenProgress();
  progress.reset(0);
  progress.observe(0, 90_000, true);
  assert.equal(progress.qualifies(120, 50, false), false);
  progress.observe(59.99, 149_990, true);
  assert.equal(progress.qualifies(120, 50, false), false);
  progress.observe(60, 150_000, true);
  assert.equal(progress.qualifies(120, 50, false), true);
  assert.equal(progress.qualifies(120, 75, false), false);
});

test("seeks, pauses, and an unsettled media clock add no listening time", () => {
  const progress = new PlaybackListenProgress();
  progress.reset(0);
  progress.observe(20, 20_000, true);
  progress.forgetPosition();
  progress.observe(80, 20_100, true);
  progress.observe(90, 30_100, true);
  progress.observe(90, 75_100, false);
  progress.forgetPosition();
  progress.observe(21, 75_200, true);
  progress.observe(91, 75_450, true);
  assert.equal(progress.qualifies(120, 50, false), false);
  progress.observe(120, 104_450, true);
  assert.equal(progress.qualifies(120, 50, true), false);
  progress.forgetPosition();
  progress.observe(0, 105_000, true);
  progress.observe(1, 106_000, true);
  assert.equal(progress.qualifies(120, 50, false), true);
});

test("an end after seeking to the tail does not qualify, and a new track starts fresh", () => {
  const progress = new PlaybackListenProgress();
  progress.reset(0);
  progress.observe(0, 0, true);
  progress.forgetPosition();
  progress.observe(119, 300, true);
  progress.observe(120, 1300, true);
  assert.equal(progress.qualifies(120, 50, true), false);
  progress.reset(1300);
  progress.observe(2, 3300, true);
  assert.equal(progress.qualifies(10, 25, false), false);
  assert.equal(progress.qualifies(null, 50, false), false);
  assert.equal(progress.qualifies(null, 50, true), false);
  progress.observe(5, 6300, true);
  assert.equal(progress.qualifies(null, 50, true), true);
});

test("a play needs five seconds of actual audio as well as the configured threshold", () => {
  const progress = new PlaybackListenProgress();
  progress.reset(0);
  progress.observe(4.999, 4999, true);
  assert.equal(progress.qualifies(6, 50, false), false);
  progress.observe(5, 5000, true);
  assert.equal(progress.qualifies(6, 50, false), true);
  assert.equal(progress.qualifies(12, 50, false), false);
});

test("a long track retains time when its position updates less often than the player polls", () => {
  const progress = new PlaybackListenProgress();
  const buffer = new ListeningTimeBuffer();
  progress.reset(0);
  buffer.start("long-track", "long-session");

  for (let ms = 250; ms <= 488_000; ms += 250) {
    const position = ms <= 21_000 ? ms / 1000 : 21 + Math.floor((ms - 21_000) / 1000);
    buffer.observe(progress.observe(position, ms, true), 60_000 + ms);
    if (ms === 243_750) assert.equal(progress.qualifies(488, 50, false), false);
    if (ms === 244_000) assert.equal(progress.qualifies(488, 50, false), true);
  }

  assert.equal(buffer.batch().reduce((total, minute) => total + minute.listenedMs, 0), 488_000);
  assert.equal(progress.qualifies(488, 100, true), true);
});

test("late replies and transient zero or jumped positions retain the real progress", () => {
  const progress = new PlaybackListenProgress();
  progress.reset(0);
  assert.equal(progress.observe(1, 1500, true, 900), 1);
  assert.equal(progress.observe(1.75, 1700, true, 1650), 0.75);
  assert.equal(progress.observe(0, 1800, true), 0);
  assert.equal(progress.observe(400, 1900, true), 0);
  assert.equal(progress.observe(2, 2100, true), 0.25);
});

test("old plays without timing have an unavailable duration", () => {
  assert.equal(listeningTotalLabel(0, 6), "Time unavailable");
  assert.equal(listeningTotalLabel(0, 0), "0s");
  assert.equal(listeningTotalLabel(488_000, 1), "8m 8s");
});

test("measured audio is split across a minute boundary and in-flight samples retain later updates", () => {
  const progress = new PlaybackListenProgress();
  const buffer = new ListeningTimeBuffer();
  progress.reset(0);
  buffer.start("a", "fixture-session-a");
  buffer.observe(progress.observe(1.5, 1500, true), 60_500);
  const firstWrite = buffer.batch();
  assert.deepEqual(firstWrite.map(({ minuteUtc, listenedMs }) => [minuteUtc, listenedMs]), [[0, 1000], [60, 500]]);
  buffer.observe(progress.observe(50, 1750, true), 60_750);
  buffer.observe(progress.observe(50, 9000, false), 68_000);
  buffer.observe(progress.observe(50.5, 9500, true), 68_500);
  buffer.saved(firstWrite);
  assert.deepEqual(buffer.batch().map(({ minuteUtc, listenedMs }) => [minuteUtc, listenedMs]), [[60, 1000]]);
  buffer.start("b", "fixture-session-b");
  buffer.observe(0.7, 69_000);
  assert.deepEqual(buffer.batch().map(({ trackId, listenedMs }) => [trackId, listenedMs]), [["a", 1000], ["b", 700]]);
  assert.deepEqual(buffer.batch("fixture-session-a").map(({ trackId, listenedMs }) => [trackId, listenedMs]), [["a", 1000]]);
});

function track(overrides) {
  const id = overrides.id;
  return {
    id,
    filePath: `/synthetic/${id}.flac`,
    fileName: `${id}.flac`,
    extension: "flac",
    title: id,
    artist: "Artist A",
    album: "Album A",
    albumArtist: "Artist A",
    genres: ["Genre A"],
    trackNumber: 1,
    discNumber: 1,
    year: 2026,
    durationSeconds: 60,
    modifiedTime: null,
    fileSize: null,
    scannedAt: null,
    coverArtPath: null,
    lyricsPath: null,
    lyricsKind: null,
    isFavorite: false,
    playCount: 0,
    lastPlayedAt: null,
    ...overrides,
  };
}

test("aggregates legacy all-time totals with established album, artist, and genre grouping", () => {
  const tracks = [
    track({ id: "legacy", playCount: 11, lastPlayedAt: null, genres: ["Genre A", "Shared"] }),
    track({ id: "zero", title: "Zero", playCount: 0, genres: ["Genre A"] }),
    track({ id: "case", album: "album a", albumArtist: "artist a", playCount: 4, genres: ["Shared"] }),
    track({ id: "other", artist: "Artist B", albumArtist: "Artist B", album: "Album A", playCount: 7, genres: [] }),
  ];

  const albums = buildAlbums(tracks);
  const artists = buildArtists(tracks);
  const genres = buildGenres(tracks);

  assert.deepEqual(albums.map(({ artist, title, trackCount, playCount }) => ({ artist, title, trackCount, playCount })), [
    { artist: "Artist A", title: "Album A", trackCount: 3, playCount: 15 },
    { artist: "Artist B", title: "Album A", trackCount: 1, playCount: 7 },
  ]);
  assert.deepEqual(artists.map(({ name, playCount }) => ({ name, playCount })), [
    { name: "Artist A", playCount: 15 },
    { name: "Artist B", playCount: 7 },
  ]);
  assert.deepEqual(genres.map(({ name, playCount }) => ({ name, playCount })), [
    { name: "Genre A", playCount: 11 },
    { name: "Shared", playCount: 15 },
    { name: "Unknown Genre", playCount: 7 },
  ]);
});

test("most and least played browse sorting includes zeroes and breaks ties deterministically", () => {
  const albums = buildAlbums([
    track({ id: "zulu", title: "Zulu", album: "Zulu", playCount: 0 }),
    track({ id: "beta", title: "Beta", album: "Beta", playCount: 3 }),
    track({ id: "alpha", title: "Alpha", album: "Alpha", playCount: 3 }),
  ]);
  const artists = buildArtists([
    track({ id: "artist-z", artist: "Zulu", albumArtist: "Zulu", playCount: 0 }),
    track({ id: "artist-b", artist: "Beta", albumArtist: "Beta", playCount: 3 }),
    track({ id: "artist-a", artist: "Alpha", albumArtist: "Alpha", playCount: 3 }),
  ]);
  const genres = buildGenres([
    track({ id: "genre-z", genres: ["Zulu"], playCount: 0 }),
    track({ id: "genre-b", genres: ["Beta"], playCount: 3 }),
    track({ id: "genre-a", genres: ["Alpha"], playCount: 3 }),
  ]);

  assert.deepEqual(sortAlbumsByActivity(albums, "mostPlayed").map((album) => album.title), ["Alpha", "Beta", "Zulu"]);
  assert.deepEqual(sortAlbumsByActivity(albums, "leastPlayed").map((album) => album.title), ["Zulu", "Alpha", "Beta"]);
  assert.deepEqual(sortArtistsByActivity(artists, "mostPlayed").map((artist) => artist.name), ["Alpha", "Beta", "Zulu"]);
  assert.deepEqual(sortGenresByActivity(genres, "leastPlayed").map((genre) => genre.name), ["Zulu", "Alpha", "Beta"]);
});

test("library activity sorts keep play counts and listening time independent", () => {
  const tracks = [
    track({ album: "Alpha", artist: "Alpha", albumArtist: "Alpha", genres: ["Alpha"], playCount: 9 }),
    track({ album: "Beta", artist: "Beta", albumArtist: "Beta", genres: ["Beta"], playCount: 1, listenedMs: 6000 }),
    track({ album: "Beta", artist: "Beta", albumArtist: "Beta", genres: ["Beta"], playCount: 0, listenedMs: 5000 }),
  ];

  for (const [build, sort] of [
    [buildAlbums, sortAlbumsByActivity],
    [buildArtists, sortArtistsByActivity],
    [buildGenres, sortGenresByActivity],
  ]) {
    const items = build(tracks);
    assert.deepEqual(sort(items, "mostPlayed").map(({ playCount, listenedMs }) => [playCount, listenedMs]), [[9, 0], [1, 11000]]);
    assert.deepEqual(sort(items, "mostListened").map(({ playCount }) => playCount), [1, 9]);
    assert.deepEqual(sort(items, "leastListened").map(({ playCount }) => playCount), [9, 1]);
  }
});

test("stats ranking preserves unique-track recently played semantics", () => {
  const tracks = [
    track({ id: "legacy", playCount: 8, lastPlayedAt: null }),
    track({ id: "older", playCount: 2, lastPlayedAt: 100 }),
    track({ id: "newer", playCount: 2, lastPlayedAt: 200 }),
    track({ id: "zero", playCount: 0, lastPlayedAt: null }),
  ];

  assert.deepEqual(rankMostPlayedTracks(tracks).map((item) => item.id), ["legacy", "newer", "older"]);
  assert.deepEqual(rankRecentlyPlayedTracks(tracks).map((item) => item.id), ["newer", "older"]);
});

test("incremental Stats loading reaches every entry beyond the old limits", () => {
  const items = Array.from({ length: 137 }, (_, index) => index);
  assert.deepEqual(visibleStatsItems(items, false, 50, 10), items.slice(0, 10));

  let limit = 50;
  assert.deepEqual(visibleStatsItems(items, true, limit, 10), items.slice(0, 50));
  limit = nextStatsLimit(limit, items.length);
  assert.equal(limit, 100);
  limit = nextStatsLimit(limit, items.length);
  assert.equal(limit, 137);
  assert.deepEqual(visibleStatsItems(items, true, limit, 10), items);
});
