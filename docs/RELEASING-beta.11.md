# Cassette 0.1.0-beta.11 Windows and Linux release checklist

- Confirm `v0.1.0-beta.11` and `windows-v0.1.0-beta.11` resolve to the same commit and match the npm, native, and Tauri versions.
- Push the tagged commit to the feature branch and confirm the two signing builds pass. Check listening time, Windows album artwork, package audits, Windows installer smoke tests, and updater signature verification.
- Verify the Linux draft contains the AppImage and its signature, DEB, and RPM. Verify the Windows draft contains the installer, its signature, and the matching updater manifest.
- Allow the exact Linux tag to deploy to GitHub Pages. Fast-forward `main` to the tested release commit and publish both prereleases with the [beta.11 release notes](RELEASE-NOTES-0.1.0-beta.11.md).
- Verify the public downloads, Windows update manifest, and [Linux beta feed](https://atilla-m.github.io/cassette/updates/beta/latest.json).
