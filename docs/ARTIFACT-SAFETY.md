# Release artifact safety

Cassette release builds must use `scripts/build-release.mjs`, normally through `npm run release:linux` or `npm run release:windows`. The wrapper supplies Cargo path-remapping flags as encoded arguments, so workspace, Cargo-home, and developer-home paths are replaced without platform-specific shell quoting. It disables ambient `ccache` for release compilation so correctness cannot depend on a developer- or runner-specific cache directory; Cargo's normal dependency caching remains available. The Cargo release profile strips native symbols. Debug and development profiles are unchanged.

Run `scripts/audit-release-artifacts.py` against exact current-version paths after building. The scanner supports native executables, DEB, RPM, AppImage/AppDir, NSIS via 7-Zip, MSI via `lessmsi` or Windows administrative extraction, and built frontend directories. It verifies release version, x86_64 architecture, expected package metadata and files, and byte-matched GPL license payloads where the format supports those checks.

The scanner fails on:

- the current build user's home-directory or repository/workspace absolute paths (third-party system-library strings naming their own upstream builders are not treated as Cassette developer paths);
- PEM private keys and recognizable AWS, GitHub, or Slack credential forms in app-owned executables/resources and text/frontend files, plus credential-like filenames throughout the payload;
- packaged databases, audio/video media, cover-art, thumbnail, or lyrics-cache files;
- development HTTP/WebSocket URLs in frontend assets or the production CSP;
- missing or mismatched executable, desktop entry, icon configuration, license, version, architecture, or Linux dependency metadata;
- a GStreamer runtime accidentally embedded in Windows installer payloads.

## Narrow native URL exception

Tauri can retain its configured development endpoint as inert native configuration text even when the release executable embeds production frontend assets. The scanner therefore permits only the exact strings `http://localhost:1420` and `ws://localhost:1420`, and only in the native Cassette executable. It reports how many such exceptions it observed. The exception does not apply to native dependencies, built HTML, CSS, JavaScript, JSON, source maps, or the production CSP; any matching development endpoint there is fatal.

Do not broaden this exception to make a failing artifact pass. Investigate any new native URL separately.

## AppImage provisioning and portability

AppImage generation requires a real `patchelf` available through the normal runner `PATH`; `NO_STRIP=1` is not a substitute. Linux CI installs `patchelf`, linuxdeploy support tools, and GStreamer/WebKitGTK build dependencies from the pinned Ubuntu runner repositories before invoking the normal release wrapper.

The AppImage keeps Tauri's media-framework bundling enabled and includes Cassette's GPL license resource. It must still be tested on a clean supported Linux installation before being described as fully portable. The artifact audit rejects databases, user media, cached lyrics/artwork, and user GStreamer cache material if any is copied into the AppDir.
