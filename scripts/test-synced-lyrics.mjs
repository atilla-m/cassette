import assert from "node:assert/strict";
import test from "node:test";
import {
  parseLrcLine,
  parseLrcLyrics,
  hasSyncedLyricsIntro,
  resolveSyncedLyricsState,
  startsSyncedLyricBreak,
} from "../src/lib/utils/syncedLyrics.ts";

test("preserves timed blank lines and uses them as bounded break cues", () => {
  const cues = parseLrcLyrics([
    "[00:01.00]First line",
    "[00:04.00]",
    "[00:08.00]Second line",
  ].join("\n"));

  assert.deepEqual(cues.map(({ kind, timeSeconds, breakSource }) => ({ kind, timeSeconds, breakSource })), [
    { kind: "lyric", timeSeconds: 1, breakSource: null },
    { kind: "break", timeSeconds: 4, breakSource: "blank" },
    { kind: "lyric", timeSeconds: 8, breakSource: null },
  ]);
  assert.deepEqual(resolveSyncedLyricsState(cues, 3), { kind: "lyric", activeCueIndex: 0 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 5), { kind: "break", activeCueIndex: 1 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 8), { kind: "lyric", activeCueIndex: 2 });
});

test("recognizes conservative explicit instrumental markers", () => {
  const markers = ["[Instrumental]", "(instrumental break)", "{Music break}", "Interlude", "♪♫"];

  for (const marker of markers) {
    const [cue] = parseLrcLine(`[00:12.50]${marker}`);
    assert.equal(cue.kind, "break", marker);
    assert.equal(cue.breakSource, "instrumental", marker);
    assert.equal(cue.text, "", marker);
  }

  const [sungLine] = parseLrcLine("[00:12.50]We sing through the instrumental night");
  assert.equal(sungLine.kind, "lyric");
});

test("shows an intro before the first timed cue", () => {
  const cues = parseLrcLyrics("[00:05.00]Opening lyric");

  assert.equal(hasSyncedLyricsIntro(cues), true);
  assert.deepEqual(resolveSyncedLyricsState(cues, 0), { kind: "intro", activeCueIndex: -1 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 4), { kind: "intro", activeCueIndex: -1 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 5), { kind: "lyric", activeCueIndex: 0 });
  assert.equal(hasSyncedLyricsIntro(parseLrcLyrics("[00:00.00]Opening lyric")), false);
  assert.equal(hasSyncedLyricsIntro(parseLrcLyrics("[00:00.00]Opening lyric"), 2), true);
});

test("does not infer breaks from long lyric gaps or invent an outro", () => {
  const cues = parseLrcLyrics([
    "[00:01.00]A line that may still be sung",
    "[00:20.00]The next line",
  ].join("\n"));

  assert.deepEqual(resolveSyncedLyricsState(cues, 15), { kind: "lyric", activeCueIndex: 0 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 120), { kind: "lyric", activeCueIndex: 1 });
});

test("an explicit final break cue is timing evidence for an outro", () => {
  const cues = parseLrcLyrics("[00:01.00]Last lyric\n[00:10.00][Instrumental]");

  assert.deepEqual(resolveSyncedLyricsState(cues, 60), { kind: "break", activeCueIndex: 1 });
});

test("keeps consecutive blank cues as one continuous displayed break", () => {
  const cues = parseLrcLyrics("[00:02.00]\n[00:04.00]\n[00:06.00]Return");

  assert.equal(cues.length, 3);
  assert.equal(startsSyncedLyricBreak(cues, 0), true);
  assert.equal(startsSyncedLyricBreak(cues, 1), false);
  assert.deepEqual(resolveSyncedLyricsState(cues, 3), { kind: "break", activeCueIndex: 0 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 5), { kind: "break", activeCueIndex: 0 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 6), { kind: "lyric", activeCueIndex: 2 });
});

test("seeking backward recomputes the cue without retaining later state", () => {
  const cues = parseLrcLyrics("[00:01.00]One\n[00:03.00]\n[00:05.00]Two");

  assert.equal(resolveSyncedLyricsState(cues, 5).kind, "lyric");
  assert.deepEqual(resolveSyncedLyricsState(cues, 3.5), { kind: "break", activeCueIndex: 1 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 1.5), { kind: "lyric", activeCueIndex: 0 });
});

test("applies positive and negative lyric offsets to all cue kinds", () => {
  const cues = parseLrcLyrics("[00:05.00]One\n[00:08.00]\n[00:10.00]Two");

  assert.equal(resolveSyncedLyricsState(cues, 5, 1).kind, "intro");
  assert.deepEqual(resolveSyncedLyricsState(cues, 6, 1), { kind: "lyric", activeCueIndex: 0 });
  assert.deepEqual(resolveSyncedLyricsState(cues, 7, -1), { kind: "break", activeCueIndex: 1 });
});

test("track changes use only the newly supplied timeline", () => {
  const firstTrack = parseLrcLyrics("[00:01.00]First\n[00:02.00]");
  const secondTrack = parseLrcLyrics("[00:01.00]Second");

  assert.equal(resolveSyncedLyricsState(firstTrack, 3).kind, "break");
  assert.deepEqual(resolveSyncedLyricsState(secondTrack, 3), { kind: "lyric", activeCueIndex: 0 });
});

test("keeps multiple timestamps on blank and lyric lines", () => {
  const cues = parseLrcLyrics("[00:01.00][00:02.00]\n[00:03.00][00:04.00]Echo");

  assert.deepEqual(cues.map(({ kind, timeSeconds }) => ({ kind, timeSeconds })), [
    { kind: "break", timeSeconds: 1 },
    { kind: "break", timeSeconds: 2 },
    { kind: "lyric", timeSeconds: 3 },
    { kind: "lyric", timeSeconds: 4 },
  ]);
  assert.deepEqual(resolveSyncedLyricsState(cues, 2.5), { kind: "break", activeCueIndex: 0 });
});
