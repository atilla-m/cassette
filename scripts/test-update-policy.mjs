import assert from "node:assert/strict";
import test from "node:test";

import {
  UPDATE_CHECK_INTERVAL_MS,
  checkForCassetteUpdate,
  getUpdateRuntimeInfo,
  loadUpdaterStorage,
  persistUpdaterStorage,
  shouldRunAutomaticUpdateCheck,
  storedSuccessfulCheck,
  updateReleaseUrl,
  automaticAttemptTimestamp,
} from "../src/lib/api/updates.ts";

test("development runtimes do not reach the updater plugin", async () => {
  const runtime = await getUpdateRuntimeInfo();
  assert.deepEqual(runtime, {
    development: true,
    platform: "development",
    packageKind: "unsupported",
    canSelfInstall: false,
    updaterAvailable: false,
  });
  await assert.rejects(
    checkForCassetteUpdate(runtime),
    /unavailable in development builds/,
  );
});

test("automatic checks default to due without an attempt timestamp", () => {
  assert.equal(shouldRunAutomaticUpdateCheck(true, null, 1_000), true);
  assert.equal(shouldRunAutomaticUpdateCheck(false, null, 1_000), false);
});

test("automatic checks are limited to the 24-hour attempt interval", () => {
  const now = 2_000_000_000_000;
  assert.equal(
    shouldRunAutomaticUpdateCheck(true, now - UPDATE_CHECK_INTERVAL_MS + 1, now),
    false,
  );
  assert.equal(
    shouldRunAutomaticUpdateCheck(true, now - UPDATE_CHECK_INTERVAL_MS, now),
    true,
  );
});

test("only finite non-negative successful timestamps are accepted", () => {
  assert.equal(storedSuccessfulCheck(null), null);
  assert.equal(storedSuccessfulCheck(""), null);
  assert.equal(storedSuccessfulCheck("not-a-number"), null);
  assert.equal(storedSuccessfulCheck("-1"), null);
  assert.equal(storedSuccessfulCheck("1234"), 1234);
});

test("release links use an immutable encoded version tag", () => {
  assert.equal(
    updateReleaseUrl("0.1.0-beta.1"),
    "https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.1",
  );
  assert.equal(
    updateReleaseUrl("1.0.0/unsafe"),
    "https://github.com/atilla-m/cassette/releases/tag/v1.0.0%2Funsafe",
  );
});

test("failed automatic attempts survive reloads and delay retries for 24 hours", () => {
  const now = 2_000_000_000_000;
  const attempted = automaticAttemptTimestamp(String(now), null, now + 1);
  assert.equal(shouldRunAutomaticUpdateCheck(true, attempted, now + 1), false);
  assert.equal(shouldRunAutomaticUpdateCheck(true, attempted, now + UPDATE_CHECK_INTERVAL_MS), true);
});

test("migration and corrupt/future attempt timestamps fail conservatively", () => {
  const now = 100_000;
  assert.equal(automaticAttemptTimestamp(null, null, now), null);
  assert.equal(automaticAttemptTimestamp(null, "1234", now), 1234);
  for (const stored of ["", "NaN", "Infinity", "-1", String(now + 1)]) {
    assert.equal(automaticAttemptTimestamp(stored, null, now), now);
    assert.equal(shouldRunAutomaticUpdateCheck(true, automaticAttemptTimestamp(stored, null, now), now), false);
  }
});

function memoryStorage(initial = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem(key) { return values.has(key) ? values.get(key) : null; },
    setItem(key, value) { values.set(key, String(value)); },
    removeItem(key) { values.delete(key); },
  };
}

test("throwing updater storage reads disable automatic networking without corrupting UI defaults", () => {
  const storage = memoryStorage();
  storage.getItem = () => { throw new Error("storage denied"); };
  assert.deepEqual(loadUpdaterStorage(storage, 100_000), {
    automaticChecksEnabled: false,
    lastSuccessfulCheck: null,
    lastAutomaticAttempt: null,
    storageAvailable: false,
  });
});

test("throwing updater storage writes fail closed", () => {
  const migrationStorage = memoryStorage({
    "cassette:last-successful-update-check": "1234",
  });
  migrationStorage.setItem = () => { throw new Error("storage full"); };
  assert.equal(loadUpdaterStorage(migrationStorage, 100_000).storageAvailable, false);
  assert.equal(persistUpdaterStorage(migrationStorage, "cassette:auto-check-updates", "off"), false);
});

test("throwing updater storage removals are contained", () => {
  const storage = memoryStorage();
  storage.removeItem = () => { throw new Error("storage denied"); };
  assert.equal(persistUpdaterStorage(storage, "cassette:last-successful-update-check", null), false);
});
