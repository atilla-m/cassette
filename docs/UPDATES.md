# Cassette Linux updater and signing handoff

## Current status

The production public key and beta Pages address are committed and unchanged. The published beta.2 release completed signed-artifact, tamper-rejection, genuine FUSE-mounted AppImage, clean Ubuntu 24.04 DEB/AppImage, clean Fedora 44 RPM, six-format playback, and isolated beta.1.e2e → beta.2 updater/data-preservation qualification. The production feed continues to serve beta.2 until beta.3 is intentionally published. Beta.3 is unreleased and must repeat signed-candidate, packaged-feature, beta.2 database-migration, and publication checks; ordinary CI is not that qualification.

The immutable beta.1 evidence remains historical: annotated tag `v0.1.0-beta.1` (tag object `f2f7dc0c1af47a370d5df6f7a0e61bb6bd8ed087`) targets commit `e239ead18d0b73032498038532b2768ecce25a3e`; signed workflow run `34475557727` produced artifact `10151906575` (`169926732` archive bytes, SHA-256 `460c5dac018a2f65c8d474d1fb149c89018ccd485d8ee4a10f6b98bee767bc0b`), recovered into unpublished draft release `386309067`. The verified files were AppImage `42c2004528577f3a18fcf395ebd159d048b0dde54e2c03963d6fab765cf88ed2`, signature `d4a78da58e755b11b18dbd11747f9c7bc5e18b0487886d827282eb4cd5c991b7`, DEB `308bfa20ee83038e70a0a2fe7d8a1245b43971187126e73b4158613fa11bdb80`, and RPM `6ef405d17c9653b04aa226bfaa3941130d9ecf3e7a13a742e98cf13559be481d`. These values document the superseded candidate; they are not selectors for the current release.

Tauri updater signatures are mandatory. Cassette must never publish an unsigned updater payload, disable signature verification, or substitute a placeholder public key.

## User-facing behavior

- Distributable signed release builds can check the stable beta feed. Credential-free package-validation builds report updates unavailable. Development builds never contact it, including when the manual check button is used.
- Automatic checks default to enabled and occur at most once every 24 hours, measured from the last automatic attempt, including failed attempts. The timestamp is persisted before the request and a per-session guard also applies. Manual checks remain available at any time after runtime detection. A failed background request is silent and does not delay startup or playback.
- A newer version is shown with its release notes. The user can choose **Later**; there is no countdown or forced installation.
- Only Linux with Tauri's embedded AppImage bundle marker can self-install. Native code canonicalizes `APPIMAGE`, `APPDIR`, and the current executable, requires a regular type-2 x86_64 ELF AppImage, checks `usr/bin/cassette`, `AppRun`, and `Cassette.desktop` inside the AppDir, and requires read-only FUSE mount evidence tying the directory to that exact image. It also captures the image's device, inode, size, modification time, and change time and revalidates that identity before download and immediately before installation. Missing, replaced, stale, inconsistent, or escaping symlink evidence falls back to download-only. Extract-and-run and runtimes whose mount source cannot be identified also fall back safely; actual release runtime compatibility remains a checkpoint 3B test.
- **Update and restart** asks for explicit confirmation before download. There is no cancellation after confirmation in the current beta. Tauri verifies the downloaded bytes, native code reauthorizes the exact AppImage immediately before installation, then Cassette relaunches. Each successful check gets a new opaque native operation identifier, so a stale confirmation cannot install a same-version update that replaced the pending operation. No direct updater permissions are granted to the webview; it can request only that opaque pending operation, never supply an arbitrary URL, path, signature, or artifact.
- Confirmation is a policy boundary in Cassette's trusted bundled frontend, not a defense against a fully compromised webview. Native code remains the package/install authorization boundary. There is also an unavoidable final local-filesystem race between the last identity check and Tauri opening/replacing the AppImage pathname; device/inode and metadata checks narrow this window but do not claim to eliminate every attack by a local account that can mutate the running executable's directory.
- DEB, RPM, and unknown Linux installations offer **View download**. Cassette does not call `sudo`, `apt`, `dpkg`, `dnf`, or `rpm`, and does not replace those installations with an AppImage.
- Windows and other platforms do not use this beta updater.

Updater state uses `cassette:auto-check-updates`, `cassette:last-automatic-update-attempt`, and `cassette:last-successful-update-check` in webview local storage. On first launch after this migration, the old successful timestamp initializes the automatic-attempt timestamp. Invalid/future values are normalized to the current time and defer checking for one interval; a missing timestamp permits an initial attempt. Every updater-specific read, migration write, preference write, attempt write, and success write is contained. Any storage failure disables automatic networking for that session without aborting runtime detection; manual checks remain available and the in-memory controls remain usable. No updater preference is stored in `library.sqlite3`, and no database migration is involved.

Updater initialization failure disables updates for the session and logs a fixed diagnostic without configuration values. Cassette still opens; manual checking explains unavailability. Release preflight independently rejects invalid public-key/endpoint configuration.

## One-time production key creation

Create the key outside the repository. For this maintainer account, the recommended explicit location is `/home/atilla/.local/share/cassette-signing/cassette-updater.key`. Create and restrict the directory first, then run the official Tauri v2 generator exactly once:

```sh
mkdir -p -- /home/atilla/.local/share/cassette-signing
chmod 700 -- /home/atilla/.local/share/cassette-signing
npm run tauri signer generate -- -w /home/atilla/.local/share/cassette-signing/cassette-updater.key
```

The command creates:

- `cassette-updater.key`: the private signing key. Never commit it, attach it to a release, put it in an Actions artifact, paste it into chat, or expose it in logs.
- `cassette-updater.key.pub`: the public verification key. This is safe to share and its contents must be committed in `src-tauri/tauri.conf.json` as `plugins.updater.pubkey`.

Store at least two encrypted, offline backups of the private key in separate trusted locations. Store the password separately from those backups. Losing the private key prevents existing Cassette installations from accepting future updates; compromise requires revoking the feed and moving users to a manually installed trust root.

Add the complete private-key **file contents**, not its path, to the GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY`. If the key is password-protected, add the password string as `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; otherwise omit that second secret or leave it empty. The release workflow scopes both values only to the signed build step and never prints them.

## Public configuration checkpoint

`src-tauri/tauri.conf.json` contains the exact contents of `cassette-updater.key.pub` and configures one endpoint only:

`https://atilla-m.github.io/cassette/updates/beta/latest.json`

The configured public-key source file has SHA-256 `a99b821c856fb4db7b3d1dfa70df38a77835d199ce4c34aa065e157e31430513` (Minisign key ID `DFF061EB2AC19D0B`). The publication workflow builds `pages-site/updates/beta/latest.json` and uploads `pages-site` as the GitHub Pages artifact root, so that file maps to the configured project-site URL above. The endpoint currently serves the published beta.2 feed; beta.3 preparation must not replace it.

Do not point beta installations at GitHub's generic `/releases/latest` endpoint because prereleases may be excluded. Do not use a mutable release tag in an artifact URL. The tag-only release preflight rejects a missing or placeholder-like public key and rejects any other endpoint list.

The committed `src-tauri/tauri.updater.conf.json` overlay enables `bundle.createUpdaterArtifacts` only for `npm run release:linux:signed`. This is the sole supported AppImage-producing command. It requires the exact signed-updater feature/overlay arguments, configured public key and beta endpoint, and signing credentials before invoking Tauri. Ambient configuration overrides and alternate argument combinations are rejected.

`npm run release:linux` produces only DEB/RPM for credential-free package validation. Normal `npm run tauri build` defaults exclude AppImage; the npm Tauri wrapper refuses explicit AppImage/all-bundle and configuration-overlay bypasses. Ordinary Linux CI uploads only DEB/RPM under a package-validation name. There is no unsigned AppImage diagnostic build. Direct invocation of the dependency's CLI outside these guarded commands is unsupported and its outputs must never be distributed.

## Draft release and beta feed flow

1. The exact `v0.1.0-beta.3` tag starts `.github/workflows/release.yml`; ordinary pushes and pull requests cannot create a release.
2. The workflow validates the tag, versions, license metadata, real public-key configuration, and exact feed URL.
3. One Linux job builds DEB, RPM, AppImage, and `Cassette_0.1.0-beta.3_amd64.AppImage.sig`. Existing package verification, artifact-safety scans, signature-envelope safety checks, public-key cryptographic verification, and the Wayland-client exclusion gate must pass for the exact AppImage/signature pair.
4. Only those four exact current-version files are admitted to a draft prerelease. NSIS is not built or uploaded by this workflow. The workflow never publishes the draft.
5. A maintainer reviews and tests the downloaded draft artifacts, replaces the draft notes with final non-empty release notes, and intentionally publishes the prerelease.
6. Only the GitHub `release.published` event can start `.github/workflows/publish-update-feed.yml`. That workflow rejects drafts, non-prereleases, another tag, and any unexpected asset set.
7. The feed workflow downloads the exact published AppImage and `.sig`, copies both into one private stable snapshot, and repeats structural and cryptographic verification with the tagged public key against that snapshot. The signature serialized into `updates/beta/latest.json` is read from the same verified snapshot, so changing either downloaded source after verification cannot change the feed. The snapshot is removed after generation and is never uploaded. Generation refuses an unverifiable pair and does not commit metadata to source history. The JSON contains the exact version, immutable tagged AppImage URL, complete verified signature envelope, release notes, and publication time required by Tauri.

The Pages deployment job alone receives `pages: write` and `id-token: write`; source and release validation use `contents: read`. The draft-creation job alone receives `contents: write`.

In GitHub repository settings, manually configure **Pages → Build and deployment → Source: GitHub Actions**. Source changes cannot safely enable that repository setting. Do not publish the prerelease until the Pages workflow exists on the default branch and this setting is enabled.

## Data boundary

Updates replace application binaries only. Cassette's Linux data remains under `$XDG_DATA_HOME/io.github.atilla.cassette` (normally `~/.local/share/io.github.atilla.cassette`) and includes `library.sqlite3`, playlists, favorites, statistics, settings, and caches. Neither an update nor package uninstall should automatically erase that directory. Back up user data before any destructive manual removal.

## Beta.3 signed-candidate and migration qualification

Use the genuine published beta.2 AppImage as the old production client and the unchanged genuine signed beta.3 draft AppImage as the update target. Keep local HTTPS update qualification, real FUSE mounting, clean-system package/playback testing, and post-publication Pages/feed verification as separate evidence. Do not repurpose the historical beta.1.e2e fixture for this release.

Before beta.3 publication:

1. Download the exact beta.3 draft assets and independently verify workflow/tag/commit provenance, sizes, SHA-256 values, versions, architectures, LICENSE payloads, artifact safety, Wayland-client exclusion, the production-key signature, and disposable payload/signature tamper rejection.
2. Launch beta.2 through real FUSE with a durable isolated profile populated only with verified synthetic media. Record tracks, theme/settings, playlists, favorites, all-time counts, last-played timestamps, and the database schema before updating.
3. Serve the genuine beta.3 bytes and signature through verified loopback HTTPS with process-local test trust. Exercise **Later**, tampered-payload rejection, genuine install/relaunch, and exact installed-byte hashing without changing the public feed.
4. After relaunch, verify the beta.2 database migration preserves all legacy state, creates the detailed event ledger/indexes once, does not invent dates for undated totals, and records exactly one event plus one total increment for a new qualifying play.
5. Run the outstanding packaged-feature checks in [RELEASING.md](RELEASING.md), including album-edit cancellation, differing-artist preservation, controlled partial failure/recovery, AppImage menu integration, Fedora GNOME notifications, lyrics seeking/breaks, and full Stats access.
6. Repeat clean Ubuntu 24.04 DEB/AppImage and Fedora 44 RPM installation, six-format playback, persistence, removal/reinstallation, and real FUSE checks with the beta.3 artifacts. Prior beta.2 results establish history, not beta.3 qualification.
7. After separate publication approval, verify the public Pages JSON, anonymous downloads, recorded hashes, production signature, and a genuine beta.2 production client discovering beta.3.

## Historical beta.2 signed-build and end-to-end runbook

### Runtime remediation status before resuming this runbook

The unpublished beta.1 tag/assets remain unchanged. Source now includes a detector correction for the generated `Cassette.desktop` marker and Fedora's basename-only FUSE mount source, with backing-file identity and FUSE-connection ownership checks retained. Source regressions do not prove that a fresh signed, mounted AppImage is correctly classified.

The Ubuntu 24 diagnostic AppDir initially remained choppy. A controlled omission of only the bundled Wayland client resolved host Mesa EGL symbol-loading failures and the user confirmed smooth Albums scrolling. Production packaging now applies that omission through `src-tauri/.appimageignore`; see [the runtime evidence and portability limits](ARTIFACT-SAFETY.md#host-wayland-client-including-x11-launches). An extracted diagnostic AppDir is neither a signed release nor a FUSE/updater-installation test. No GPU selection or graphics workaround is required in the production launcher.

Beta.2 carries the reviewed detector and packaging corrections. Do not move beta.1 or replace its assets. Before tagging beta.2, record the newly reviewed commit and repeat artifact/license/signature/tamper checks, mounted detection and manual scrolling, supported-format playback (the audio-plugin warnings remain unresolved), real older-to-newer replacement and data preservation, and clean-machine DEB/RPM/FUSE checks. Publication and live-feed qualification remain a separate stop point.

Use fresh profiles populated only from verified synthetic media. One earlier host comparison profile was later found to contain real-library references; it must not be reused or described as isolated. The passing Wayland-loader diagnostic creates a fresh database with 24 synthetic albums and separate XDG roots; it does not reuse that host profile.

The updater uses SemVer and offers an update only when the feed version is greater than the embedded application version. Production builds accept exactly one endpoint, `https://atilla-m.github.io/cassette/updates/beta/latest.json`. The frontend cannot supply an endpoint, download URL, signature, or payload. Tauri consumes the download URL in the trusted feed; the production feed generator pins it to the immutable HTTPS asset URL under `github.com/atilla-m/cassette/releases/download/v0.1.0-beta.2/`.

A draft release asset is available to an authenticated maintainer through `gh release download`, but it is not anonymously downloadable and therefore cannot be the direct remote source for an updater client. The genuine beta.1 AppImage misclassifies its installation type and cannot exercise the corrected self-install path. Instead, build an isolated `0.1.0-beta.1.e2e` AppImage from the reviewed beta.2 source so it contains the corrected detector and Wayland-client packaging. It retains the production public key but permits exactly `https://localhost:44443/cassette/updates/beta/latest.json`. Serve an authenticated download of the untouched genuine `0.1.0-beta.2` draft AppImage and its signature from loopback HTTPS. `0.1.0-beta.1.e2e` is lower than `0.1.0-beta.2`; scan both executable payloads for their embedded versions. Nothing is published, and the production feed is not changed.

This arrangement proves real FUSE AppImage detection on the qualification host, SemVer discovery, production-key signature verification, tamper rejection, explicit confirmation, download, replacement, data separation, and relaunch into the genuine release-candidate bytes. Its controlled endpoint is the only test-build divergence from production transport policy. It does not prove GitHub Pages availability, the public GitHub asset path, or clean-machine FUSE/package compatibility; those have separate gates below.

### 1. Create the production draft

Tagging and pushing require separate authorization. Use a separate clean worktree so this documentation plan can remain under review without entering or being discarded from the release commit. Fail closed if the reviewed commit is no longer in remote `main` history or the tag/release already exists, then create an unsigned annotated tag at the exact reviewed commit:

```sh
set -euo pipefail
export CASSETTE_REPOSITORY=atilla-m/cassette
export REVIEWED_SHA="$(git rev-parse HEAD)"
export RELEASE_VERSION=0.1.0-beta.2
export RELEASE_TAG=v0.1.0-beta.2

MAIN_ROOT="$(git rev-parse --show-toplevel)"
git -C "$MAIN_ROOT" fetch origin
test "$REVIEWED_SHA" = "$(git -C "$MAIN_ROOT" rev-parse origin/main)"
git -C "$MAIN_ROOT" merge-base --is-ancestor "$REVIEWED_SHA" origin/main
TAG_WORKTREE_ROOT="$(mktemp -d /tmp/cassette-beta2-tag.XXXXXX)"
RELEASE_SOURCE="$TAG_WORKTREE_ROOT/source"
git -C "$MAIN_ROOT" worktree add --detach "$RELEASE_SOURCE" "$REVIEWED_SHA"
cd "$RELEASE_SOURCE"
test "$(git rev-parse HEAD)" = "$REVIEWED_SHA"
test -z "$(git status --porcelain=v1)"
test -z "$(git tag --list "$RELEASE_TAG")"
if git ls-remote --exit-code --tags origin "refs/tags/$RELEASE_TAG" >/dev/null 2>&1; then
  echo "Refusing to reuse an existing remote tag." >&2
  exit 1
fi
if gh release view "$RELEASE_TAG" --repo "$CASSETTE_REPOSITORY" >/dev/null 2>&1; then
  echo "Refusing to reuse an existing release." >&2
  exit 1
fi

git tag --no-sign -a "$RELEASE_TAG" "$REVIEWED_SHA" -m "Cassette $RELEASE_VERSION"
test "$(git rev-list -n 1 "$RELEASE_TAG")" = "$REVIEWED_SHA"
git push origin "refs/tags/$RELEASE_TAG:refs/tags/$RELEASE_TAG"
cd "$MAIN_ROOT"
git worktree remove "$RELEASE_SOURCE"
rmdir "$TAG_WORKTREE_ROOT"
```

The tag starts only `.github/workflows/release.yml`. Select the run by both SHA and tag, wait for it, and inspect its exact association:

```sh
RELEASE_RUN_ID=
for attempt in 1 2 3 4 5 6 7 8 9 10 11 12; do
  RELEASE_RUN_ID="$(gh run list --repo "$CASSETTE_REPOSITORY" \
    --workflow release.yml --event push --commit "$REVIEWED_SHA" --limit 10 \
    --json databaseId,headBranch,headSha \
    | jq -r --arg sha "$REVIEWED_SHA" \
      'map(select(.headSha == $sha)) | if length == 1 then .[0].databaseId elif length == 0 then empty else error("multiple matching release runs") end')"
  test -n "$RELEASE_RUN_ID" && break
  sleep 5
done
test -n "$RELEASE_RUN_ID"
gh run watch "$RELEASE_RUN_ID" --repo "$CASSETTE_REPOSITORY" --exit-status
gh run view "$RELEASE_RUN_ID" --repo "$CASSETTE_REPOSITORY" \
  --json url,status,conclusion,headBranch,headSha,jobs
gh release view "$RELEASE_TAG" --repo "$CASSETTE_REPOSITORY" \
  --json tagName,isDraft,isPrerelease,assets \
  | jq -e '.tagName == "v0.1.0-beta.2" and .isDraft and .isPrerelease and
    ([.assets[].name] | sort) == [
      "Cassette-0.1.0-beta.2-1.x86_64.rpm",
      "Cassette_0.1.0-beta.2_amd64.AppImage",
      "Cassette_0.1.0-beta.2_amd64.AppImage.sig",
      "Cassette_0.1.0-beta.2_amd64.deb"
    ]'
```

The expected workflow artifact is `cassette-0.1.0-beta.2-linux-x86_64-signed`. The expected draft release assets are the DEB, RPM, AppImage, and `.AppImage.sig` listed above—nothing else. The workflow must remain a draft prerelease; do not edit or publish it during qualification.

### 2. Download, scan, verify, and tamper-test the genuine pair

Create one narrowly scoped directory and retain it until its logs and hashes have been recorded. `gh` authentication is intentionally required for the draft download.

```sh
MAIN_ROOT="$(git rev-parse --show-toplevel)"
QUALIFICATION_ROOT="$(mktemp -d /tmp/cassette-3b-qualification.XXXXXX)"
ORIGINALS="$QUALIFICATION_ROOT/original-draft"
APPIMAGE_NAME=Cassette_0.1.0-beta.2_amd64.AppImage
SIGNATURE_NAME="$APPIMAGE_NAME.sig"
mkdir -p "$ORIGINALS" "$QUALIFICATION_ROOT/scan"

gh release download "$RELEASE_TAG" --repo "$CASSETTE_REPOSITORY" \
  --dir "$ORIGINALS" --pattern "$APPIMAGE_NAME" --pattern "$SIGNATURE_NAME"
test "$(find "$ORIGINALS" -maxdepth 1 -type f | wc -l)" -eq 2
(cd "$ORIGINALS" && sha256sum "$APPIMAGE_NAME" "$SIGNATURE_NAME" > SHA256SUMS)

node "$MAIN_ROOT/scripts/verify-update.mjs" \
  "$ORIGINALS/$APPIMAGE_NAME" "$ORIGINALS/$SIGNATURE_NAME" "$RELEASE_VERSION"
cp "$ORIGINALS/$APPIMAGE_NAME" "$QUALIFICATION_ROOT/scan/$APPIMAGE_NAME"
cp "$ORIGINALS/$SIGNATURE_NAME" "$QUALIFICATION_ROOT/scan/$SIGNATURE_NAME"
chmod u+x "$QUALIFICATION_ROOT/scan/$APPIMAGE_NAME"
python3 -B "$MAIN_ROOT/scripts/audit-release-artifacts.py" \
  --workspace "$MAIN_ROOT" --version "$RELEASE_VERSION" --license "$MAIN_ROOT/LICENSE" \
  --artifact "$QUALIFICATION_ROOT/scan/$APPIMAGE_NAME" \
  --artifact "$QUALIFICATION_ROOT/scan/$SIGNATURE_NAME"
```

Reject independent payload and signature mutations while keeping the originals byte-for-byte unchanged:

```sh
mkdir -p "$QUALIFICATION_ROOT/tampered-payload" "$QUALIFICATION_ROOT/tampered-signature"
cp "$ORIGINALS/$APPIMAGE_NAME" "$QUALIFICATION_ROOT/tampered-payload/$APPIMAGE_NAME"
cp "$ORIGINALS/$SIGNATURE_NAME" "$QUALIFICATION_ROOT/tampered-payload/$SIGNATURE_NAME"
printf '\x00' >> "$QUALIFICATION_ROOT/tampered-payload/$APPIMAGE_NAME"
if node "$MAIN_ROOT/scripts/verify-update.mjs" \
  "$QUALIFICATION_ROOT/tampered-payload/$APPIMAGE_NAME" \
  "$QUALIFICATION_ROOT/tampered-payload/$SIGNATURE_NAME" "$RELEASE_VERSION"; then
  echo "ERROR: tampered AppImage was accepted" >&2
  exit 1
fi

cp "$ORIGINALS/$APPIMAGE_NAME" "$QUALIFICATION_ROOT/tampered-signature/$APPIMAGE_NAME"
cp "$ORIGINALS/$SIGNATURE_NAME" "$QUALIFICATION_ROOT/tampered-signature/$SIGNATURE_NAME"
printf 'A' >> "$QUALIFICATION_ROOT/tampered-signature/$SIGNATURE_NAME"
if node "$MAIN_ROOT/scripts/verify-update.mjs" \
  "$QUALIFICATION_ROOT/tampered-signature/$APPIMAGE_NAME" \
  "$QUALIFICATION_ROOT/tampered-signature/$SIGNATURE_NAME" "$RELEASE_VERSION"; then
  echo "ERROR: tampered signature was accepted" >&2
  exit 1
fi
(cd "$ORIGINALS" && sha256sum --check SHA256SUMS)
```

### 3. Build the isolated lower-version AppImage

Create a detached worktree from the same reviewed commit. Do not make these changes on `main`. First run `npm version` so npm updates both version fields in its lockfile normally, then apply the exact test-only native/build policy patch shown below:

```sh
OLD_SOURCE="$QUALIFICATION_ROOT/old-source"
OLD_VERSION=0.1.0-beta.1.e2e
OLD_APPIMAGE_NAME=Cassette_0.1.0-beta.1.e2e_amd64.AppImage
git -C "$MAIN_ROOT" worktree add --detach "$OLD_SOURCE" "$REVIEWED_SHA"
cd "$OLD_SOURCE"
npm version "$OLD_VERSION" --no-git-tag-version --ignore-scripts
test "$(grep -Fxc 'const expectedVersion = "0.1.0-beta.2";' scripts/build-release.mjs)" -eq 1
sed -i 's/const expectedVersion = "0.1.0-beta.2";/const expectedVersion = "0.1.0-beta.1.e2e";/' \
  scripts/build-release.mjs

git apply <<'PATCH'
diff --git a/scripts/audit-release-artifacts.py b/scripts/audit-release-artifacts.py
--- a/scripts/audit-release-artifacts.py
+++ b/scripts/audit-release-artifacts.py
@@ -30,8 +30,9 @@
-EXPECTED_VERSION = "0.1.0-beta.2"
+EXPECTED_VERSION = "0.1.0-beta.1.e2e"
 EXPECTED_LICENSE = "GPL-3.0-or-later"
 NATIVE_TAURI_DEV_URL_EXCEPTIONS = {
     b"http://localhost:1420",
     b"ws://localhost:1420",
+    b"https://localhost:44443",
 }
 FRONTEND_SUFFIXES = {".css", ".html", ".js", ".json", ".map", ".mjs"}
 DATABASE_SUFFIXES = {".db", ".sqlite", ".sqlite3", ".db-shm", ".db-wal"}
diff --git a/scripts/release-policy.mjs b/scripts/release-policy.mjs
--- a/scripts/release-policy.mjs
+++ b/scripts/release-policy.mjs
@@ -20,9 +20,9 @@ export function validateUpdaterConfiguration(config) {
     throw new Error("Signed AppImage requires exactly one configured HTTPS beta endpoint.");
   }
   const endpoint = new URL(updater.endpoints[0]);
-  if (endpoint.protocol !== "https:" || endpoint.hostname !== "atilla-m.github.io"
+  if (endpoint.protocol !== "https:" || endpoint.hostname !== "localhost"
       || endpoint.pathname !== "/cassette/updates/beta/latest.json"
-      || endpoint.username || endpoint.password || endpoint.port || endpoint.search || endpoint.hash
+      || endpoint.username || endpoint.password || endpoint.port !== "44443" || endpoint.search || endpoint.hash
       || Object.keys(updater).some((key) => key.startsWith("dangerous") && updater[key])) {
     throw new Error("Updater endpoint or transport configuration is invalid.");
   }
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -1,6 +1,6 @@
 [package]
 name = "cassette"
-version = "0.1.0-beta.2"
+version = "0.1.0-beta.1.e2e"
 description = "A private, local-first desktop music library and player"
 authors = ["Cassette contributors"]
 edition = "2021"
diff --git a/src-tauri/src/updates.rs b/src-tauri/src/updates.rs
--- a/src-tauri/src/updates.rs
+++ b/src-tauri/src/updates.rs
@@ -519,11 +519,11 @@ fn valid_configuration(value: Option<&serde_json::Value>) -> bool {
         return false;
     };
     endpoint.scheme() == "https"
-        && endpoint.host_str() == Some("atilla-m.github.io")
+        && endpoint.host_str() == Some("localhost")
         && endpoint.path() == "/cassette/updates/beta/latest.json"
         && endpoint.username().is_empty()
         && endpoint.password().is_none()
-        && endpoint.port().is_none()
+        && endpoint.port() == Some(44443)
         && endpoint.query().is_none()
         && endpoint.fragment().is_none()
         && !value.as_object().is_some_and(|o| {
diff --git a/src-tauri/tauri.conf.json b/src-tauri/tauri.conf.json
--- a/src-tauri/tauri.conf.json
+++ b/src-tauri/tauri.conf.json
@@ -1,7 +1,7 @@
 {
   "$schema": "https://schema.tauri.app/config/2",
   "productName": "Cassette",
-  "version": "0.1.0-beta.2",
+  "version": "0.1.0-beta.1.e2e",
   "identifier": "io.github.atilla.cassette",
   "build": {
     "beforeDevCommand": "npm run dev",
@@ -30,7 +30,7 @@
     "updater": {
       "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IERGRjA2MUVCMkFDMTlEMEIKUldRTG5jRXE2Mkh3M3k0WEFRWmhBaDdaQVNnK2dqazhTd1Z2VHNDK1J4UDVLeVdZdk1PUzdMRFEK",
       "endpoints": [
-        "https://atilla-m.github.io/cassette/updates/beta/latest.json"
+        "https://localhost:44443/cassette/updates/beta/latest.json"
       ]
     }
   },
PATCH

cargo metadata --offline --manifest-path src-tauri/Cargo.toml --format-version 1 --no-deps >/dev/null
npm ci
npm run check
npm run test:updater
python3 -B scripts/test-signature-safety.py
CCACHE_DISABLE=1 cargo check --offline --manifest-path src-tauri/Cargo.toml --features signed-updater
git diff --check
git diff --stat
git diff -- package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock \
  src-tauri/tauri.conf.json src-tauri/src/updates.rs scripts/build-release.mjs \
  scripts/release-policy.mjs scripts/audit-release-artifacts.py
MAIN_CHANGES="$(git -C "$MAIN_ROOT" status --porcelain=v1)"
test -z "$MAIN_CHANGES" || test "$MAIN_CHANGES" = " M docs/UPDATES.md"
```

The diff must contain only the displayed version changes, the exact loopback endpoint/port, and the scanner exception for that exact test URL. The public key, HTTPS requirement, single-endpoint rule, dangerous-option rejection, signed-updater feature, signing requirement, exact signed bundle list, application source, and release workflows remain unchanged.

Run the normal guarded signed command. The subshell prevents credentials from surviving the build; `set +x` keeps them out of shell tracing. Enter the password only at the local hidden prompt—never in chat, command history, or logs.

```sh
(
  set +x
  test -f /home/atilla/.local/share/cassette-signing/cassette-updater.key
  export TAURI_SIGNING_PRIVATE_KEY="$(< /home/atilla/.local/share/cassette-signing/cassette-updater.key)"
  IFS= read -r -s -p "Local updater-key password: " TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  printf '\n'
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  npm run release:linux:signed
)

OLD_PAIR="$QUALIFICATION_ROOT/old-pair"
mkdir -p "$OLD_PAIR"
cp "src-tauri/target/release/bundle/appimage/$OLD_APPIMAGE_NAME" "$OLD_PAIR/$OLD_APPIMAGE_NAME"
cp "src-tauri/target/release/bundle/appimage/$OLD_APPIMAGE_NAME.sig" "$OLD_PAIR/$OLD_APPIMAGE_NAME.sig"
chmod u+x "$OLD_PAIR/$OLD_APPIMAGE_NAME"
python3 -B scripts/audit-release-artifacts.py --workspace . --version "$OLD_VERSION" \
  --license LICENSE --artifact "$OLD_PAIR/$OLD_APPIMAGE_NAME" \
  --artifact "$OLD_PAIR/$OLD_APPIMAGE_NAME.sig"
node scripts/verify-update.mjs "$OLD_PAIR/$OLD_APPIMAGE_NAME" \
  "$OLD_PAIR/$OLD_APPIMAGE_NAME.sig" "$OLD_VERSION"
(cd "$OLD_PAIR" && sha256sum "$OLD_APPIMAGE_NAME" "$OLD_APPIMAGE_NAME.sig" > SHA256SUMS)
```

This local lower-version build must never be uploaded or installed as a system package. Its accompanying DEB/RPM outputs are incidental consequences of the fail-closed signed command and are not qualification evidence.

### 4. Serve the verified newer bytes over isolated HTTPS

Create a disposable local CA and localhost server certificate. They are transport-only test keys, not updater signing credentials, and are confined to the qualification directory. The client keeps certificate validation enabled and trusts this CA only through its process environment.

```sh
TLS_ROOT="$QUALIFICATION_ROOT/tls"
SERVE_ROOT="$QUALIFICATION_ROOT/server"
mkdir -p "$TLS_ROOT/empty-cert-dir" "$SERVE_ROOT/cassette/updates/beta"
umask 077
openssl req -x509 -newkey rsa:3072 -sha256 -nodes -days 2 \
  -keyout "$TLS_ROOT/ca.key" -out "$TLS_ROOT/ca.crt" \
  -subj "/CN=Cassette checkpoint 3B local CA" \
  -addext "basicConstraints=critical,CA:TRUE,pathlen:0" \
  -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -newkey rsa:3072 -sha256 -nodes \
  -keyout "$TLS_ROOT/server.key" -out "$TLS_ROOT/server.csr" \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature,keyEncipherment" \
  -addext "extendedKeyUsage=serverAuth"
openssl x509 -req -in "$TLS_ROOT/server.csr" -CA "$TLS_ROOT/ca.crt" \
  -CAkey "$TLS_ROOT/ca.key" -CAcreateserial -days 2 -sha256 \
  -copy_extensions copy -out "$TLS_ROOT/server.crt"
chmod 600 "$TLS_ROOT/ca.key" "$TLS_ROOT/server.key"
openssl verify -CAfile "$TLS_ROOT/ca.crt" "$TLS_ROOT/server.crt"
openssl x509 -in "$TLS_ROOT/server.crt" -noout -checkhost localhost

cp "$ORIGINALS/$APPIMAGE_NAME" "$SERVE_ROOT/$APPIMAGE_NAME"
node --input-type=module - "$SERVE_ROOT" "$APPIMAGE_NAME" "$ORIGINALS/$SIGNATURE_NAME" <<'NODE'
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
const root = process.argv[2];
const name = process.argv[3];
const signature = readFileSync(process.argv[4], "utf8").trim();
const feed = {
  version: "0.1.0-beta.2",
  notes: "Checkpoint 3B local end-to-end fixture; not a published release.",
  pub_date: new Date().toISOString(),
  platforms: {
    "linux-x86_64": { signature, url: `https://localhost:44443/${name}` },
  },
};
const directory = join(root, "cassette", "updates", "beta");
mkdirSync(directory, { recursive: true });
writeFileSync(join(directory, "latest.json"), `${JSON.stringify(feed, null, 2)}\n`, { mode: 0o600 });
NODE

if ss -H -ltn 'sport = :44443' | grep -q .; then
  echo "TCP port 44443 is already in use; stop and choose a new exact port in every test-only location." >&2
  exit 1
fi
python3 - "$SERVE_ROOT" "$TLS_ROOT/server.crt" "$TLS_ROOT/server.key" \
  >"$QUALIFICATION_ROOT/https-server.log" 2>&1 <<'PY' &
import http.server
import ssl
import sys

serve_root, certificate, private_key = sys.argv[1:4]
handler = lambda *args, **kwargs: http.server.SimpleHTTPRequestHandler(
    *args, directory=serve_root, **kwargs
)
server = http.server.ThreadingHTTPServer(("127.0.0.1", 44443), handler)
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(certificate, private_key)
server.socket = context.wrap_socket(server.socket, server_side=True)
server.serve_forever()
PY
HTTPS_SERVER_PID=$!
curl --fail --silent --show-error --cacert "$TLS_ROOT/ca.crt" \
  --retry 5 --retry-connrefused --retry-delay 1 \
  https://localhost:44443/cassette/updates/beta/latest.json \
  --output "$QUALIFICATION_ROOT/served-latest.json"
jq -e '.version == "0.1.0-beta.2" and
  .platforms["linux-x86_64"].url == "https://localhost:44443/Cassette_0.1.0-beta.2_amd64.AppImage" and
  (.platforms["linux-x86_64"].signature | length > 0)' \
  "$QUALIFICATION_ROOT/served-latest.json"
```

The production generator is intentionally not used here because it correctly refuses a test version or non-production URL. The local JSON uses the exact signature bytes from the verified draft pair and names the genuine `0.1.0-beta.2` executable; it does not pretend an unchanged executable has a newer version.

### 5. Exercise check, rejection, install, and relaunch

Verify `/dev/fuse` is usable and launch the AppImage directly—never with `--appimage-extract-and-run`. Generate disposable media, use four isolated XDG roots, and select only the disposable media directory inside Cassette.

```sh
test -r /dev/fuse
ldconfig -p | grep -F 'libfuse.so.2'
RUN_ROOT="$QUALIFICATION_ROOT/run"
RUN_IMAGE="$RUN_ROOT/$OLD_APPIMAGE_NAME"
mkdir -p "$RUN_ROOT" "$QUALIFICATION_ROOT/media" \
  "$QUALIFICATION_ROOT/xdg/data" "$QUALIFICATION_ROOT/xdg/config" \
  "$QUALIFICATION_ROOT/xdg/cache" "$QUALIFICATION_ROOT/xdg/state"
cp "$OLD_PAIR/$OLD_APPIMAGE_NAME" "$RUN_IMAGE"
chmod u+x "$RUN_IMAGE"
ffmpeg -nostdin -hide_banner -loglevel error -f lavfi -i sine=frequency=440:duration=4 \
  -metadata title="Checkpoint Tone A" -metadata artist="Cassette E2E" \
  -metadata album="Disposable Update Test" -metadata genre="Test" \
  "$QUALIFICATION_ROOT/media/checkpoint-tone-a.flac"
ffmpeg -nostdin -hide_banner -loglevel error -f lavfi -i sine=frequency=660:duration=4 \
  -metadata title="Checkpoint Tone B" -metadata artist="Cassette E2E" \
  -metadata album="Disposable Update Test" -metadata genre="Test" \
  "$QUALIFICATION_ROOT/media/checkpoint-tone-b.flac"
(cd "$QUALIFICATION_ROOT/media" && sha256sum *.flac > "$QUALIFICATION_ROOT/media.sha256")

export XDG_DATA_HOME="$QUALIFICATION_ROOT/xdg/data"
export XDG_CONFIG_HOME="$QUALIFICATION_ROOT/xdg/config"
export XDG_CACHE_HOME="$QUALIFICATION_ROOT/xdg/cache"
export XDG_STATE_HOME="$QUALIFICATION_ROOT/xdg/state"
export SSL_CERT_FILE="$TLS_ROOT/ca.crt"
export SSL_CERT_DIR="$TLS_ROOT/empty-cert-dir"
"$RUN_IMAGE" &
QUALIFICATION_APP_PID=$!
for attempt in 1 2 3 4 5 6 7 8 9 10; do
  if grep -E ' - fuse[^ ]* ' "/proc/$QUALIFICATION_APP_PID/mountinfo" \
    > "$QUALIFICATION_ROOT/fuse-mount.txt"; then
    break
  fi
  sleep 1
done
test -s "$QUALIFICATION_ROOT/fuse-mount.txt"
```

On the first launch, confirm Settings reports version `0.1.0-beta.1.e2e`, installation type **AppImage**, and **Update and restart**. This is also the native proof that the corrected detector recognizes a real FUSE mount tied to its backing file. Confirm the automatic check discovers `0.1.0-beta.2`, displays the local fixture notes, and offers **Later**; choose **Later**. Import only `$QUALIFICATION_ROOT/media`, set a non-default theme and another harmless setting, favorite a track, create a playlist containing both tracks, and play briefly. Close and relaunch the same AppImage. The HTTPS server log must show no second automatic feed request within 24 hours; a manual check must remain available and add a request.

With Cassette closed, wait for only the process started above and record the stable library state:

```sh
LIBRARY_DB="$XDG_DATA_HOME/io.github.atilla.cassette/library.sqlite3"
wait "$QUALIFICATION_APP_PID" || true
test -s "$LIBRARY_DB"
sqlite3 "$LIBRARY_DB" '.dump' > "$QUALIFICATION_ROOT/library-before.sql"
OLD_RUN_SHA="$(sha256sum "$RUN_IMAGE" | awk '{print $1}')"
NEW_DRAFT_SHA="$(sha256sum "$ORIGINALS/$APPIMAGE_NAME" | awk '{print $1}')"
```

Relaunch the old AppImage, check manually, then replace only the server's disposable download copy with the tampered payload. Confirm **Update and restart**. The download must fail signature verification, Cassette must remain usable, and no relaunch may occur:

```sh
cp "$QUALIFICATION_ROOT/tampered-payload/$APPIMAGE_NAME" "$SERVE_ROOT/$APPIMAGE_NAME"
```

After the failure, close Cassette and require the original executable and data to remain usable:

```sh
test "$(sha256sum "$RUN_IMAGE" | awk '{print $1}')" = "$OLD_RUN_SHA"
sqlite3 "$LIBRARY_DB" '.dump' > "$QUALIFICATION_ROOT/library-after-rejection.sql"
diff -u "$QUALIFICATION_ROOT/library-before.sql" "$QUALIFICATION_ROOT/library-after-rejection.sql"
(cd "$QUALIFICATION_ROOT/media" && sha256sum --check "$QUALIFICATION_ROOT/media.sha256")
```

Relaunch and confirm the version, theme/setting, library records, playlist, favorite, and playback still work. Restore the genuine server copy, run a new manual check, confirm the operation, observe download/install progress, and allow Cassette to relaunch:

```sh
cp "$ORIGINALS/$APPIMAGE_NAME" "$SERVE_ROOT/$APPIMAGE_NAME"
```

After the relaunched application is closed, verify that the writable old path now contains the exact draft bytes and that data did not change during the update:

```sh
test "$(sha256sum "$RUN_IMAGE" | awk '{print $1}')" = "$NEW_DRAFT_SHA"
sqlite3 "$LIBRARY_DB" '.dump' > "$QUALIFICATION_ROOT/library-after-update.sql"
diff -u "$QUALIFICATION_ROOT/library-before.sql" "$QUALIFICATION_ROOT/library-after-update.sql"
(cd "$QUALIFICATION_ROOT/media" && sha256sum --check "$QUALIFICATION_ROOT/media.sha256")
(cd "$ORIGINALS" && sha256sum --check SHA256SUMS)
```

Confirm Settings now reports version `0.1.0-beta.2`; the theme/setting, tracks, playlist, favorite, and playable media must remain present. WebKit cache and updater-local-storage files may legitimately change while checking and relaunching, so validate the saved settings in the UI rather than requiring byte-identical cache files.

Stop only the loopback server started above, retain the qualification directory until its hashes/results are copied into the release checklist, and leave the draft and production feed untouched:

```sh
kill "$HTTPS_SERVER_PID"
wait "$HTTPS_SERVER_PID" || true
unset SSL_CERT_FILE SSL_CERT_DIR XDG_DATA_HOME XDG_CONFIG_HOME XDG_CACHE_HOME XDG_STATE_HOME
test -z "$(git -C "$MAIN_ROOT" status --porcelain=v1)"
git -C "$MAIN_ROOT" diff --check
```

Do not publish at this point. Clean-machine AppImage/FUSE launch remains outstanding even if this host passes. DEB installation/removal on a clean Debian/Ubuntu machine and RPM installation/removal on a clean Fedora machine also remain outstanding. Use only isolated XDG data and disposable media on those machines; confirm notification-only updater behavior and data retention for both package types.

### 6. Deliberate publication and production-feed qualification

Publication requires separate authorization and must wait for every draft, end-to-end, and clean-machine gate above. Prepare final non-empty release notes, then the authorized publication command is:

```sh
gh release edit "$RELEASE_TAG" --repo "$CASSETTE_REPOSITORY" \
  --notes-file /path/to/reviewed-release-notes.md --draft=false --prerelease
```

That `release.published` event—not the tag and not the draft—starts `.github/workflows/publish-update-feed.yml`. Select the run by the exact commit/tag, wait for it, then download the deployed JSON and public assets anonymously into a new isolated directory:

```sh
FEED_RUN_ID=
for attempt in 1 2 3 4 5 6 7 8 9 10 11 12; do
  FEED_RUN_ID="$(gh run list --repo "$CASSETTE_REPOSITORY" \
    --workflow publish-update-feed.yml --event release --commit "$REVIEWED_SHA" --limit 10 \
    --json databaseId,headBranch,headSha \
    | jq -r --arg sha "$REVIEWED_SHA" \
      'map(select(.headSha == $sha)) | if length == 1 then .[0].databaseId elif length == 0 then empty else error("multiple matching feed runs") end')"
  test -n "$FEED_RUN_ID" && break
  sleep 5
done
test -n "$FEED_RUN_ID"
gh run watch "$FEED_RUN_ID" --repo "$CASSETTE_REPOSITORY" --exit-status
gh run view "$FEED_RUN_ID" --repo "$CASSETTE_REPOSITORY" \
  --json url,status,conclusion,headBranch,headSha,jobs

PUBLIC_CHECK="$(mktemp -d /tmp/cassette-3b-public-feed.XXXXXX)"
curl --disable --fail --location --silent --show-error --retry 10 --retry-all-errors \
  --header 'Authorization:' --dump-header "$PUBLIC_CHECK/headers.txt" \
  --output "$PUBLIC_CHECK/latest.json" \
  https://atilla-m.github.io/cassette/updates/beta/latest.json
grep -Eiq '^content-type:[[:space:]]*application/json([;[:space:]]|$)' "$PUBLIC_CHECK/headers.txt"
EXPECTED_ASSET_URL="https://github.com/atilla-m/cassette/releases/download/v0.1.0-beta.2/$APPIMAGE_NAME"
jq -e --arg url "$EXPECTED_ASSET_URL" '.version == "0.1.0-beta.2" and
  (.notes | length > 0) and (.pub_date | fromdateiso8601) and
  .platforms["linux-x86_64"].url == $url and
  (.platforms["linux-x86_64"].signature | length > 0)' "$PUBLIC_CHECK/latest.json"

curl --disable --fail --location --silent --show-error --header 'Authorization:' \
  --output "$PUBLIC_CHECK/$APPIMAGE_NAME" "$EXPECTED_ASSET_URL"
curl --disable --fail --location --silent --show-error --header 'Authorization:' \
  --output "$PUBLIC_CHECK/$SIGNATURE_NAME" "$EXPECTED_ASSET_URL.sig"
node "$MAIN_ROOT/scripts/verify-update.mjs" \
  "$PUBLIC_CHECK/$APPIMAGE_NAME" "$PUBLIC_CHECK/$SIGNATURE_NAME" "$RELEASE_VERSION"
test "$(sha256sum "$PUBLIC_CHECK/$APPIMAGE_NAME" | awk '{print $1}')" = \
  "$(sha256sum "$ORIGINALS/$APPIMAGE_NAME" | awk '{print $1}')"
test "$(sha256sum "$PUBLIC_CHECK/$SIGNATURE_NAME" | awk '{print $1}')" = \
  "$(sha256sum "$ORIGINALS/$SIGNATURE_NAME" | awk '{print $1}')"
node --input-type=module - "$PUBLIC_CHECK/latest.json" "$PUBLIC_CHECK/$SIGNATURE_NAME" <<'NODE'
import { readFileSync } from "node:fs";
const feed = JSON.parse(readFileSync(process.argv[2], "utf8"));
const signature = readFileSync(process.argv[3], "utf8").trim();
if (feed.platforms?.["linux-x86_64"]?.signature !== signature) {
  throw new Error("Published feed signature differs from the verified signature asset.");
}
NODE
```

Finally, launch the public `0.1.0-beta.2` AppImage directly with a new isolated XDG profile and normal system trust (unset the local test CA). A manual update check must reach and parse the production Pages feed and report `0.1.0-beta.2` up to date. This qualifies production client/feed discovery. The pre-publication `beta.1.e2e` exercise is the corrected older-to-newer installation proof; a later release can test the public channel from the genuine beta.2 client.

```sh
mkdir -p "$PUBLIC_CHECK/xdg/data" "$PUBLIC_CHECK/xdg/config" \
  "$PUBLIC_CHECK/xdg/cache" "$PUBLIC_CHECK/xdg/state"
chmod u+x "$PUBLIC_CHECK/$APPIMAGE_NAME"
env -u SSL_CERT_FILE -u SSL_CERT_DIR \
  XDG_DATA_HOME="$PUBLIC_CHECK/xdg/data" \
  XDG_CONFIG_HOME="$PUBLIC_CHECK/xdg/config" \
  XDG_CACHE_HOME="$PUBLIC_CHECK/xdg/cache" \
  XDG_STATE_HOME="$PUBLIC_CHECK/xdg/state" \
  "$PUBLIC_CHECK/$APPIMAGE_NAME"
```

Record the tag target, both Actions run URLs, all draft/public artifact hashes, feed JSON and headers, FUSE mount result, isolated data checks, and every warning. A generated signature or successful deployment alone is not evidence that update installation works.

## Release-side verification

The installed Tauri CLI and the [official CLI reference](https://v2.tauri.app/reference/cli/#signer) expose `signer sign` and `signer generate`, but no `signer verify`. Do not invent that command. `node scripts/verify-update.mjs APPIMAGE SIGNATURE VERSION` runs the artifact scanner's envelope/pairing checks and the standalone Rust helper in `scripts/update-verifier`. The helper uses the same pinned `minisign-verify` library, Tauri base64 envelope decoding, and `PublicKey::verify(..., true)` operation as tauri-plugin-updater 2.11.0. This is offline public-key verification of Tauri's format, not a new cryptographic format or a Tauri CLI subcommand. Neither helper accepts a private key. Positive verification and both tamper-rejection checks passed for the historical beta.1 and beta.2 pairs and must be repeated for each new signed candidate.

## Advancing the beta channel

The beta.3 project versions and exact tag/version/asset assertions are prepared in `release.yml`, `publish-update-feed.yml`, `build-release.mjs`, `generate-update-feed.mjs`, the artifact scanner, policy tests, and documentation. Retain the configured beta endpoint and stable artifact naming convention, keep tag URLs immutable, and land this publication workflow on the default branch before publishing the prerelease. The beta.3-only condition deliberately ignores the beta.1 draft, published beta.2 release, and every other release. Never move either historical tag, replace their assets, or edit the public feed by committing generated metadata to source.

## Key rotation, loss, and compromise

For planned rotation while the old key is still trusted: (1) create and back up the replacement key outside the repository in a separately authorized ceremony; (2) publish a bridge AppImage signed with the old key whose embedded configuration trusts the new public key; (3) retain an old-key bridge channel long enough for older clients to migrate; (4) test both client cohorts before publishing new-key-only artifacts. One static feed and one public key cannot simultaneously serve clients trusting different keys indefinitely. A separate bridge endpoint/channel or a documented manual download is necessary for clients that miss the bridge; design and qualify this before rotating, not during an emergency.

If the key is lost, restore a verified encrypted backup. Without a usable backup, existing clients cannot trust newly signed updates and must manually install a release with the replacement trust root. If compromised, suspend feed publication/distribution, rotate repository credentials as applicable, investigate exposure, and distribute a manually installed replacement through a verified channel. An old-key-signed bridge is not a safe recovery assumption after compromise. Never overwrite the only key backup or paste private material into chat/logs/source.
