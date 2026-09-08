import { readFileSync } from "node:fs";
import { resolve } from "node:path";

export function validateUpdaterConfiguration(config) {
  const updater = config.plugins?.updater;
  if (!updater || typeof updater.pubkey !== "string") {
    throw new Error("Signed AppImage requires a configured public verification key.");
  }
  const key = updater.pubkey.trim();
  const decoded = Buffer.from(key, "base64");
  const lines = decoded.toString("utf8").trim().split(/\r?\n/);
  const rawKey = Buffer.from(lines[1] || "", "base64");
  if (decoded.toString("base64") !== key || lines.length !== 2
      || !lines[0].startsWith("untrusted comment:") || rawKey.length !== 42
      || rawKey.subarray(0, 2).toString() !== "Ed"
      || /placeholder|replace|todo|example/i.test(key)) {
    throw new Error("Updater public verification key is malformed.");
  }
  if (!Array.isArray(updater.endpoints) || updater.endpoints.length !== 1) {
    throw new Error("Signed AppImage requires exactly one configured HTTPS beta endpoint.");
  }
  const endpoint = new URL(updater.endpoints[0]);
  if (endpoint.protocol !== "https:" || endpoint.hostname !== "atilla-m.github.io"
      || endpoint.pathname !== "/cassette/updates/beta/latest.json"
      || endpoint.username || endpoint.password || endpoint.port || endpoint.search || endpoint.hash
      || Object.keys(updater).some((key) => key.startsWith("dangerous") && updater[key])) {
    throw new Error("Updater endpoint or transport configuration is invalid.");
  }
}

export function validateReleaseArguments(args, config, overlay, environment, platform = process.platform) {
  if (environment.TAURI_CONFIG) throw new Error("Ambient TAURI_CONFIG overrides are forbidden for release builds.");
  // A closed argument set prevents aliases, repeated options or later overlays
  // from silently undoing the requirements checked here.
  const ordinary = platform === "win32"
    ? ["--target", "x86_64-pc-windows-msvc", "--bundles", "nsis"]
    : ["--bundles", "deb,rpm"];
  const signed = ["--features", "signed-updater", "--bundles", "appimage,deb,rpm",
    "--config", "src-tauri/tauri.updater.conf.json", "--ci"];
  const same = (expected) => JSON.stringify(args) === JSON.stringify(expected);
  if (same(ordinary)) return false;
  if (platform !== "linux" || !same(signed)) {
    throw new Error("Unsupported release arguments; use release:linux or release:linux:signed exactly.");
  }
  validateUpdaterConfiguration(config);
  if (JSON.stringify(overlay.bundle) !== JSON.stringify({ createUpdaterArtifacts: true })
      || Object.keys(overlay).some((key) => !["$schema", "bundle"].includes(key))) {
    throw new Error("Signed AppImage overlay must only enable createUpdaterArtifacts.");
  }
  if (!environment.TAURI_SIGNING_PRIVATE_KEY?.trim()) {
    throw new Error("Signed AppImage requires signing credentials.");
  }
  // Never inspect, print, or persist the private key/password.
  return true;
}

export function readReleaseConfiguration(workspace) {
  return JSON.parse(readFileSync(resolve(workspace, "src-tauri/tauri.conf.json"), "utf8"));
}
