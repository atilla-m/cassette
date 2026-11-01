import assert from "node:assert/strict";
import test from "node:test";
import { createCdRipMetadataSnapshot, mergeCdLookupMetadata } from "../src/lib/utils/cdRipMetadata.ts";
import { ripCdToFlac } from "../src/lib/api/library.ts";

function metadata(title = "Track 02") {
  return {
    albumArtist: "", albumTitle: "", year: "", genre: "", discNumber: null, cover: null,
    tracks: [
      { number: 1, title: "Track 01", artist: "", discNumber: null },
      { number: 2, title, artist: "", discNumber: null },
    ],
  };
}

test("late lookup preserves manual titles and fills untouched fields by track number", () => {
  const current = metadata("Ark");
  current.tracks[1].artist = "Manual Artist";
  const incoming = metadata("Lookup Title");
  incoming.albumTitle = "Lookup Album";
  incoming.tracks.reverse();
  const merged = mergeCdLookupMetadata(current, incoming, new Set(["track.2.title", "track.2.artist"]));
  assert.equal(merged.albumTitle, "Lookup Album");
  assert.equal(merged.tracks[0].number, 2);
  assert.equal(merged.tracks[0].title, "Ark");
  assert.equal(merged.tracks[0].artist, "Manual Artist");
  assert.equal(merged.tracks[1].title, "Track 01");
});

test("explicitly blank or reverted edits are still manual choices", () => {
  for (const title of ["", "   ", "Track 02"]) {
    const merged = mergeCdLookupMetadata(metadata(title), metadata("Lookup Title"), new Set(["track.2.title"]));
    assert.equal(merged.tracks[1].title, title);
    assert.equal(createCdRipMetadataSnapshot(merged).tracks[1].title, "Track 02");
  }
});

test("manual album fields and disc numbers survive automatic lookup", () => {
  const current = metadata("手動の曲名");
  Object.assign(current, { albumArtist: "Manual Artist", albumTitle: "Manual Album", year: "2026", genre: "Manual Genre", discNumber: 3 });
  current.tracks.forEach((track) => track.discNumber = 3);
  const incoming = metadata("Lookup Title");
  incoming.discNumber = 1;
  const fields = new Set(["albumArtist", "albumTitle", "year", "genre", "discNumber", "track.2.title"]);
  const merged = mergeCdLookupMetadata(current, incoming, fields);
  for (const key of ["albumArtist", "albumTitle", "year", "genre", "discNumber"]) assert.equal(merged[key], current[key]);
  assert.deepEqual(merged.tracks.map((track) => track.discNumber), [3, 3]);
  assert.equal(merged.tracks[1].title, "手動の曲名");
});

test("snapshot owns tracks and cover, normalizes blank titles, and leaves Unicode/path text for the tag writer", () => {
  const draft = metadata("  ../夜/道\\彼方..  ");
  draft.cover = { source: "manual", path: "/synthetic/cover.png", mimeType: "image/png", extension: "png" };
  draft.tracks[0].title = " \t ";
  const snapshot = createCdRipMetadataSnapshot(draft);
  assert.equal(snapshot.tracks[0].title, "Track 01");
  assert.equal(snapshot.tracks[1].title, "../夜/道\\彼方..");
  draft.tracks[1].title = "Later edit";
  draft.cover.path = "/synthetic/later.png";
  draft.tracks.push({ number: 3, title: "Later track", artist: "", discNumber: null });
  assert.equal(snapshot.tracks.length, 2);
  assert.equal(snapshot.tracks[1].title, "../夜/道\\彼方..");
  assert.equal(snapshot.cover.path, "/synthetic/cover.png");
});

test("real frontend invoke sends the submitted title snapshot", async () => {
  const previousWindow = globalThis.window;
  const calls = [];
  globalThis.window = { __TAURI_INTERNALS__: { invoke: async (command, args) => {
    calls.push({ command, args: structuredClone(args) });
    return { outputFolder: "/synthetic/rip", tracks: [] };
  } } };
  try {
    const draft = metadata("Ark");
    await ripCdToFlac("/synthetic", createCdRipMetadataSnapshot(draft));
    draft.tracks[1].title = "Later edit";
    assert.equal(calls[0].command, "rip_cd_to_flac");
    assert.equal(calls[0].args.outputFolder, "/synthetic");
    assert.equal(calls[0].args.metadata.tracks[1].title, "Ark");
  } finally {
    if (previousWindow === undefined) delete globalThis.window;
    else globalThis.window = previousWindow;
  }
});
