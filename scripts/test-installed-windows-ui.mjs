import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { copyFile, mkdir, rename } from "node:fs/promises";
import net from "node:net";
import { dirname, join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const [application, fixtureDirectory, testDirectory] = process.argv.slice(2);
assert(application && fixtureDirectory && testDirectory, "Expected application, sample library, and test directory");
assert.equal(process.env.RUNNER_ENVIRONMENT, "github-hosted", "The scan smoke test requires a disposable runner profile");

async function waitFor(check, message) {
  const deadline = Date.now() + 30_000;
  do {
    const result = await check();
    if (result) return result;
    await delay(250);
  } while (Date.now() < deadline);
  throw new Error(message);
}

async function findDebugPort() {
  const server = net.createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address();
  server.close();
  await once(server, "close");
  return port;
}

async function connectToWebView(port, child) {
  const endpoint = `http://127.0.0.1:${port}/json/list`;
  const target = await waitFor(async () => {
    assert.equal(child.exitCode, null, "Cassette exited during startup");
    try {
      const response = await fetch(endpoint, { signal: AbortSignal.timeout(1_000) });
      if (!response.ok) return null;
      return (await response.json()).find(({ url, type }) =>
        type === "page" && /^https?:\/\/tauri\.localhost\//.test(url),
      );
    } catch {
      return null;
    }
  }, "Cassette did not expose its WebView2 test page");
  const webSocketUrl = new URL(target.webSocketDebuggerUrl);
  assert.equal(webSocketUrl.hostname, "127.0.0.1");
  assert.equal(webSocketUrl.port, String(port));
  const socket = new WebSocket(webSocketUrl);
  await once(socket, "open", { signal: AbortSignal.timeout(5_000) });
  return socket;
}

function makeDevToolsClient(socket) {
  let nextId = 0;

  async function send(method, params = {}) {
    const id = ++nextId;
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => finish(new Error(`WebView2 timed out on ${method}`)), 30_000);
      function finish(error, value) {
        clearTimeout(timeout);
        socket.removeEventListener("message", onMessage);
        if (error) reject(error);
        else resolve(value);
      }
      function onMessage(event) {
        const response = JSON.parse(event.data);
        if (response.id !== id) return;
        finish(response.error && new Error(response.error.message), response.result);
      }
      socket.addEventListener("message", onMessage);
      socket.send(JSON.stringify({ id, method, params }));
    });
  }

  async function evaluate(expression) {
    const { result, exceptionDetails } = await send("Runtime.evaluate", {
      expression, awaitPromise: true, returnByValue: true,
    });
    assert(!exceptionDetails, exceptionDetails?.exception?.description ?? exceptionDetails?.text);
    return result.value;
  }

  return { evaluate, send };
}

const port = await findDebugPort();
const child = spawn(application, [], {
  cwd: dirname(application),
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
    WEBVIEW2_USER_DATA_FOLDER: join(testDirectory, "webview2-profile"),
  },
  stdio: "ignore",
});
await once(child, "spawn");

let socket;
try {
  socket = await connectToWebView(port, child);
  const { evaluate, send } = makeDevToolsClient(socket);
  const ready = "Boolean(document.querySelector('.app-shell') && window.__TAURI_INTERNALS__?.invoke)";
  const interfaceLoaded = () => evaluate(ready).catch(() => false);
  await waitFor(interfaceLoaded, "Cassette's bundled interface did not load");

  async function reload() {
    const originalLoad = await evaluate("performance.timeOrigin");
    await send("Page.reload");
    const newInterface = () => evaluate(`performance.timeOrigin !== ${originalLoad} && ${ready}`).catch(() => false);
    await waitFor(newInterface, "Cassette's bundled interface did not reload");
  }

  const invoke = (command, args = {}) => evaluate(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)})`,
  );
  assert.equal((await invoke("get_platform_capabilities")).os, "windows");
  assert.equal(await evaluate("document.documentElement.dataset.theme"), "obsidian");

  const extensions = ["flac", "m4a", "mp3", "ogg", "opus", "wav"];
  const tracks = await invoke("scan_library", { root: fixtureDirectory });
  assert.deepEqual(tracks.map(({ extension }) => extension).sort(), extensions);
  const originalCache = await invoke("get_library_cache");
  assert.equal(originalCache.tracks.length, extensions.length);
  assert.equal(originalCache.lastScannedFolder, fixtureDirectory);

  await reload();
  const openSettings = () => evaluate(`(() => {
    const settings = [...document.querySelectorAll('.sidebar button')]
      .find((item) => item.lastElementChild?.textContent.trim() === 'Settings');
    settings?.click();
    return Boolean(settings);
  })()`);
  await waitFor(openSettings, "Cassette's Settings button was missing");
  await waitFor(() => evaluate(`Boolean(document.querySelector('.library-folder-card'))`), "Library settings did not load");
  await delay(1_100);
  const scan = await evaluate(`(() => {
    const button = [...document.querySelectorAll('button')]
      .find((item) => item.textContent.trim() === 'Rescan Library' && !item.disabled);
    button?.click();
    return Boolean(button);
  })()`);
  assert(scan, "Rescan Library was missing or disabled");
  const newCache = await waitFor(async () => {
    const cache = await invoke("get_library_cache");
    return cache.lastScannedAt > originalCache.lastScannedAt && cache;
  }, "Rescan Library did not refresh the native library cache");
  assert.deepEqual(newCache.tracks.map(({ extension }) => extension).sort(), extensions);
  console.log("Installed Cassette scanned all six synthetic audio formats and rescanned the saved folder.");

  const importFolder = join(testDirectory, "disposable-import");
  const sample = tracks.find(({ extension }) => extension === "flac");
  assert(sample, "FLAC fixture is missing");
  await mkdir(importFolder);
  await copyFile(sample.filePath, join(importFolder, "imported.flac"));
  const imported = await invoke("scan_library", { root: importFolder, append: true });
  assert.equal(imported.length, extensions.length + 1);
  assert.equal((await invoke("get_library_cache")).lastScannedFolder, fixtureDirectory);
  const rescanned = await invoke("scan_library", { root: fixtureDirectory });
  assert.equal(rescanned.length, extensions.length + 1);
  console.log("Installed Cassette added a separate folder and kept its tracks on rescan.");

  const importedTrack = imported.find(({ filePath }) => filePath.startsWith(importFolder));
  assert(importedTrack);
  const disconnected = join(testDirectory, "disconnected-import");
  await rename(importFolder, disconnected);
  try {
    const retained = await invoke("scan_library", { root: fixtureDirectory });
    assert(retained.some(({ id }) => id === importedTrack.id));
    const cache = await invoke("get_library_cache");
    assert(cache.unavailableTrackIds.includes(importedTrack.id));
    assert(cache.folders.find(({ path }) => path === importFolder)?.error);
  } finally {
    await rename(disconnected, importFolder);
  }
  await invoke("scan_library", { root: fixtureDirectory });
  assert(!(await invoke("get_library_cache")).unavailableTrackIds.includes(importedTrack.id));
  console.log("Installed Cassette kept a disconnected folder’s tracks and restored availability on rescan.");

  const lyrics = "[00:01.50] Synthetic lyric";
  const edited = await invoke("save_edited_lyrics", { trackPath: sample.id, kind: "synced", text: lyrics });
  assert.equal(edited.source, "edited");
  assert.equal((await invoke("read_track_lyrics", { trackPath: sample.id })).text, lyrics);
  await invoke("restore_original_lyrics", { trackPath: sample.id });
  assert.notEqual((await invoke("read_track_lyrics", { trackPath: sample.id }))?.source, "edited");
  const added = await invoke("set_ignored_tracks", { trackIds: [sample.id, importedTrack.id], ignored: true });
  assert(added.includes(sample.id) && added.includes(importedTrack.id));
  const cachedIgnores = (await invoke("get_library_cache")).ignoredTrackIds;
  assert(cachedIgnores.includes(sample.id) && cachedIgnores.includes(importedTrack.id));
  await invoke("set_ignored_tracks", { trackIds: [sample.id, importedTrack.id], ignored: false });
  console.log("Installed Cassette saved and restored lyrics and bulk ignored tracks in its isolated library.");

  const light = await evaluate(`(() => {
    const preset = [...document.querySelectorAll('.theme-preset-grid button')]
      .find((item) => item.querySelector('strong')?.textContent.trim() === 'Light');
    preset?.click();
    return Boolean(preset);
  })()`);
  assert(light, "Light mode was unavailable in the packaged interface");
  await waitFor(() => evaluate("getComputedStyle(document.documentElement).colorScheme === 'light'"), "Light mode did not apply");
  await reload();
  assert.equal(await evaluate("document.documentElement.dataset.theme"), "light");
  console.log("Installed Cassette applied Light mode and restored it on reload.");
} finally {
  socket?.close();
  if (child.exitCode === null) {
    child.kill();
    await once(child, "exit");
  }
}
