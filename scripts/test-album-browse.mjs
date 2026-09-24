import assert from "node:assert/strict";
import test from "node:test";
import { sortAlbumDisplayTracks } from "../src/lib/utils/albumTrackSort.ts";
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
  assert.deepEqual(ids("title", "desc"), ["c", "b", "a"]);
  assert.deepEqual(ids("artist"), ["a", "c", "b"]);
  assert.deepEqual(ids("duration"), ["c", "b", "a"]);
  assert.deepEqual(original.map(({ id }) => id), ["b", "c", "a"]);

  const withMissingNumber = [...original, track("missing", null, null, "No number", "One", 0, null)];
  assert.equal(sortAlbumDisplayTracks(withMissingNumber, "trackNumber", "desc").at(-1).id, "missing");
  assert.equal(sortAlbumDisplayTracks(withMissingNumber, "duration", "desc").at(-1).id, "missing");
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
