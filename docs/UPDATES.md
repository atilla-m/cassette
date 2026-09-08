# Cassette Linux updater and signing handoff

## Current status

Checkpoint 3A prepares the Linux updater, signed-build path, and beta feed. Automatic updating is **not yet release-qualified**. A maintainer must generate and protect the real production keypair, commit only the public key, configure GitHub Actions secrets and Pages, and pass the end-to-end test described below before advertising automatic updates.

Tauri updater signatures are mandatory. Cassette must never publish an unsigned updater payload, disable signature verification, or substitute a placeholder public key.

## User-facing behavior

- Distributable signed release builds can check the stable beta feed. Credential-free package-validation builds report updates unavailable. Development builds never contact it, including when the manual check button is used.
- Automatic checks default to enabled and occur at most once every 24 hours, measured from the last automatic attempt, including failed attempts. The timestamp is persisted before the request and a per-session guard also applies. Manual checks remain available at any time after runtime detection. A failed background request is silent and does not delay startup or playback.
- A newer version is shown with its release notes. The user can choose **Later**; there is no countdown or forced installation.
- Only Linux with Tauri's embedded AppImage bundle marker can self-install. Native code canonicalizes `APPIMAGE`, `APPDIR`, and the current executable, requires a regular type-2 x86_64 ELF AppImage, checks `usr/bin/cassette`, `AppRun`, and `cassette.desktop` inside the AppDir, and requires read-only FUSE mount evidence tying the directory to that exact image. It also captures the image's device, inode, size, modification time, and change time and revalidates that identity before download and immediately before installation. Missing, replaced, stale, inconsistent, or escaping symlink evidence falls back to download-only. Extract-and-run and runtimes whose mount source cannot be identified also fall back safely; actual release runtime compatibility remains a checkpoint 3B test.
- **Update and restart** asks for explicit confirmation before download. There is no cancellation after confirmation in beta.1. Tauri verifies the downloaded bytes, native code reauthorizes the exact AppImage immediately before installation, then Cassette relaunches. Each successful check gets a new opaque native operation identifier, so a stale confirmation cannot install a same-version update that replaced the pending operation. No direct updater permissions are granted to the webview; it can request only that opaque pending operation, never supply an arbitrary URL, path, signature, or artifact.
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

Add the private key content to the GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY`. If the key is password-protected, add the password as `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; otherwise that second secret may be empty. The release workflow scopes both values only to the signed build step and never prints them.

## Public configuration checkpoint

After generating the real key, edit `src-tauri/tauri.conf.json` and add the Tauri updater plugin configuration. Set `pubkey` to the exact contents of `cassette-updater.key.pub` and configure one endpoint only:

`https://atilla-m.github.io/cassette/updates/beta/latest.json`

Do not point beta installations at GitHub's generic `/releases/latest` endpoint because prereleases may be excluded. Do not use a mutable release tag in an artifact URL. The tag-only release preflight rejects a missing or placeholder-like public key and rejects any other endpoint list.

The committed `src-tauri/tauri.updater.conf.json` overlay enables `bundle.createUpdaterArtifacts` only for `npm run release:linux:signed`. This is the sole supported AppImage-producing command. It requires the exact signed-updater feature/overlay arguments, configured public key and beta endpoint, and signing credentials before invoking Tauri. Ambient configuration overrides and alternate argument combinations are rejected.

`npm run release:linux` produces only DEB/RPM for credential-free package validation. Normal `npm run tauri build` defaults exclude AppImage; the npm Tauri wrapper refuses explicit AppImage/all-bundle and configuration-overlay bypasses. Ordinary Linux CI uploads only DEB/RPM under a package-validation name. There is no unsigned AppImage diagnostic build. Direct invocation of the dependency's CLI outside these guarded commands is unsupported and its outputs must never be distributed.

## Draft release and beta feed flow

1. The exact `v0.1.0-beta.1` tag starts `.github/workflows/release.yml`; ordinary pushes and pull requests cannot create a release.
2. The workflow validates the tag, versions, license metadata, real public-key configuration, and exact feed URL.
3. One Linux job builds DEB, RPM, AppImage, and `Cassette_0.1.0-beta.1_amd64.AppImage.sig`. Existing package verification, artifact-safety scans, signature-envelope safety checks, and public-key cryptographic verification of the exact AppImage/signature pair must pass.
4. Only those four exact current-version files are admitted to a draft prerelease. NSIS is not built or uploaded by this workflow. The workflow never publishes the draft.
5. A maintainer reviews and tests the downloaded draft artifacts, replaces the draft notes with final non-empty release notes, and intentionally publishes the prerelease.
6. Only the GitHub `release.published` event can start `.github/workflows/publish-update-feed.yml`. That workflow rejects drafts, non-prereleases, another tag, and any unexpected asset set.
7. The feed workflow downloads the exact published AppImage and `.sig`, copies both into one private stable snapshot, and repeats structural and cryptographic verification with the tagged public key against that snapshot. The signature serialized into `updates/beta/latest.json` is read from the same verified snapshot, so changing either downloaded source after verification cannot change the feed. The snapshot is removed after generation and is never uploaded. Generation refuses an unverifiable pair and does not commit metadata to source history. The JSON contains the exact version, immutable tagged AppImage URL, complete verified signature envelope, release notes, and publication time required by Tauri.

The Pages deployment job alone receives `pages: write` and `id-token: write`; source and release validation use `contents: read`. The draft-creation job alone receives `contents: write`.

In GitHub repository settings, manually configure **Pages → Build and deployment → Source: GitHub Actions**. Source changes cannot safely enable that repository setting. Do not publish the prerelease until the Pages workflow exists on the default branch and this setting is enabled.

## Data boundary

Updates replace application binaries only. Cassette's Linux data remains under `$XDG_DATA_HOME/io.github.atilla.cassette` (normally `~/.local/share/io.github.atilla.cassette`) and includes `library.sqlite3`, playlists, favorites, statistics, settings, and caches. Neither an update nor package uninstall should automatically erase that directory. Back up user data before any destructive manual removal.

## Required end-to-end checkpoint

Before enabling or advertising automatic updates:

1. Insert the genuine public key and endpoint in `src-tauri/tauri.conf.json` and configure both named Actions secrets as applicable.
2. Enable GitHub Pages with GitHub Actions as its source.
3. Use a temporary lower-version test build that trusts the same public key and points at a controlled beta test feed. Do not reuse or mutate a published production tag.
4. Build and sign a higher-version disposable AppImage through the same workflow path.
5. Verify no check occurs in `tauri dev`; manual development checks report unavailable.
6. Verify automatic rate limiting, silent up-to-date/background-failure behavior, notification, release notes, **Later**, confirmation, progress, signature rejection after deliberate payload tampering, successful install, and relaunch.
7. Verify DEB/RPM/unknown packaging never calls the installer or a package-manager command and opens the expected immutable GitHub release page.
8. Confirm `library.sqlite3`, settings, playlists, favorites, media, and caches retain their hashes/content across update and uninstall tests.

Record the test versions, artifact hashes, feed JSON, Actions run URLs, clean-machine details, and results in the release checklist. A generated signature alone is not evidence that update installation works.

## Release-side verification

The installed Tauri CLI and the [official CLI reference](https://v2.tauri.app/reference/cli/#signer) expose `signer sign` and `signer generate`, but no `signer verify`. Do not invent that command. `node scripts/verify-update.mjs APPIMAGE SIGNATURE VERSION` runs the artifact scanner's envelope/pairing checks and the standalone Rust helper in `scripts/update-verifier`. The helper uses the same pinned `minisign-verify` library, Tauri base64 envelope decoding, and `PublicKey::verify(..., true)` operation as tauri-plugin-updater 2.11.0. This is offline public-key verification of Tauri's format, not a new cryptographic format or a Tauri CLI subcommand. Neither helper accepts a private key. Positive verification with the production key and tampered-payload rejection remain pending checkpoint 3B.

## Advancing the beta channel

Before beta.2, deliberately update project versions and exact tag/version/asset assertions in `release.yml`, `publish-update-feed.yml`, `build-release.mjs`, `generate-update-feed.mjs`, the artifact scanner, and policy tests/documentation. Retain the same configured beta endpoint and stable artifact naming convention, and keep tag URLs immutable. Land the publication workflow on the default branch before publishing the new prerelease. The current beta.1-only condition deliberately ignores beta.2 until that work is complete. Never move beta.1's tag or edit the public feed by committing generated metadata to source.

## Key rotation, loss, and compromise

For planned rotation while the old key is still trusted: (1) create and back up the replacement key outside the repository in a separately authorized ceremony; (2) publish a bridge AppImage signed with the old key whose embedded configuration trusts the new public key; (3) retain an old-key bridge channel long enough for older clients to migrate; (4) test both client cohorts before publishing new-key-only artifacts. One static feed and one public key cannot simultaneously serve clients trusting different keys indefinitely. A separate bridge endpoint/channel or a documented manual download is necessary for clients that miss the bridge; design and qualify this before rotating, not during an emergency.

If the key is lost, restore a verified encrypted backup. Without a usable backup, existing clients cannot trust newly signed updates and must manually install a release with the replacement trust root. If compromised, suspend feed publication/distribution, rotate repository credentials as applicable, investigate exposure, and distribute a manually installed replacement through a verified channel. An old-key-signed bridge is not a safe recovery assumption after compromise. Never overwrite the only key backup or paste private material into chat/logs/source.
