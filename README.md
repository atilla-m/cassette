# Cassette

Cassette is a private, local-first desktop music library and player. It scans folders you choose, keeps its library cache locally, and provides albums, artists, genres, songs, playlists, queue management, lyrics, statistics, themes, and safe FLAC metadata editing.

## Beta status

Cassette `0.1.0-beta.3` is the unreleased next Linux x86_64 candidate. The current public release remains [`v0.1.0-beta.2`](https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.2); keep using its downloads until beta.3 has completed signed-candidate qualification and is intentionally published. Windows build infrastructure remains in CI, but Windows installer qualification and distribution are deferred to a later beta.

Choose the beta.2 file matching the installation method:

- `Cassette_0.1.0-beta.2_amd64.AppImage` is the portable, signed AppImage and the recommended download.
- `Cassette_0.1.0-beta.2_amd64.deb` is for the tested Ubuntu 24.04 package path.
- `Cassette-0.1.0-beta.2-1.x86_64.rpm` is for the tested Fedora 44 package path.

Beta.3 adds per-user AppImage application-menu integration; transient/replaced playback notifications and streamlined theme choices; detailed timestamped play history; instrumental-break cues and stable lyric seeking; expanded play counts, sorting, and full Stats lists; and guarded album-wide FLAC tag editing. Date-filtered Stats screens are not included. Cassette Teal remains the default user-selectable theme, alongside Glacier and Obsidian. The hidden Rose Noir and Royal Gold implementations remain in the source for possible later reactivation.

Modern UI work is reserved for a future release. This beta does not include a Modern/Legacy interface switch or promise the rejected Modern design. Experimental video and DVD functionality is disabled and unsupported in this beta; hidden backend code and tools such as `lsdvd`, `ffmpeg`, `ffprobe`, and `mpv` are not part of the beta runtime contract.

The Linux beta formats are DEB, RPM, and AppImage. AppImage updates are signed and require explicit user confirmation. DEB and RPM stay under the system package manager: Cassette may notify those users about a newer version and open its GitHub release page, but it will not invoke `sudo`, `apt`, `dpkg`, `dnf`, or `rpm`, and it will not convert an installation to AppImage. Download a newer DEB or RPM from Releases and install it with APT or DNF; no automatic package repository is configured. Windows NSIS work remains available in CI but is not a beta.3 release asset. MSI is deferred because WiX/MSI cannot represent the authoritative `0.1.0-beta.3` prerelease identifier without changing the project version.

The beta uses the stock Tauri desktop icons as an accepted known limitation. DEB and RPM packages are not repository-signed, and planned Windows installers remain unsigned; package managers, desktop security tools, or Windows SmartScreen may warn about an unrecognized publisher. Tauri updater signatures are a separate mandatory verification layer for AppImage updates.

## Linux x86_64

The AppImage is the recommended download. It can run without replacing a system package. The production updater public key and beta endpoint remain configured; updates are never forced, and development builds do not contact the update feed. The public feed continues to serve beta.2 until beta.3 is separately published.

A clean Ubuntu 24.04 installation required the FUSE 2 compatibility library before the AppImage could mount and run:

```sh
sudo apt install libfuse2t64
```

This prerequisite applies to the AppImage. It records the tested Ubuntu 24.04 setup and does not imply the same package name or requirement on other distributions.

Automatic checks are limited to one attempt per 24 hours across restarts, including network failures; manual checks remain available. AppImage installation requires native runtime verification and explicit confirmation. Downloads cannot be cancelled after confirmation in the current beta. `npm run release:linux` and ordinary CI build DEB/RPM only; `npm run release:linux:signed` is the sole supported distributable AppImage build and requires the production configuration and credentials. See [the update handoff](docs/UPDATES.md) for signature verification, publication, and key rotation.

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

Detailed play-history tracking begins when this database is first opened by a build containing the `track_play_events` ledger. The exact start is stored as a UTC Unix timestamp in `library_meta` under `detailed_play_history_started_at_utc`. Events use Cassette's existing track identity and remain stored independently of the limited Recently Played display, rescans, and metadata refreshes. Existing all-time play counts are preserved. When an older track has a usable `last_played_at` value, migration preserves that one known event timestamp; it does not invent dates for the rest of the legacy total.

Play events are stored in UTC. Future calendar statistics should convert the user's requested local period boundaries to UTC and query a half-open interval (`start <= played_at_utc < end`). This keeps daylight-saving and timezone handling at the reporting boundary instead of permanently assigning a local date to an event. Undated legacy plays remain part of all-time totals only.

Application binaries and user data are separate. A signed AppImage replacement, DEB/RPM update, or uninstall must not remove or replace the library database, settings, playlists, favorites, cached artwork, or other application data.

Remove the installed package with the matching package manager:

```sh
sudo apt remove cassette
sudo dnf remove cassette
```

Run only the command appropriate for the installed package. Package removal does not automatically delete the library database or other user data. After backing up anything you want to keep, optional manual data removal is:

```sh
rm -r -- "${XDG_DATA_HOME:-$HOME/.local/share}/io.github.atilla.cassette"
```

## Windows 10/11 x86_64 (deferred)

Cassette uses the MSVC build of GStreamer. The planned beta installers are not self-contained: install the official GStreamer **1.26.11 MSVC x86_64 runtime** before starting Cassette.

1. Download `gstreamer-1.0-msvc-x86_64-1.26.11.msi` from the [official GStreamer 1.26.11 MSVC directory](https://gstreamer.freedesktop.org/pkg/windows/1.26.11/msvc/).
2. Install the complete runtime, keeping its directory structure intact.
3. Ensure the runtime's `bin` directory is visible in `PATH` before launching Cassette. With the release-build layout this is `C:\gstreamer\1.0\msvc_x86_64\bin`; use the actual directory if you installed it elsewhere.

Windows is not included in the Linux-first `0.1.0-beta.3` candidate. The following limitations remain relevant to its later qualification:

- Linux desktop notifications, MPRIS, and CD detection/ripping are unavailable.
- Experimental video/DVD functionality is disabled and unsupported.
- Missing or undiscoverable GStreamer runtime files can prevent the dynamically linked application from starting.
- The planned NSIS installer is unsigned and has not passed clean Windows 10/11 testing.
- MSI is excluded from this beta because its version rules cannot represent `0.1.0-beta.3` faithfully.
- Tauri's WebView2 download bootstrapper may require network access if WebView2 is missing.

See [docs/GSTREAMER-WINDOWS.md](docs/GSTREAMER-WINDOWS.md) for the build and runtime strategy.

## FLAC tag editor

FLAC remains the only writable tag format in Cassette. Editing tags modifies metadata inside the actual audio file. Keep backups and test important workflows on disposable copies first. MP3, OGG, Opus, WAV, and M4A tags remain read-only because they have not received equivalent real-file rollback testing.

## Development and validation

Requirements include Node.js 22, Rust stable, Tauri's native platform dependencies, and GStreamer development files.

```sh
npm ci
npm run build
npm run check
npm run test:updater
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --bundles deb,rpm
git diff --check
```

The 13 ignored real-media fixture tests are intentionally excluded from normal validation and must be run only against deliberately prepared disposable media.

## Bugs and license

Report beta problems through [Cassette GitHub Issues](https://github.com/atilla-m/cassette/issues). Include the operating system, package format, Cassette version, relevant GStreamer/runtime details, reproduction steps, and sanitized logs; do not attach private library databases or personal media.

The release process is documented in [docs/RELEASING.md](docs/RELEASING.md), with release-path and artifact-scanner details in [docs/ARTIFACT-SAFETY.md](docs/ARTIFACT-SAFETY.md).
Updater security, signing-key custody, and beta-feed publication are documented in [docs/UPDATES.md](docs/UPDATES.md).

Cassette is free software licensed under the [GNU General Public License version 3 or later](LICENSE) (`GPL-3.0-or-later`). The full license is included in application bundle resources.
