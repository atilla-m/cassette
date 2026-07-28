# Cassette release checklist

This checklist prepares a draft; publishing is always a separate manual decision.

## 1. Source and metadata

- [ ] Confirm `main` is clean and contains only the original Cassette interface.
- [ ] Confirm the release branch was created directly from the intended `main` commit.
- [ ] Confirm the release contains only the stable original/Legacy interface, with no alternate interface components, selectors, or persisted interface settings.
- [ ] Review every uncommitted change before committing it intentionally.
- [ ] Verify `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` all say `0.1.0`.
- [ ] Verify product name `Cassette`, executable `cassette`, and bundle identifier `io.github.atilla.cassette`.
- [ ] Choose an approved license, add `LICENSE`, and update package/Cargo metadata. Do not publish while the project is `UNLICENSED`.
- [ ] Replace the stock Tauri/Svelte icons with approved Cassette artwork and regenerate every required icon size.
- [ ] Check authors, description, homepage, repository, category, and release notes.
- [ ] Confirm there are no signing identities, fake signatures, or secrets in source.

## 2. Automated validation

- [ ] Run the complete Linux CI job on Ubuntu 22.04.
- [ ] Run the complete Windows CI job on `windows-latest` with stable `x86_64-pc-windows-msvc`.
- [ ] Confirm both jobs run `npm ci`, frontend build/check, Cargo test/check, and compile the Tauri application.
- [ ] Confirm ignored real-media fixture tests were not run.
- [ ] Validate workflow YAML and inspect action logs for warnings.
- [ ] Confirm the draft workflow produces AppImage, RPM, DEB, NSIS setup EXE, and MSI artifacts without publishing on `workflow_dispatch`.

## 3. Local Fedora test

- [ ] Install `patchelf` before testing AppImage bundling; the GStreamer bundling plugin requires it.
- [ ] Run `npm ci`.
- [ ] Run `npm run build`.
- [ ] Run `npm run check`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `npm run tauri build`.
- [ ] Run `git diff --check`.
- [ ] Install the generated RPM and test uninstall.
- [ ] Install the generated DEB in a clean compatible VM/container and test uninstall.
- [ ] Run the AppImage on a clean Linux installation, not only the build machine.
- [ ] Inspect AppImage contents and runtime logs to see which WebKitGTK/GStreamer libraries and plugins are actually included.
- [ ] Do not call the AppImage fully portable until the clean-install test succeeds.
- [ ] If Fedora's newer ELF sections are incompatible with `linuxdeploy`'s bundled `strip`, retry the local diagnostic build with `NO_STRIP=1 npm run tauri build -- --bundles appimage`; use the Ubuntu 22.04 workflow artifact for release review.
- [ ] Test Albums startup, Album Detail, Artists, Artist Detail, Genres, Genre Detail, Songs, Playlists, Queue, Lyrics, Stats, Settings, and the bottom player.
- [ ] Test all five themes.
- [ ] Test Linux notifications with and without `notify-send`.
- [ ] Test CD ripping only on a disposable disc/output location with `cdparanoia`, `flac`, and `libdiscid`.
- [ ] Test DVD/video only with legally usable disposable media and the optional Linux tools installed.

## 4. Clean Windows 10/11 VM tests

Perform all checks on clean x86_64 VMs, not only on a GitHub Actions runner.

- [ ] Test Windows 10 with current updates.
- [ ] Test Windows 11 with current updates.
- [ ] Verify the documented missing-GStreamer behavior before installing GStreamer.
- [ ] Install the official GStreamer 1.26.11 MSVC x86_64 runtime and ensure its `bin` directory is in `PATH`.
- [ ] Verify Cassette starts without development files installed.
- [ ] Install/uninstall the NSIS setup as a standard user.
- [ ] Confirm NSIS is per-user, creates a Start Menu shortcut and uninstall entry, does not require admin for Cassette, and does not enable automatic startup.
- [ ] Install/uninstall the MSI and decide whether it is reliable enough to include.
- [ ] Verify normal WebView2 bootstrapper behavior with WebView2 present and absent.
- [ ] Scan a library on `C:`.
- [ ] Scan a library on another drive.
- [ ] Scan Unicode folders and filenames, including non-Latin characters.
- [ ] Test path containment with a sibling folder sharing the library root's name prefix.
- [ ] Play real FLAC, MP3, OGG/Vorbis, Opus, WAV, and M4A/AAC files.
- [ ] Verify duration discovery, artwork, pause/resume, seek, previous/next, auto-advance, shuffle, repeat, volume, and current-track state.
- [ ] Test Queue order and manipulation.
- [ ] Test playlists and favorites across restart.
- [ ] Test local lyrics and online lyrics where networking is available.
- [ ] Edit FLAC tags on a disposable copy; verify metadata, audio decoding, cover art, cache update, and absence of `.cassette-*` sidecars.
- [ ] Interrupt a disposable FLAC write in a controlled test and verify rollback remains safe.
- [ ] Confirm non-FLAC formats remain read-only.
- [ ] Confirm Linux notification and MPRIS controls are unavailable without errors.
- [ ] Confirm CD ripping and DVD/video navigation are hidden or clearly unavailable.
- [ ] Verify paths, cache, covers, lyrics, and playlists survive restart.

## 5. Privacy and package inspection

Inspect source, built frontend, Linux bundles, Windows installers, and extracted package contents.

- [ ] Search for `/home/atilla`, the developer username, personal music paths, and Windows user-profile paths.
- [ ] Search for databases, cached covers, cached lyrics, test audio/video, source fixtures, logs, `.env` files, API keys, tokens, certificates, and temporary tag-editor files.
- [ ] Confirm no raw `target`, `node_modules`, `.git`, personal nested repositories, or recovery files are packaged.
- [ ] Confirm no alternate interface branch names, components, or settings are present.
- [ ] Inspect installer file lists rather than relying only on ignore rules.
- [ ] Verify GStreamer development files are not in Windows installers.
- [ ] Record checksums for every release asset after downloading it from the draft.

## 6. Create and review the draft

- [ ] From clean `main`, create the annotated tag intentionally: `git tag -a v0.1.0 -m "Cassette 0.1.0"`.
- [ ] Push only the reviewed tag: `git push origin v0.1.0`.
- [ ] Wait for `.github/workflows/release.yml` to finish.
- [ ] Confirm there is exactly one **draft** GitHub Release.
- [ ] Confirm the draft contains only the actual AppImage, RPM, DEB, NSIS setup EXE, and MSI generated filenames.
- [ ] Download every installer from the draft rather than testing only runner outputs.
- [ ] Repeat install, launch, playback, FLAC edit, and uninstall smoke tests with the downloaded files.
- [ ] Review release notes, limitations, GStreamer requirement, unsigned SmartScreen warning, license, third-party notices, and checksums.
- [ ] Manually publish only after every blocker is closed.

## 7. Remove a bad draft/tag without rewriting main

If a tag-triggered draft is wrong, do not reset, rebase, force-push, or rewrite `main`.

1. Delete the draft release:

   ```sh
   gh release delete v0.1.0 --yes
   ```

2. Delete the remote tag:

   ```sh
   git push origin :refs/tags/v0.1.0
   ```

3. Delete the local tag:

   ```sh
   git tag -d v0.1.0
   ```

4. Fix the release branch through normal reviewed commits, merge normally, then create and push a new annotated tag only when ready.

Deleting a draft does not delete its tag automatically. Deleting a tag does not alter branch history.
