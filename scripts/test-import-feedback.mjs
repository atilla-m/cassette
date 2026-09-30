import assert from "node:assert/strict";
import test from "node:test";
import { importCompletionHeading } from "../src/lib/utils/importFeedback.ts";

test("import completion distinguishes dated events, legacy-only adjustments, and a no-op", () => {
  const result = { importedEvents: 0, importedUndatedPlays: 0, matchedTracks: 1, pendingTracks: 0 };
  assert.equal(importCompletionHeading({ ...result, importedEvents: 304 }), "Import completed — 304 events added.");
  assert.equal(importCompletionHeading({ ...result, importedUndatedPlays: 180 }), "Import completed — 0 events added.");
  assert.equal(importCompletionHeading(result), "Import completed — no new history added.");
});
