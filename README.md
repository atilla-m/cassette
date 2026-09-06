# Cassette

Cassette is a private, local-first desktop music library and player. It scans folders you choose, keeps its library cache locally, and provides albums, artists, genres, songs, playlists, queue management, lyrics, statistics, themes, and safe FLAC metadata editing.

## v0.1.0-beta.1 status

Cassette `0.1.0-beta.1` is a cross-platform beta. Linux x86_64 is the primary tested platform. Windows 10/11 x86_64 support remains beta, requires the external GStreamer runtime described below, and still needs clean-machine installer verification. No release has been published yet, so there are no download links in this document.

This beta includes the current Cassette interface and all five existing themes: Cassette Teal, Rose Noir, Royal Gold, Glacier, and Obsidian. Playback, scanning, the queue, shuffle and repeat, playlists, favorites, lyrics, FLAC tag editing, CD ripping, MPRIS, and notifications retain their current behavior.

Modern UI work is reserved for a future release. This beta does not include a Modern/Legacy interface switch or promise the rejected Modern design. Experimental video and DVD functionality is disabled and unsupported in this beta; hidden backend code and tools such as `lsdvd`, `ffmpeg`, `ffprobe`, and `mpv` are not part of the beta runtime contract.

The configured package formats are DEB and RPM for Linux, AppImage for Linux, and NSIS setup EXE and MSI for Windows. Checkpoint 1 validates only DEB and RPM. Do not treat AppImage or Windows installer generation as passing until checkpoint 2 and clean-machine testing verify them.

The beta uses the stock Tauri desktop icons as an accepted known limitation. Locally generated packages and planned Windows installers are unsigned; package managers, desktop security tools, or Windows SmartScreen may warn about an unrecognized publisher.

## Linux x86_64

Install a downloaded RPM on Fedora:

```sh
sudo dnf install ./Cassette*.rpm
```

Install a downloaded DEB on Ubuntu or Debian:

```sh
sudo apt install ./Cassette*.deb
```

Cassette requires WebKitGTK and GStreamer with the base, good, bad/codec, ugly/restricted-codec, and libav plugin families for the documented FLAC, MP3, OGG/Vorbis, Opus, WAV, and M4A/AAC playback. DEB and RPM metadata declare these required runtime families; the package manager resolves the platform-specific GUI libraries.

Linux notifications call the external `notify-send` command. The package recommends `libnotify-bin` on Debian/Ubuntu or `libnotify` on Fedora, but it is optional: when absent, notification attempts report that notification support is unavailable without preventing Cassette from launching or using the music library.

CD detection and ripping remain an optional Linux feature and use `cdparanoia`, `flac`, and `libdiscid`. Missing CD tools produce a feature-specific error and do not affect the core music library. Experimental video/DVD functionality is disabled for this beta, so its external tools are neither required nor advertised as available beta features.

### Data and uninstall

Cassette stores Linux application data under `$XDG_DATA_HOME/io.github.atilla.cassette`, defaulting to `~/.local/share/io.github.atilla.cassette`. The library database is `library.sqlite3` in that directory; cached cover art and other application-managed data are stored alongside it.

Remove the installed package with the matching package manager:

```sh
sudo apt remove cassette
sudo dnf remove cassette
```

Run only the command appropriate for the installed package. Package removal does not automatically delete the library database or other user data. After backing up anything you want to keep, optional manual data removal is:

```sh
rm -r -- "${XDG_DATA_HOME:-$HOME/.local/share}/io.github.atilla.cassette"
```

## Windows 10/11 x86_64

Cassette uses the MSVC build of GStreamer. The planned beta installers are not self-contained: install the official GStreamer **1.26.11 MSVC x86_64 runtime** before starting Cassette.

1. Download `gstreamer-1.0-msvc-x86_64-1.26.11.msi` from the [official GStreamer 1.26.11 MSVC directory](https://gstreamer.freedesktop.org/pkg/windows/1.26.11/msvc/).
2. Install the complete runtime, keeping its directory structure intact.
3. Ensure the runtime's `bin` directory is visible in `PATH` before launching Cassette. With the release-build layout this is `C:\gstreamer\1.0\msvc_x86_64\bin`; use the actual directory if you installed it elsewhere.

Windows beta limitations:

- Linux desktop notifications, MPRIS, and CD detection/ripping are unavailable.
- Experimental video/DVD functionality is disabled and unsupported.
- Missing or undiscoverable GStreamer runtime files can prevent the dynamically linked application from starting.
- The planned NSIS and MSI installers are unsigned and have not passed checkpoint 2 or clean Windows 10/11 testing.
- Tauri's WebView2 download bootstrapper may require network access if WebView2 is missing.

See [docs/GSTREAMER-WINDOWS.md](docs/GSTREAMER-WINDOWS.md) for the build and runtime strategy.

## FLAC tag editor

FLAC is the only writable tag format in Cassette `0.1.0-beta.1`. Editing tags modifies metadata inside the actual audio file. Keep backups and test important workflows on disposable copies first. MP3, OGG, Opus, WAV, and M4A tags remain read-only because they have not received equivalent real-file rollback testing.

## Development and validation

Requirements include Node.js 22, Rust stable, Tauri's native platform dependencies, and GStreamer development files.

```sh
npm ci
npm run build
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --bundles deb,rpm
git diff --check
```

The 13 ignored real-media fixture tests are intentionally excluded from normal validation and must be run only against deliberately prepared disposable media.

## Bugs and license

Report beta problems through [Cassette GitHub Issues](https://github.com/atilla-m/cassette/issues). Include the operating system, package format, Cassette version, relevant GStreamer/runtime details, reproduction steps, and sanitized logs; do not attach private library databases or personal media.

The release process is documented in [docs/RELEASING.md](docs/RELEASING.md), with release-path and artifact-scanner details in [docs/ARTIFACT-SAFETY.md](docs/ARTIFACT-SAFETY.md).

Cassette is free software licensed under the [GNU General Public License version 3 or later](LICENSE) (`GPL-3.0-or-later`). The full license is included in application bundle resources.
