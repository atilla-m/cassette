# Release artifact safety

Cassette release builds must use `scripts/build-release.mjs`, through `npm run release:linux` for DEB/RPM validation, `npm run release:linux:signed` for distributable Linux bundles, or `npm run release:windows` for deferred Windows qualification. The wrapper supplies Cargo path-remapping flags as encoded arguments and matching C/C++ compiler prefix maps, so workspace, Cargo-home, and developer-home paths are replaced in Rust and native dependency sources without platform-specific shell invocation. It disables ambient `ccache` for release compilation so correctness cannot depend on a developer- or runner-specific cache directory; Cargo's normal dependency caching remains available. The Cargo release profile strips native symbols. Debug and development profiles are unchanged.

Run `scripts/audit-release-artifacts.py` against exact current-version paths after building. The scanner supports native executables, DEB, RPM, AppImage/AppDir, NSIS via 7-Zip, MSI via `lessmsi` or Windows administrative extraction, and built frontend directories. It verifies release version, x86_64 architecture, expected package metadata and files, and byte-matched GPL license payloads where the format supports those checks.

The scanner fails on:

- the current build user's home-directory or repository/workspace absolute paths (third-party system-library strings naming their own upstream builders are not treated as Cassette developer paths);
- PEM private keys and recognizable AWS, GitHub, or Slack credential forms in app-owned executables/resources and text/frontend files, plus credential-like filenames throughout the payload;
- packaged databases, audio/video media, cover-art, thumbnail, or lyrics-cache files;
- development HTTP/WebSocket URLs in frontend assets or the production CSP;
- missing or mismatched executable, desktop entry, icon configuration, license, version, architecture, or Linux dependency metadata;
- a GStreamer runtime accidentally embedded in Windows installer payloads;
- any `libwayland-client.so*` file or symlink in an AppDir or extracted AppImage.

## Narrow native URL exception

Tauri can retain its configured development endpoint as inert native configuration text even when the release executable embeds production frontend assets. The scanner therefore permits only the exact strings `http://localhost:1420` and `ws://localhost:1420`, and only in the native Cassette executable. It reports how many such exceptions it observed. The exception does not apply to native dependencies, built HTML, CSS, JavaScript, JSON, source maps, or the production CSP; any matching development endpoint there is fatal.

Do not broaden this exception to make a failing artifact pass. Investigate any new native URL separately.

## AppImage provisioning and portability

AppImage generation requires a real `patchelf` available through the normal runner `PATH`; `NO_STRIP=1` is not a substitute. The signed AppImage workflow uses the pinned Ubuntu 24.04 runner and refuses a WebKitGTK runtime older than 2.52 before invoking the normal release wrapper. Ordinary Linux CI remains on Ubuntu 22.04 and continues to validate DEB/RPM packaging independently.

The AppImage keeps Tauri's media-framework bundling enabled and includes Cassette's GPL license resource. The workflow passes an explicit plugin directory to the linuxdeploy GStreamer plugin and excludes only `libgstneonhttpsrc.so`: Cassette supplies validated local-file URIs to GStreamer and does not expose that network source, whose Ubuntu 24.04 binary also contains a scanner-forbidden development localhost URL. DEB/RPM dependencies and all local playback plugin families remain unchanged.

The build and qualification target for this bundle is x86_64 Ubuntu 24.04 or a distribution with an equivalent-or-newer runtime; moving the packaging host from Ubuntu 22.04 trades older-distribution compatibility for the newer bundled WebKitGTK runtime. The diagnostic Ubuntu 24.04 AppDir required at most `GLIBC_2.38`, but the completed signed artifact must be measured and tested on clean hosts before that observation can be treated as its minimum runtime requirement or the AppImage can be described as fully portable. The artifact audit rejects databases, user media, cached lyrics/artwork, and user GStreamer cache material if any is copied into the AppDir.

### Host Wayland client, including X11 launches

`src-tauri/.appimageignore` uses appimagetool's [supported exclusion file](https://github.com/AppImage/appimagetool/blob/main/README.md). Tauri runs packaging from `src-tauri`; appimagetool applies this file with SquashFS wildcard exclusions. The patterns omit only `libwayland-client.so*` (unversioned name, SONAME, versioned files and symlinks) from standard and multiarch library locations. They do not copy host libraries, select a GPU, alter AppRun, or change DEB/RPM/Windows packaging. The intermediate linuxdeploy AppDir can still contain the library: the exclusion is applied when constructing the final filesystem. Audit the extracted final payload, not that intermediate directory, for this gate. The scanner independently rejects reintroduction anywhere in the payload, including dangling symlinks.

On the Fedora 44 qualification laptop, the bundled Wayland client shadowed the host library through AppRun's library search path. Loading host Mesa EGL then failed to resolve `wl_fixes_interface`, `wl_display_create_queue_with_name`, and `wl_display_dispatch_queue_timeout`. This mattered even with `GDK_BACKEND=x11`, because the host EGL library also links Wayland. The failing AppDir used NVIDIA; omitting only the bundled Wayland client allowed host Mesa/Wayland resolution and AMD rendering while retaining Ubuntu 24's bundled WebKitGTK 2.52.6/GTK runtime. The user confirmed smooth Albums scrolling in that one-library diagnostic candidate. Removing artwork had not helped; CPU, accessibility-call latency, and XDamage counts were not treated as frame-rate evidence.

This is a demonstrated correction on that host, not a compatibility guarantee for other drivers, distributions, or Wayland sessions. Host Wayland-client availability and compatibility, clean-machine playback, and fresh signed/FUSE-mounted qualification remain required. The diagnostic still reported a FluidSynth MIDI plugin load warning involving PipeWire JACK's `pw_log_topic_register`, and a duplicate local LV2 Ratatouille plugin warning was observed. No additional plugins were removed to hide these warnings. Supported-format playback (including plugin-dependent formats) remains an explicit release gate.

The sole supported AppImage build is `npm run release:linux:signed`; ordinary `release:linux` and Linux CI validate DEB/RPM only. Default Tauri targets exclude AppImage and the npm wrapper refuses explicit AppImage/config-overlay bypasses. There is no unsigned AppImage diagnostic artifact.

The scanner recognizes the exact adjacent `Cassette_0.1.0-beta.1_amd64.AppImage.sig` as a detached signature, not an executable. It rejects empty/oversized files, noncanonical base64, malformed Minisign packets, unexpected comment/credential material, wrong versions/names, and absent or symlinked pairs. It checks the signed trusted filename. These structural checks do not establish authenticity.

`node scripts/verify-update.mjs APPIMAGE SIGNATURE VERSION` additionally verifies the exact payload using the configured public key and the same pinned Minisign verifier/envelope as Tauri's updater. Tauri's CLI has no `signer verify` subcommand. The release workflow must pass this helper before uploading assets or creating a draft. Feed generation downloads and reverifies both published files before copying the complete signature into JSON. Neither verification step receives signing secrets. The AppImage's package scan remains separately mandatory. The original beta.1 pair passed real signature and tamper checks, but it does not contain the runtime/detector remediation. Repeat those checks for the next signed candidate; installation remains pending checkpoint 3B. See [UPDATES.md](UPDATES.md).
