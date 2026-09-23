# Cassette 0.1.0-beta.3 release notes

> Unreleased. These notes describe the beta.3 candidate and must not be presented as a published release until signed-package qualification and publication are complete.

Cassette 0.1.0-beta.3 is a Linux x86_64 beta focused on desktop integration, playback polish, richer listening statistics, and safer library maintenance.

## Highlights

- AppImage users can add, refresh, or remove Cassette's managed per-user application-menu launcher from Settings. The launcher continues to point to the AppImage's current location and never replaces package-managed entries.
- Linux playback notifications are transient and replace Cassette's previous playback banner instead of accumulating stale entries. Cassette Teal, Glacier, and Obsidian remain the visible theme choices.
- Qualifying plays now record a timestamped history event while preserving existing all-time totals. Date-filtered weekly, monthly, and yearly Stats screens are not part of this release.
- Synced lyrics show a subtle musical-note cue for timestamped instrumental breaks, and lyric-click seeking no longer flashes or scrolls through 0:00 while a seek is in flight.
- Albums, artists, genres, and album tracks can show play totals; browse pages add most/least-played sorting; and every Top/Recently Played Stats result is reachable through incremental full lists.
- Album detail now supports guarded batch editing of shared FLAC album fields, with mixed-value handling, explicit clearing, confirmation, verified writes, backups, rollback reporting, and library regrouping.

## Scope and limitations

- Tag writing remains FLAC-only. MP3, OGG, Opus, WAV, and M4A metadata is read-only.
- WAV track-number reading remains a known beta limitation.
- Stock application icons remain in use.
- Windows builds remain CI regression coverage; Windows distribution is deferred.

The candidate is not yet published. Until publication, use the existing [`v0.1.0-beta.2` release](https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.2).
