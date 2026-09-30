import assert from "node:assert/strict";
import test from "node:test";
import { sortMenuPlacement } from "../src/lib/utils/sortMenuPlacement.ts";

test("sort menus fit the content area above the player bar", () => {
  assert.deepEqual(sortMenuPlacement(163, 197, 0, 634, 7), { opensUp: false, availableHeight: 370 });
  assert.deepEqual(sortMenuPlacement(550, 584, 0, 634, 5), { opensUp: true, availableHeight: 370 });
  assert.deepEqual(sortMenuPlacement(130, 164, 0, 280, 7), { opensUp: true, availableHeight: 118 });
});
import { sortAlbumDisplayTracks, topPlayedTracks } from "../src/lib/utils/albumTrackSort.ts";
import { stepView, visitView } from "../src/lib/utils/viewHistory.ts";

const track = (id, discNumber, trackNumber, title, artist, playCount, durationSeconds) => ({
  id, filePath: `/synthetic/${id}.flac`, discNumber, trackNumber, title, artist, playCount, durationSeconds,
});

test("album sorting is display-only, deterministic, and includes zero plays", () => {
  const original = [
    track("b", 1, 2, "Beta", "Two", 0, 12),
    track("c", 2, 1, "Gamma", "One", 4, 9),
    track("a", 1, 1, "Alpha", "One", 4, 15),
  ];
  const ids = (key, direction = "asc") => sortAlbumDisplayTracks(original, key, direction).map(({ id }) => id);

  assert.deepEqual(ids("trackNumber"), ["a", "b", "c"]);
  assert.deepEqual(ids("trackNumber", "desc"), ["c", "b", "a"]);
  assert.deepEqual(ids("mostPlayed"), ["a", "c", "b"]);
  assert.deepEqual(ids("leastPlayed"), ["b", "a", "c"]);
  assert.deepEqual(ids("playCount", "desc"), ids("mostPlayed"));
  assert.deepEqual(ids("playCount", "asc"), ids("leastPlayed"));
  assert.deepEqual(ids("title", "desc"), ["c", "b", "a"]);
  assert.deepEqual(ids("artist"), ["a", "c", "b"]);
  const acrossAlbums = [
    { ...original[0], album: "Zulu" },
    { ...original[1], album: "Alpha" },
    { ...original[2], album: "Alpha" },
  ];
  assert.deepEqual(sortAlbumDisplayTracks(acrossAlbums, "album", "asc").map(({ id }) => id), ["a", "c", "b"]);
  assert.deepEqual(sortAlbumDisplayTracks(acrossAlbums, "album", "desc").map(({ id }) => id), ["b", "a", "c"]);
  const withAlbumArtistFallback = [
    { ...original[0], artist: null, albumArtist: "Zulu" },
    { ...original[1], artist: null, albumArtist: "Alpha" },
  ];
  assert.deepEqual(sortAlbumDisplayTracks(withAlbumArtistFallback, "artist", "asc").map(({ id }) => id), ["c", "b"]);
  assert.deepEqual(ids("duration"), ["c", "b", "a"]);
  assert.deepEqual(original.map(({ id }) => id), ["b", "c", "a"]);

  const withMissingNumber = [...original, track("missing", null, null, "No number", "One", 0, null)];
  assert.equal(sortAlbumDisplayTracks(withMissingNumber, "trackNumber", "desc").at(-1).id, "missing");
  assert.equal(sortAlbumDisplayTracks(withMissingNumber, "duration", "desc").at(-1).id, "missing");
});

test("artist and genre previews cap at ten most-played songs with deterministic zero-play ties", () => {
  const ranked = Array.from({ length: 13 }, (_, index) =>
    track(`rank-${index + 1}`, 1, index + 1, `Song ${index + 1}`, "Artist", 12 - index, 10));
  assert.deepEqual(topPlayedTracks([...ranked].reverse()).map(({ id }) => id),
    ranked.slice(0, 10).map(({ id }) => id));
  assert.equal(topPlayedTracks(ranked, ranked.length).length, 13);

  const unplayed = ranked.map((entry) => ({ ...entry, playCount: 0 }));
  assert.deepEqual(topPlayedTracks([...unplayed].reverse()).map(({ id }) => id),
    unplayed.slice(0, 10).map(({ id }) => id));
});

test("view history moves backward and forward and branches after a new visit", () => {
  const equals = (left, right) => left === right;
  let history = { entries: ["Albums"], index: 0 };
  history = visitView(history, "Album detail", equals);
  history = visitView(history, "Artist detail", equals);
  history = stepView(history, -1);
  assert.equal(history.entries[history.index], "Album detail");
  history = stepView(history, 1);
  assert.equal(history.entries[history.index], "Artist detail");
  history = stepView(history, -1);
  history = visitView(history, "Genre detail", equals);
  assert.deepEqual(history.entries, ["Albums", "Album detail", "Genre detail"]);
  assert.equal(stepView(history, 1), history);
  assert.equal(visitView(history, "Genre detail", equals), history);
});
