# Cassette 0.1.0-beta.5.1 — unreleased

Planned Linux x86_64 prerelease. This candidate carries the beta.5 source changes and a final Stats layout refinement. The earlier signed beta.5 candidate remains an unpublished draft and is not the download for this version.

- Detailed Stats: local-calendar periods, total plays, distinct tracks and artists, a daily chart, and dedicated **View all** pages for the complete period-filtered track, artist, album, genre, and recently played lists. Full lists load incrementally; the selected period remains in effect when opening a list.
- Portable listening-history export and previewed import, now under **Settings → Portable listening history**. Backups contain the full history independent of the Stats period, retain original event identities and legacy totals, and preview duplicate, unmatched, ambiguous, and conflicting data before import.
- Browsing controls, artist and genre top-ten songs with sortable full lists, album-song display sorting, persistent Songs search/sort, and optional numbered Songs positions.
- Player-bar artist and album links, with Lyrics opening only from its button.

Undated legacy plays remain in authoritative all-time totals but cannot be assigned to dated periods or chart days. Historical coverage is partial; rankings group by current tags rather than past metadata snapshots. Physical-device transfer and real-library restoration have **not** been manually tested.

Lyrics editing and Windows distribution remain deferred. The stock icon and themes are unchanged. Tag editing keeps its documented format and layout restrictions; back up important music before editing. The planned Linux release contains only DEB, RPM, a signed AppImage, and its detached updater signature. DEB/RPM are not repository-signed; AppImage updates require signature verification and user confirmation. No beta.5.1 package is available until this candidate is built, verified, and separately published.
