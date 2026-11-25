# Cassette 0.1.0-beta.8 Windows and Linux release checklist

- Confirm the annotated tags `v0.1.0-beta.8` and `windows-v0.1.0-beta.8` resolve to the same commit and match the native, Tauri, and npm versions.
- Put the tagged commit on the feature branch so the signing workflows can create drafts. Confirm package audits, installer smoke tests, and signature verification passed.
- Verify the Linux draft contains the DEB, RPM, AppImage, and AppImage signature. Verify the Windows draft contains the installer, its signature, and the update manifest for that installer.
- Allow the exact Linux release tag to deploy to GitHub Pages. Fast-forward `main` to the tested release commit and publish both drafts as prereleases with the [beta.8 release notes](RELEASE-NOTES-0.1.0-beta.8.md).
- Verify the public downloads, Windows update manifest, and [Linux beta feed](https://atilla-m.github.io/cassette/updates/beta/latest.json).
