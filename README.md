# Cassette

Cassette is a private, local-first desktop music library and player for Windows and Linux. It scans folders you choose, keeps its library and listening history locally, and provides albums, artists, genres, songs, playlists, gapless playback, lyric editing, statistics, themes, and guarded metadata editing.

## Beta status

Cassette 0.1.0-beta.8 is the current prerelease. Both platform packages use the same source commit. See the [beta.8 release notes](docs/RELEASE-NOTES-0.1.0-beta.8.md).

Download `Cassette_0.1.0-beta.8_x64-setup.exe` from the [Windows x64 release](https://github.com/atilla-m/cassette/releases/tag/windows-v0.1.0-beta.8).

Choose the package matching your installation method from the [Linux x86_64 release](https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.8).

- `Cassette_0.1.0-beta.8_amd64.AppImage` is the portable AppImage and the recommended download.
- `Cassette_0.1.0-beta.8_amd64.deb` is for the Ubuntu package path.
- `Cassette-0.1.0-beta.8-1.x86_64.rpm` is for the Fedora package path.

Earlier Linux releases include [beta.5.1](https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.5.1) and [beta.4](https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.4). Beta.4 shipped `Cassette_0.1.0-beta.4_amd64.AppImage`, `Cassette_0.1.0-beta.4_amd64.deb`, and `Cassette-0.1.0-beta.4-1.x86_64.rpm`.

Obsidian is the default theme for new installations. Choose Light, Glacier, or Cassette Teal in Settings. The album page’s **⋯** menu includes **Edit album art** and, when a custom cover is set, **Restore original art**. Custom covers are stored in Cassette’s library.

## Linux x86_64

The AppImage is the recommended download. It runs without replacing a system package. AppImage updates require confirmation and verification with Cassette’s bundled updater key. DEB and RPM users can check for an update in Settings and install the newer package with APT or DNF. See the [Linux beta update feed](https://atilla-m.github.io/cassette/updates/beta/latest.json).

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

Install a downloaded DEB on the tested Ubuntu 24.04 package path:

```sh
sudo apt install ./Cassette*.deb
```

Cassette requires WebKitGTK and GStreamer with the base, good, bad/codec, ugly/restricted-codec, and libav plugin families for the documented FLAC, MP3, OGG/Vorbis, Opus, WAV, and M4A/AAC playback. DEB and RPM metadata declare these required runtime families; the package manager resolves the platform-specific GUI libraries.

Linux notifications call the external `notify-send` command. The package recommends `libnotify-bin` on Debian/Ubuntu or `libnotify` on Fedora, but it is optional: when absent, notification attempts report that notification support is unavailable without preventing Cassette from launching or using the music library.

CD detection and ripping remain an optional Linux feature and use `cdparanoia`, `flac`, and `libdiscid`. Missing CD tools produce a feature-specific error and do not affect the core music library. Experimental video/DVD functionality is disabled for this beta, so its external tools are neither required nor advertised as available beta features.

### Data and uninstall

Cassette stores Linux application data under `$XDG_DATA_HOME/io.github.atilla.cassette`, defaulting to `~/.local/share/io.github.atilla.cassette`. The library database is `library.sqlite3` in that directory; cached cover art and other application-managed data are stored alongside it.

Detailed play-history tracking begins when this database is first opened by a build containing the `track_play_events` ledger. The exact start is stored as a UTC Unix timestamp in `library_meta` under `detailed_play_history_started_at_utc`. Events use Cassette's existing track identity and remain stored independently of the limited Recently Played display, rescans, and metadata refreshes. Existing all-time play counts are preserved. When an older track has a usable `last_played_at` value, migration preserves that one known event timestamp; it does not invent dates for the rest of the legacy total.

Play events are stored in UTC. Stats converts requested local calendar boundaries to UTC and queries a half-open interval (`start <= played_at_utc < end`). This keeps daylight-saving and timezone handling at the reporting boundary instead of permanently assigning a local date to an event. Undated legacy plays remain part of all-time totals only.

Beta.5.1 includes local-calendar Stats periods, a daily chart, and portable JSON history export/import. See [listening-history accuracy and restore behavior](docs/LISTENING-HISTORY.md) for matching, legacy-total conflict handling, and isolated manual testing. Physical-device transfer and real-library restoration have not been manually tested.

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

## Windows 10/11 x86_64

Run the installer from the Windows release. It includes Cassette’s GStreamer playback runtime and the WebView2 offline installer. Supported headphone buttons use Windows system media controls when Cassette has an active playback session.

Use **Settings → Updates** to check for Windows prereleases. Cassette verifies the installer with its bundled updater key before launching it with your confirmation. Windows may show an unknown publisher prompt.

## Tag editors

The individual-song and album editors can write FLAC, MP3, Ogg/Vorbis, Opus, WAV, and M4A files containing AAC audio. The album editor lists every affected file and any file it must exclude; leaving a shared field unchanged retains each song's own value. Editing writes metadata into the audio file, so keep independent backups of important music.

Cassette preserves MP3 and WAV ID3v2.3 or ID3v2.4 layouts. Existing ID3v2.2 tags, MP3 files tagged only with ID3v1/APEv2, and WAV files tagged only with RIFF INFO are read-only because writing the shared editor fields would require a tag-format conversion. Raw AAC/ADTS is outside the six library formats; M4A/AAC means AAC in an MP4 container. The editor verifies the unchanged audio payload and unrelated metadata before replacing the original file. If verification fails, the original remains in place.

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

See the [beta.8 release checklist](docs/RELEASING-beta.8.md) and [artifact scanner documentation](docs/ARTIFACT-SAFETY.md).
Updater security, signing-key custody, and beta-feed publication are documented in [docs/UPDATES.md](docs/UPDATES.md).

Cassette is free software licensed under the [GNU General Public License version 3 or later](LICENSE) (`GPL-3.0-or-later`). The full license is included in application bundle resources.
