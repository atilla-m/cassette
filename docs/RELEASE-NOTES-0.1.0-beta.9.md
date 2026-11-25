# Cassette 0.1.0-beta.9

Windows x64 and Linux x86_64 are built from the same source commit. [Windows installer](https://github.com/atilla-m/cassette/releases/tag/windows-v0.1.0-beta.9) · [Linux AppImage, DEB, and RPM](https://github.com/atilla-m/cassette/releases/tag/v0.1.0-beta.9)

**Continuous playback counts the right track.** During a gapless handoff, Cassette waits for the new audio stream before switching tracks and starting its listen timer. Finishing a song can no longer give the next song a listen at the start. Each track earns a listen after reaching its own configured threshold, which defaults to 50%.

**Remove individual listens.** Stats → Recently Played shows each dated listen, including repeat plays of the same track. Select **Remove listen** to update that track’s count and the listening charts.

Windows and Linux AppImage updates are verified with Cassette’s bundled updater key. DEB and RPM users can install the newer package through their package manager.
