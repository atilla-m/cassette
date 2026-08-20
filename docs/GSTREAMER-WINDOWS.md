# Windows GStreamer strategy

## Decision for Cassette 0.1.0-beta.1

Linux is the primary tested platform for `v0.1.0-beta.1`. Windows 10/11 x86_64 support is beta, the planned installers are unsigned and not self-contained, and checkpoint 2 must still verify that they build and work on clean machines. Users must install the official GStreamer 1.26.11 MSVC x86_64 runtime and make its `bin` directory available through `PATH`.

The CI and release builders install both official packages:

- `gstreamer-1.0-msvc-x86_64-1.26.11.msi`
- `gstreamer-1.0-devel-msvc-x86_64-1.26.11.msi`

They are downloaded only from `gstreamer.freedesktop.org` and checked against SHA-256 values published alongside the installers. The automated setup is in `scripts/install-gstreamer-windows.ps1`.

The development package is used only by the build runner. It is not uploaded or added to Cassette's installer. Cassette does not package headers, `.lib` files, static libraries, PDBs, test programs, or raw GStreamer development directories.

## Why the runtime is external

A private runtime deployment needs a verified closure of DLL dependencies and plugins, preserved GStreamer layout, codec-by-codec license notices, and patent/redistribution review. That work has not been completed. In particular, a convenient “full runtime” copy can include GPL or patent-sensitive codec packages that should not be redistributed casually.

The official GStreamer deployment documentation permits shared-runtime installation and explains that the application must be able to locate the runtime `bin` directory. It also explains that plugins are discovered relative to `gstreamer-1.0-0.dll` when the installed layout is preserved.

Because Rust's GStreamer bindings dynamically link the core DLL, an absent or undiscoverable runtime may fail in the Windows loader before Cassette can display an in-app error. Documentation alone is not the desired final experience. A tested self-contained runtime or a launcher/installer prerequisite check remains release work before Windows should be described as generally available.

## Required playback elements

Cassette uses `playbin` for audio and GStreamer's discovery/typefinding support while scanning. CI verifies these runtime elements:

- Core/playback: `playbin`, `uridecodebin`, `decodebin3`, `filesrc`, `typefind`
- Audio pipeline: `audioconvert`, `audioresample`, `autoaudiosink`
- FLAC: `flacdec`
- MP3: `mpg123audiodec` or `avdec_mp3`
- OGG/Vorbis: `oggdemux`, `vorbisdec`
- Opus: `opusdec`
- WAV: `wavparse`
- M4A/AAC: `qtdemux` plus `faad` or `avdec_aac`

A successful Rust compile proves that headers/import libraries were found; it does not prove that these plugins load or that real files play. The release checklist therefore requires representative real-file tests on clean Windows 10 and Windows 11 virtual machines.

Experimental video/DVD functionality is disabled and unsupported in Cassette `0.1.0-beta.1` on every platform. Existing backend code may remain, but the beta does not expose its UI and does not require or advertise `mpv`, `ffmpeg`, `ffprobe`, or `lsdvd` as beta runtime dependencies.

## Future self-contained distribution requirements

Before bundling a private GStreamer runtime:

1. Enumerate the transitive DLL dependency closure for every required plugin.
2. Keep `bin` and `lib\gstreamer-1.0` in the layout expected by GStreamer.
3. Initialize DLL and plugin search paths before any GStreamer symbol is loaded.
4. Exclude headers, import/static libraries, debug symbols, caches, development tools, and unused plugins.
5. Generate third-party notices from the exact redistributed files.
6. Review LGPL obligations and every codec/plugin license.
7. Exclude GPL or patent-sensitive codec components unless redistribution has been explicitly approved.
8. Test clean installs without GStreamer or a modified `PATH`.
9. Test uninstall and upgrades without removing a separately installed shared runtime.

Do not change the Windows installer claim to “self-contained” until those checks pass.
