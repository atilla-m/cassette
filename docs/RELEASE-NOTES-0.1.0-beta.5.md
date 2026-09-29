# Cassette 0.1.0-beta.5 — unreleased

Planned Linux x86_64 prerelease. Beta.5 carries the earlier library, playback, six-format tag-editing, lyrics, and cover-viewer features and adds:

- Detailed Stats with local-calendar Today, This week (starting Monday), This month, This year, previous month/year, and custom periods; total plays, distinct tracks and artists; a daily chart; and complete period-filtered track, album, artist, and genre rankings.
- Portable JSON listening-history export and previewed import. Backups include full history independent of the selected Stats period, original event identities, coverage information, and legacy totals. Import detects duplicates, retains unmatched history for later association, and refuses ambiguous matches or unresolved legacy-total conflicts instead of guessing.
- More browsing controls: consistent back/forward placement, artist and genre top songs (up to ten most-played) with sortable **View all** pages, and album-song display sorting.
- Songs search and sort retained when leaving and returning, plus optional numbered list positions for seeing a song's place in the displayed ranking.
- Player-bar artist and album links, and corrected Lyrics click behavior so blank player-bar space does not open Lyrics.

All-time totals include undated legacy plays; dated periods and charts cannot assign dates to them. Detailed-tracking coverage is partial for older libraries, and an earlier known last-played date does not prove complete historical coverage. Rankings group by current metadata, not historical tag snapshots. Physical-device transfer and real-library restoration have **not** been manually tested.

Lyrics editing and Windows distribution remain deferred. The stock icon and existing themes are unchanged. Tag editing retains its documented format and layout restrictions; back up important music before editing. The Linux release, when qualified, will contain only DEB, RPM, a signed AppImage, and its detached updater signature. DEB/RPM are not repository-signed; AppImage updates require signature verification and user confirmation. No beta.5 package is available until the signed candidate is built, verified, and separately published.
