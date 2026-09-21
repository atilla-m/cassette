import assert from "node:assert/strict";
import test from "node:test";
import {
  PlaybackSeekCoordinator,
  failedSeekRecoveryPosition,
} from "../src/lib/utils/playbackSeek.ts";

test("a seek invalidates position updates captured before or during it", () => {
  const coordinator = new PlaybackSeekCoordinator();
  coordinator.reset("track-a");
  const beforeSeek = coordinator.capturePositionUpdate("track-a");
  const request = coordinator.request("track-a", 18);
  const duringSeek = coordinator.capturePositionUpdate("track-a");

  assert.equal(coordinator.allowsPositionUpdate(beforeSeek), false);
  assert.equal(coordinator.takeNext(), request);
  assert.equal(coordinator.allowsPositionUpdate(duringSeek), false);
  assert.equal(coordinator.complete(request), true);
  assert.equal(coordinator.allowsPositionUpdate(duringSeek), false);
  assert.equal(coordinator.allowsPositionUpdate(coordinator.capturePositionUpdate("track-a")), true);
});

test("rapid seeks coalesce queued work and settle on the latest request", () => {
  const coordinator = new PlaybackSeekCoordinator();
  coordinator.reset("track-a");
  const first = coordinator.request("track-a", 10);

  assert.equal(coordinator.takeNext(), first);

  coordinator.request("track-a", 20);
  const latest = coordinator.request("track-a", 30);

  assert.equal(coordinator.complete(first), false);
  assert.equal(coordinator.takeNext(), latest);
  assert.equal(coordinator.complete(latest), true);
  assert.equal(coordinator.takeNext(), null);
});

test("zero is a valid latest seek target", () => {
  const coordinator = new PlaybackSeekCoordinator();
  coordinator.reset("track-a");
  const request = coordinator.request("track-a", 0);

  assert.equal(coordinator.takeNext(), request);
  assert.equal(request.positionSeconds, 0);
  assert.equal(coordinator.complete(request), true);
});

test("track changes invalidate in-flight and queued seeks", () => {
  const coordinator = new PlaybackSeekCoordinator();
  coordinator.reset("track-a");
  const oldTrackSeek = coordinator.request("track-a", 12);
  coordinator.takeNext();
  coordinator.request("track-a", 24);

  coordinator.reset("track-b");

  assert.equal(coordinator.complete(oldTrackSeek), false);
  assert.equal(coordinator.takeNext(), null);
  assert.equal(coordinator.allowsPositionUpdate(coordinator.capturePositionUpdate("track-b")), true);
});

test("failed seeks prefer a recovered backend position and otherwise restore the last confirmed one", () => {
  assert.equal(failedSeekRecoveryPosition(14, 8), 14);
  assert.equal(failedSeekRecoveryPosition(0, 8), 0);
  assert.equal(failedSeekRecoveryPosition(null, 8), 8);
  assert.equal(failedSeekRecoveryPosition(Number.NaN, 8), 8);
});
