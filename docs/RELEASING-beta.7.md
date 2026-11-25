# Cassette 0.1.0-beta.7 Windows and Linux release checklist

- Confirm the annotated tags `v0.1.0-beta.7` and `windows-v0.1.0-beta.7` resolve to the same commit and match the native, Tauri, and npm versions.
- Confirm the signed Linux and Windows jobs passed. Check the installer smoke tests, package audits, and updater signature verification.
- Verify both drafts and their uploaded files. Linux includes DEB, RPM, AppImage, and the AppImage signature. Windows includes the installer, its signature, and a manifest with the matching version, URL, and signature.
- Add the exact Linux release tag to the GitHub Pages deployment allowlist. Fast-forward `main` to the tested release commit so the publication event uses the beta.7 Pages workflow.
- Publish both drafts as prereleases with the [beta.7 release notes](RELEASE-NOTES-0.1.0-beta.7.md). Confirm the public downloads, the Windows update manifest, and the deployed [Linux beta update feed](https://atilla-m.github.io/cassette/updates/beta/latest.json).
