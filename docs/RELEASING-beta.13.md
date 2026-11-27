# Cassette 0.1.0-beta.13 Windows and Linux release checklist

- Confirm `v0.1.0-beta.13` and `windows-v0.1.0-beta.13` resolve to the same commit and match the npm, native, and Tauri versions.
- Push the release commit to the feature branch and confirm both signed release builds pass. Check listening-time accuracy, Songs layout, Windows installer smoke tests, package audits, and updater signature verification.
- Verify the Linux draft contains the AppImage, signature, DEB, and RPM. Verify the Windows draft contains the installer, signature, and updater manifest.
- Allow the exact Linux tag to deploy to GitHub Pages. Fast-forward `main` to the tested commit and publish both prereleases using the [release notes](RELEASE-NOTES-0.1.0-beta.13.md).
- Verify the public downloads, Windows update manifest, and [Linux beta feed](https://atilla-m.github.io/cassette/updates/beta/latest.json).
