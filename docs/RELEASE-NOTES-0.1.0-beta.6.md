# Cassette 0.1.0-beta.6

This beta is available for Windows x64 and Linux x86_64. Both builds use the same source commit. [Windows downloads](https://github.com/atilla-m/cassette/releases/tag/windows-v0.1.0-beta.6) · [Linux downloads](https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.6)

- **Playback** advances to the next track when audio ends and recovers when playback reaches the end without an end signal. Windows supports play, pause, previous, and next through system media controls, including supported headset buttons.
- **Library** pages fit narrower windows, including artist and genre details. The Albums header keeps Rescan Library. Use the album header’s **⋯** menu to edit or restore artwork. Custom covers are stored in Cassette’s library.
- **Stats** puts listening activity and rankings up front, with period filters and access to the full rankings. Plain lyrics use the same reading layout as synced lyrics.
- **Appearance** defaults to Obsidian for new installations, adds a Light theme, and matches the Windows title bar to the selected theme.
- **Audio CDs** can be ripped to FLAC on Windows and Linux. Windows supports audio-only discs through an optical drive. Linux uses `cdparanoia`, `flac`, and `libdiscid`.
- **Updates** are available in Settings. Cassette verifies the Windows installer and Linux AppImage with its updater key. DEB and RPM users install the downloaded update through their package manager.

Windows ships as an installer with WebView2 and GStreamer. Linux ships as an AppImage, DEB, and RPM. Physical optical drives and headset buttons have limited manual coverage.
