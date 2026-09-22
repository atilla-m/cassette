import assert from "node:assert/strict";
import test from "node:test";
import { buildAlbums, buildArtists, buildGenres } from "../src/lib/data/libraryViews.ts";
import {
  nextStatsLimit,
  rankMostPlayedTracks,
  rankRecentlyPlayedTracks,
  sortAlbumsByPlayCount,
  sortArtistsByPlayCount,
  sortGenresByPlayCount,
  visibleStatsItems,
} from "../src/lib/utils/playStats.ts";

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

  assert.deepEqual(sortAlbumsByPlayCount(albums, "mostPlayed").map((album) => album.title), ["Alpha", "Beta", "Zulu"]);
  assert.deepEqual(sortAlbumsByPlayCount(albums, "leastPlayed").map((album) => album.title), ["Zulu", "Alpha", "Beta"]);
  assert.deepEqual(sortArtistsByPlayCount(artists, "mostPlayed").map((artist) => artist.name), ["Alpha", "Beta", "Zulu"]);
  assert.deepEqual(sortGenresByPlayCount(genres, "leastPlayed").map((genre) => genre.name), ["Zulu", "Alpha", "Beta"]);
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
