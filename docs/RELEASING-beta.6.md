# Cassette 0.1.0-beta.6 Windows and Linux release checklist

- Confirm both annotated tags, `v0.1.0-beta.6` and `windows-v0.1.0-beta.6`, resolve to the same commit. The tagged version must match the native, Tauri, and npm versions.
- Wait for the signed Linux release workflow and the signed Windows job in CI to pass. Check the signature verification results and the installed Windows scanner and interface smoke tests.
- Verify the Linux draft has exactly the DEB, RPM, AppImage, and AppImage signature files. Verify the Windows draft has the installer, its signature, and `windows-update.json`. The manifest must identify the matching version, installer, and signature.
- Fast-forward `main` to that commit so GitHub’s publication event uses the beta.6 Pages workflow. Use the [beta.6 release notes](RELEASE-NOTES-0.1.0-beta.6.md) for both drafts and publish them as prereleases.
- Confirm both public releases and the published Windows manifest. Wait for the Pages workflow to verify the Linux AppImage again and deploy the [Linux beta feed](https://atilla-m.github.io/cassette/updates/beta/latest.json). Check that the feed identifies the beta.6 AppImage.
