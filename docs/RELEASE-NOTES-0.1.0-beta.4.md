# Cassette 0.1.0-beta.4

Linux x86_64 prerelease. This release carries the beta.3 desktop, lyrics, play-history, and Stats improvements and adds:

- Individual and album-wide tag editing for FLAC, MP3, Ogg/Vorbis, Opus, WAV, and M4A/AAC, with explicit Keep/Set/Clear choices, preflight, backups, verification, and recovery reporting. Mixed-format albums show editable and excluded tracks before saving.
- Back/forward browsing history, linked artist and genre labels, consistent play-total visibility across detail pages, and album-song display sorting that does not change track numbers or the playback queue.
- A persistent ♪ cue for opening instrumental lyrics markers and an uncropped, keyboard-accessible cover-art viewer on album and lyrics pages.
- A release-workflow fix that uses the newly created draft's ID instead of depending on immediate releases-list visibility.

Download the signed `Cassette_0.1.0-beta.4_amd64.AppImage` for portable use, `Cassette_0.1.0-beta.4_amd64.deb` for Ubuntu/Debian packaging, or `Cassette-0.1.0-beta.4-1.x86_64.rpm` for Fedora packaging. On a clean Ubuntu 24.04 desktop, the AppImage requires `sudo apt install libfuse2t64` for real FUSE mounting. Run the AppImage from its chosen location; install the DEB with `sudo apt install ./Cassette_0.1.0-beta.4_amd64.deb` or the RPM with `sudo dnf install ./Cassette-0.1.0-beta.4-1.x86_64.rpm`. The detached `.AppImage.sig` is for signed updater verification, not a separate application.

AppImage updates require confirmation and signature verification. DEB/RPM updates are notification-only: download a newer package from Releases and install it with APT/DNF; no automatic package repository is configured.

Tag editing changes audio-file metadata, so keep independent backups of important music. MP3 and WAV preserve supported existing ID3v2.3/v2.4 layouts; ID3v2.2, MP3 files with only ID3v1/APEv2, and WAV files with only RIFF INFO remain read-only rather than being silently converted. M4A/AAC means AAC inside MP4, not raw ADTS. WAV track-number reading remains limited. Date-filtered Stats screens and lyrics editing are deferred; stock icons remain. Windows is not distributed in this Linux release while its separate installer branch awaits manual qualification. DEB/RPM are not repository-signed; the AppImage updater signature is a separate protection.
