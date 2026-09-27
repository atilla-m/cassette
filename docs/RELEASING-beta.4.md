# Cassette 0.1.0-beta.4 Linux release checklist

This is the active beta.4 checklist. [The beta.3 checklist](RELEASING.md), prior release notes, and diagnostics remain historical evidence. The exact release is Linux x86_64 only: DEB, RPM, AppImage, and AppImage signature. The separate `feature/windows-beta4` branch must not enter this release; its loaded-track editing recheck is pending. Keep the stock icon and defer lyrics editing to beta.5.

## Source and CI

- [ ] Clean reviewed `main` equals `origin/main`; verify the configured GitHub-linked author and committer identities.
- [ ] Verify package/npm/Rust/Tauri/runtime versions and exact workflow, scanner, feed, and test selectors all say `0.1.0-beta.4`; retain the production updater key and endpoint.
- [ ] Verify updater signing guards, draft/prerelease policy, exact four-asset allowlist, LICENSE, and AppImage host-Wayland-client exclusion remain intact.
- [ ] Run focused version/release-policy, frontend, and native checks without the 13 ignored real-media tests. Require both Linux and Windows CI success for the exact release commit; Windows CI is regression coverage, not Windows qualification.

## Tagged draft

- [ ] Confirm `v0.1.0-beta.4` tag and release are absent. Annotate and push the exact reviewed `main` commit without moving any older tag.
- [ ] Require Ubuntu 24 signed release build, scanner, signature, and draft-creation jobs to pass; retain only the four allowlisted assets.
- [ ] Download the exact assets from the draft by ID and record run, tag, commit, asset IDs, sizes, and SHA-256 values. Verify package identity, LICENSE byte match, expected runtime payloads, excluded host Wayland client, and genuine AppImage signature against the committed production public key. Test tamper rejection without executing altered assets.
- [ ] Use disposable data and synthetic media for a focused genuine-FUSE Fedora packaged AppImage smoke. Do not touch the installed app or personal library. Record automated observations separately from any human GUI/audio result.

## Publication

- [ ] Recheck draft ID, exact tag target, four asset IDs/hashes, and notes. Confirm the Pages environment permits only the exact beta.4 tag alongside existing rules; add that exact rule only if absent.
- [ ] Publish the existing draft with `draft=false`, `prerelease=true`, `make_latest=false`. Do not replace assets or rebuild to clear a later failure.
- [ ] Wait for the release-triggered feed workflow. Check anonymous download bytes/hashes, public Pages version/URL/notes/signature, and independent production-key verification of the downloaded AppImage/signature pair.
- [ ] Report any unperformed clean-system Ubuntu/DEB/RPM, audible playback, uninstall, or Windows checks as pending, not passed. Preserve older releases and installed user data.
