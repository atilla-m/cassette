# Cassette

Cassette is a private, local-first desktop music library and player. It scans folders you choose, keeps its library cache locally, and provides albums, artists, genres, songs, playlists, queue management, lyrics, statistics, themes, and FLAC metadata editing.

Cassette 0.1.0 is being prepared for Linux x86_64 and Windows 10/11 x86_64. No release has been published yet, so there are no download links in this document.

## Download

When the first draft has passed the release checklist, the GitHub Release will contain the actual generated filenames for:

- Linux: AppImage, RPM, and DEB
- Windows: NSIS `-setup.exe` and MSI

The Windows NSIS setup executable is the recommended Windows download. It installs per user, does not require administrator access for Cassette itself, creates normal Start Menu and uninstall entries, and does not configure automatic startup. The MSI is an alternative for users who specifically prefer Windows Installer.

The first Windows build is beta and unsigned. Windows SmartScreen may warn about an unrecognized publisher; inspect the file and its GitHub source/release provenance, and do not bypass organizational security policy.

## Windows 10/11 x86_64

Cassette uses the MSVC build of GStreamer. The first installers are **not self-contained**: install the official GStreamer **1.26.11 MSVC x86_64 runtime** before starting Cassette:

1. Download `gstreamer-1.0-msvc-x86_64-1.26.11.msi` from the [official GStreamer 1.26.11 MSVC directory](https://gstreamer.freedesktop.org/pkg/windows/1.26.11/msvc/).
2. Install the complete runtime, keeping its directory structure intact.
3. Ensure the runtime's `bin` directory is visible in `PATH` before launching Cassette. With the release-build layout this is `C:\gstreamer\1.0\msvc_x86_64\bin`; use the actual directory if you installed it elsewhere.

This setup is expected to support local library scanning, SQLite caching, artwork, audio playback, seek, queue, playlists, favorites, lyrics, themes, and FLAC tag editing. Windows release testing must still verify FLAC, MP3, OGG/Vorbis, Opus, WAV, and M4A/AAC playback on clean Windows 10 and 11 virtual machines.

Windows 0.1.0 limitations:

- Linux desktop notifications and MPRIS are unavailable.
- CD detection/ripping and DVD import/playback are Linux-only.
- The GStreamer runtime is not bundled; missing runtime files can prevent Windows from starting the dynamically linked application. This is why the runtime and `PATH` steps above are mandatory.
- The installers are unsigned.

The WebView2 download bootstrapper uses Tauri's normal setup behavior and may require network access if WebView2 is missing.

## Linux x86_64

The AppImage is the simplest download to try:

```sh
chmod +x Cassette*.AppImage
./Cassette*.AppImage
```

Install a downloaded RPM on Fedora:

```sh
sudo dnf install ./Cassette*.rpm
```

Install a downloaded DEB on Ubuntu or Debian:

```sh
sudo apt install ./Cassette*.deb
```

Cassette needs WebKitGTK and GStreamer with the base, good, bad, ugly, and libav plugin families for the supported audio formats. RPM and DEB metadata declare the runtime package dependencies. The AppImage build enables Tauri's media-framework bundling, but it must not be treated as fully portable until it has been tested on a clean Linux installation.

Optional Linux features use external tools:

- Notifications: `notify-send` (`libnotify-bin` on Debian/Ubuntu, `libnotify` on Fedora)
- CD ripping: `cdparanoia`, `flac`, and `libdiscid`
- DVD/video: `lsdvd`, `ffmpeg`, `ffprobe`, and `mpv`

Missing optional tools produce an error and do not affect the core music library.

## FLAC tag editor

FLAC is the only writable tag format in Cassette 0.1.0. Editing tags modifies metadata inside the actual audio file. Keep backups and test important workflows on disposable copies first. MP3, OGG, Opus, WAV, and M4A tags remain read-only because they have not received equivalent real-file rollback testing.

## Development

Requirements include Node.js 22, Rust stable, Tauri's native platform dependencies, and GStreamer development files.

```sh
npm ci
npm run build
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

Ignored real-media fixture tests are intentionally excluded from normal CI.

## Release and license status

The release process is documented in [docs/RELEASING.md](docs/RELEASING.md), with GStreamer details in [docs/GSTREAMER-WINDOWS.md](docs/GSTREAMER-WINDOWS.md).

This repository currently has no `LICENSE` file. The package metadata is therefore marked `UNLICENSED`; selecting and adding a license is a blocker before public binary distribution. The current desktop icons are inherited Tauri template artwork and should also be replaced with approved Cassette artwork before the first public release.
