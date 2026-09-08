import assert from "node:assert/strict";
import test from "node:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { validateReleaseArguments } from "./release-policy.mjs";
import { parseTauriInvocation, validateTauriInvocation } from "./tauri.mjs";
import { snapshotUpdatePair } from "./generate-update-feed.mjs";
const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const pkg = JSON.parse(read("package.json"));
const config = JSON.parse(read("src-tauri/tauri.conf.json"));
const overlay = JSON.parse(read("src-tauri/tauri.updater.conf.json"));

test("default and credential-free Linux packaging cannot produce AppImage", () => {
  assert.ok(!config.bundle.targets.includes("appimage"));
  assert.equal(pkg.scripts["release:linux"], "node scripts/build-release.mjs --bundles deb,rpm");
  assert.equal(validateReleaseArguments(["--bundles", "deb,rpm"], config, overlay, {}, "linux"), false);
  for (const args of [["--bundles", "appimage"], ["--bundles", "all"], ["--bundles", "deb,rpm", "--config", "arbitrary"]]) {
    assert.throws(() => validateReleaseArguments(args, config, overlay, {}, "linux"));
  }
});

test("signed path stops without production configuration before invoking builder", () => {
  const args = pkg.scripts["release:linux:signed"].split(" ").slice(2);
  assert.throws(() => validateReleaseArguments(args, { plugins: {} }, overlay, {}, "linux"), /public verification key/);
  assert.throws(() => validateReleaseArguments(args, config, overlay, { TAURI_CONFIG: "override" }, "linux"), /TAURI_CONFIG/);
});

test("npm tauri wrapper finds build and bundle after every supported global verbosity form", () => {
  for (const args of [
    ["-v", "build"],
    ["--verbose", "build"],
    ["-vv", "bundle"],
    ["-v", "--verbose", "build"],
  ]) {
    const parsed = parseTauriInvocation(args);
    assert.equal(parsed.command, args.at(-1));
    assert.equal(parsed.commandIndex, args.length - 1);
  }
});

test("npm tauri root parser rejects hidden, value-taking, separator, and reordered commands", () => {
  for (const args of [
    ["--config", "hidden.json", "build"],
    ["--target", "x86_64-unknown-linux-gnu", "build"],
    ["--", "build", "--bundles", "appimage"],
    ["--help", "build"],
    ["unknown", "build"],
  ]) {
    assert.throws(() => parseTauriInvocation(args), /unsupported|ambiguous|standalone/i);
  }
  assert.throws(
    () => validateTauriInvocation(["build", "--verbose", "bundle"]),
    /ambiguous/i,
  );
});

test("ordinary build and bundle reject every AppImage, all, config, feature, and separator bypass", () => {
  const attempts = [
    ["build", "--bundles", "appimage"],
    ["-v", "build", "--bundles", "appimage"],
    ["--verbose", "bundle", "--bundles", "all"],
    ["build", "--bundles=appimage"],
    ["bundle", "--bundles=all"],
    ["build", "-bappimage"],
    ["build", "-b=appimage"],
    ["build", "--bundles", "deb", "appimage"],
    ["build", "--config", "arbitrary"],
    ["bundle", "--config=arbitrary"],
    ["build", "-carbitrary"],
    ["build", "--features", "signed-updater"],
    ["bundle", "--features=signed-updater"],
    ["bundle", "-f=signed-updater"],
    ["build", "--features", "diagnostic", "signed-updater"],
    ["build", "--", "--bundles", "appimage"],
    ["-v", "bundle", "--debug", "--config", "arbitrary", "--bundles=appimage"],
  ];
  for (const args of attempts) {
    assert.throws(() => validateTauriInvocation(args), /AppImage|all-bundle|config|signed-updater|separator/i);
  }
  assert.throws(
    () => validateTauriInvocation(["build"], { TAURI_CONFIG: "override.json" }),
    /config-overlay/i,
  );
});

test("normal development, information, and ordinary build invocations remain permitted", () => {
  for (const args of [
    [],
    ["--help"],
    ["--version"],
    ["dev"],
    ["-v", "dev", "--host", "127.0.0.1"],
    ["info"],
    ["signer", "generate", "--help"],
    ["build"],
    ["--verbose", "build", "--debug"],
    ["bundle", "--bundles", "deb,rpm"],
  ]) {
    assert.doesNotThrow(() => validateTauriInvocation(args, {}));
  }
});

test("native-only installation and recoverable initialization remain mandatory", () => {
  const permissions = JSON.parse(read("src-tauri/capabilities/linux-updater.json")).permissions;
  assert.ok(permissions.every((permission) => !permission.startsWith("updater:")));
  const frontend = read("src/lib/api/updates.ts");
  assert.ok(!frontend.includes("@tauri-apps/plugin-updater"));
  assert.match(frontend, /invoke\("install_cassette_update"/);
  assert.match(frontend, /operationId: update\.operationId/);
  const page = read("src/routes/+page.svelte");
  assert.match(page, /const confirmedUpdate = availableUpdate/);
  assert.match(page, /installAppImageUpdate\(confirmedUpdate/);
  const native = read("src-tauri/src/updates.rs");
  assert.match(native, /application startup continues/);
  assert.match(native, /valid_configuration/);
  assert.match(native, /platform::bundle_type\(\)/);
  assert.match(native, /BundleType::AppImage/);
  assert.match(native, /pending_authorizes/);
  assert.match(native, /still_matches_path/);
  assert.match(native, /operation_id/);
});

test("release/feed require exact signed assets and verification; CI has no AppImage", () => {
  const ci = read(".github/workflows/ci.yml");
  const release = read(".github/workflows/release.yml");
  const feed = read(".github/workflows/publish-update-feed.yml");
  assert.ok(!ci.includes(".AppImage"));
  assert.match(release, /npm run release:linux:signed/);
  assert.match(release, /node scripts\/verify-update.mjs/);
  assert.match(release, /needs: \[preflight, build-linux\]/);
  assert.match(release, /--draft/);
  assert.ok(!release.includes("release:windows"));
  assert.match(feed, /types: \[published\]/);
  assert.match(feed, /github.event.release.prerelease == true/);
  assert.match(feed, /APPIMAGE_FILE:/);
  const generator = read("scripts/generate-update-feed.mjs");
  assert.match(generator, /verifyUpdate\(snapshot.image, snapshot.signature/);
  assert.match(generator, /readFileSync\(snapshot.signature/);
  assert.ok(!feed.includes("contents: write"));
});

test("feed snapshot cannot publish a signature changed after verification", () => {
  const directory = mkdtempSync(join(tmpdir(), "cassette-feed-snapshot-test-"));
  const image = join(directory, "Cassette_0.1.0-beta.1_amd64.AppImage");
  const signature = `${image}.sig`;
  writeFileSync(image, "original AppImage bytes");
  writeFileSync(signature, "signature bytes accepted by the verifier");
  const snapshot = snapshotUpdatePair(image, signature, "0.1.0-beta.1");
  try {
    const cryptographicallyVerifiedBytes = readFileSync(snapshot.signature, "utf8");
    writeFileSync(signature, "signature bytes swapped after verification");
    writeFileSync(image, "AppImage bytes swapped after verification");
    const bytesSerializedIntoFeed = readFileSync(snapshot.signature, "utf8");
    assert.equal(bytesSerializedIntoFeed, cryptographicallyVerifiedBytes);
    assert.notEqual(bytesSerializedIntoFeed, readFileSync(signature, "utf8"));
    assert.equal(readFileSync(snapshot.image, "utf8"), "original AppImage bytes");
  } finally {
    snapshot.dispose();
    rmSync(directory, { recursive: true, force: true });
  }
});
