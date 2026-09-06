# Cassette v0.1.0-beta.1 release checklist

This checklist prepares a draft for the cross-platform beta. Publishing is always a separate manual decision. Linux x86_64 is the primary tested platform; Windows 10/11 x86_64 remains beta and requires the documented external GStreamer runtime.

## 1. Source, scope, and metadata

- [ ] Confirm the release branch was created from the intended source commit and every change has been reviewed.
- [ ] Verify `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` all resolve to `0.1.0-beta.1`.
- [ ] Verify generated About/package metadata and runtime User-Agent strings report `0.1.0-beta.1`.
- [ ] Verify product name `Cassette`, executable `cassette`, bundle identifier `io.github.atilla.cassette`, and license `GPL-3.0-or-later`.
- [ ] Confirm the beta contains the current interface and all five themes: Cassette Teal, Rose Noir, Royal Gold, Glacier, and Obsidian.
- [ ] Confirm there is no interface selector and no Modern UI implementation or persisted interface setting. Modern UI is future work, not a beta promise.
- [ ] Confirm `ENABLE_EXPERIMENTAL_VIDEOS` is `false` and video/DVD navigation remains hidden. Existing backend code may remain, but video/DVD is unsupported in this beta.
- [ ] Confirm playback, scanning, queues, shuffle/repeat, playlists, favorites, lyrics, FLAC editing, copied-FLAC safety, CD ripping, themes, MPRIS, notifications, and database behavior remain unchanged.
- [ ] Accept the stock Tauri icons as a documented `v0.1.0-beta.1` limitation; replacement artwork is not a release blocker for this beta.
- [ ] Check authors, description, homepage, repository, category, release notes, and bug-report link.
- [ ] Confirm there are no signing identities, fake signatures, credentials, or secrets in source.

## 2. License and dependency payload

- [ ] Verify the repository `LICENSE` is the unmodified full GPLv3 text and metadata uses `GPL-3.0-or-later`.
- [ ] Verify `bundle.licenseFile` points to `../LICENSE` and `bundle.resources` maps that same file to the stable bundle resource path `LICENSE`.
- [ ] Inspect, without installing, the DEB and RPM file lists. Confirm both contain `$RESOURCE/LICENSE`, DEB contains `/usr/share/doc/cassette/LICENSE`, and RPM contains `/usr/share/licenses/cassette/LICENSE`, all sourced from the top-level `LICENSE`.
- [ ] Compare SHA-256 for each packaged `LICENSE` with the repository `LICENSE`.
- [ ] Confirm Tauri/Cargo/npm metadata still identifies the license as `GPL-3.0-or-later` and the RPM header reports it. Debian control has no standard SPDX license field, so verify the DEB license through its byte-matched payload and source/package metadata.
- [ ] Confirm required GStreamer base, good, bad/codec, ugly/restricted-codec, and libav plugin families remain package dependencies.
- [ ] Confirm `libnotify-bin` (DEB) and `libnotify` (RPM) are recommendations rather than mandatory dependencies. Cassette invokes optional `notify-send` and handles its absence.
- [ ] Record the complete generated dependency metadata, including Tauri's automatically detected WebKitGTK/GTK requirements.

## 3. Checkpoint 1 Linux validation

Do not run the 13 ignored real-media fixture tests and do not install generated packages during this checkpoint.

- [ ] Run `npm run build`.
- [ ] Run `npm run check`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `npm run tauri build -- --bundles deb,rpm`.
- [ ] Run `git diff --check`.
- [ ] Record exact DEB/RPM paths, sizes, versions, architectures, dependency metadata, and every packaging warning.
- [ ] Extract or inspect both packages without `sudo` or installation.
- [ ] Confirm package file lists contain only expected executable, desktop entry, stock icons, and license resources.
- [ ] Search package contents for personal absolute paths, databases, copied media, credentials, development URLs, debug/recovery overrides, source trees, and other release hazards.

Checkpoint 1 does not validate or repair AppImage, CI, NSIS, or MSI packaging.

Release builds and package audits use the portable wrappers documented in [ARTIFACT-SAFETY.md](ARTIFACT-SAFETY.md). Never upload a bundle that has not passed the scanner from an exact current-version path.

## 4. Checkpoint 2 and clean-machine validation

- [ ] Run the complete Linux CI job on Ubuntu 22.04.
- [ ] Run the complete Windows CI job on pinned `windows-2022` with stable `x86_64-pc-windows-msvc`.
- [ ] Confirm both jobs run `npm ci`, frontend build/check, Cargo test/check, and compile the Tauri application without running ignored real-media tests.
- [ ] Verify AppImage generation and launch on a clean Linux installation. Do not call it portable before this succeeds.
- [ ] Verify unsigned NSIS and MSI generation, install, launch, and uninstall on clean Windows 10 and Windows 11 x86_64 VMs.
- [ ] Confirm the tag-only draft workflow creates the expected artifacts as a draft prerelease and never runs for ordinary pushes or pull requests.
- [ ] Review workflow logs and packaging warnings.

For local AppImage diagnostics, provision `patchelf` and the documented linuxdeploy support tools first, then run `npm run release:linux`. Do not use `NO_STRIP=1` as a substitute for missing packaging tools. Release decisions must use a normally built, scanned artifact from a clean supported host.

## 5. Linux functional and uninstall tests

Use disposable library data and media for release testing.

- [ ] Test Albums startup, Album Detail, Artists, Artist Detail, Genres, Genre Detail, Songs, Playlists, Queue, Lyrics, Stats, Settings, and the bottom player.
- [ ] Test all five themes and verify their saved behavior.
- [ ] Test playback, seek, previous/next, auto-advance, shuffle, repeat, volume, queue manipulation, playlists, favorites, lyrics, and restart persistence.
- [ ] Test notifications with and without optional `notify-send`; absence must not prevent launch or core library use.
- [ ] Test CD ripping only with a disposable disc/output location and `cdparanoia`, `flac`, and `libdiscid`.
- [ ] Confirm experimental video/DVD UI remains disabled; do not enable or test hidden backend features for this beta.
- [ ] Install and remove the generated RPM on a clean Fedora system.
- [ ] Install and remove the generated DEB on a clean compatible Debian/Ubuntu system.
- [ ] Confirm package removal leaves `$XDG_DATA_HOME/io.github.atilla.cassette` (default `~/.local/share/io.github.atilla.cassette`) and its `library.sqlite3` database intact unless the user explicitly removes that data.

Document the user-facing uninstall commands as `sudo dnf remove cassette` for RPM and `sudo apt remove cassette` for DEB. Explain that users may manually remove the application-data directory after backing it up.

## 6. Clean Windows 10/11 VM tests

Perform all checks on clean x86_64 VMs, not only on a CI runner. Follow [GSTREAMER-WINDOWS.md](GSTREAMER-WINDOWS.md).

- [ ] Verify the documented missing-GStreamer behavior before installing GStreamer.
- [ ] Install the official GStreamer 1.26.11 MSVC x86_64 runtime and make its `bin` directory available in `PATH`.
- [ ] Verify Cassette starts without development files installed.
- [ ] Install/uninstall the unsigned NSIS setup as a standard user and verify its per-user Start Menu and uninstall entries.
- [ ] Install/uninstall the unsigned MSI and decide whether it is reliable enough to include.
- [ ] Verify normal WebView2 bootstrapper behavior with WebView2 present and absent.
- [ ] Scan disposable libraries on `C:`, another drive, and Unicode paths.
- [ ] Play representative FLAC, MP3, OGG/Vorbis, Opus, WAV, and M4A/AAC files.
- [ ] Test queue, playlists, favorites, lyrics, themes, and restart persistence.
- [ ] Edit FLAC tags only on disposable copies; verify rollback safety and confirm other formats remain read-only.
- [ ] Confirm Linux notifications, MPRIS, and CD ripping are unavailable without application errors.
- [ ] Confirm experimental video/DVD navigation is absent.

## 7. Privacy and package inspection

Inspect source, built frontend, Linux bundles, Windows installers, and extracted package contents.

- [ ] Search for `/home/atilla`, the developer username, personal music paths, and Windows user-profile paths.
- [ ] Search for databases, cached covers, cached lyrics, test audio/video, source fixtures, logs, `.env` files, API keys, tokens, certificates, and temporary tag-editor files.
- [ ] Confirm no raw `target`, `node_modules`, `.git`, personal nested repositories, recovery files, or development URLs are packaged.
- [ ] Confirm no interface selector, Modern UI implementation, or interface setting is present.
- [ ] Inspect installer file lists rather than relying only on ignore rules.
- [ ] Verify GStreamer development files are not in Windows installers.
- [ ] Record checksums for every release asset after downloading it from the draft.

## 8. Create and review the draft

Run these steps only after both checkpoints and clean-machine tests pass. Tagging, pushing, and publishing are explicitly outside checkpoint 1.

- [ ] From reviewed, clean `main`, create the annotated tag intentionally: `git tag -a v0.1.0-beta.1 -m "Cassette 0.1.0-beta.1"`.
- [ ] Push only the reviewed tag: `git push origin v0.1.0-beta.1`.
- [ ] Wait for `.github/workflows/release.yml` to finish.
- [ ] Confirm there is exactly one draft GitHub Release.
- [ ] Confirm the draft contains only formats that actually passed validation; do not claim AppImage or Windows artifacts passed merely because they were configured.
- [ ] Download every draft installer rather than testing only runner outputs.
- [ ] Repeat install, launch, playback, FLAC edit, and uninstall smoke tests with downloaded files.
- [ ] Review beta warning, platform limitations, dependency requirements, stock-icon status, unsigned-package warnings, license, third-party notices, and checksums.
- [ ] Manually publish only after every blocker is closed.

## 9. Remove a bad draft/tag without rewriting main

If a tag-triggered draft is wrong, do not reset, rebase, force-push, or rewrite `main`.

1. Delete the draft release:

   ```sh
   gh release delete v0.1.0-beta.1 --yes
   ```

2. Delete the remote tag:

   ```sh
   git push origin :refs/tags/v0.1.0-beta.1
   ```

3. Delete the local tag:

   ```sh
   git tag -d v0.1.0-beta.1
   ```

4. Fix the release branch through normal reviewed commits, merge normally, then create and push a new annotated tag only when ready.

Deleting a draft does not delete its tag automatically. Deleting a tag does not alter branch history.
