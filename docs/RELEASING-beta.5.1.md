# Cassette 0.1.0-beta.5.1 Linux release checklist — unreleased

This is the active Linux x86_64 checklist. Preserve the [unpublished beta.5 candidate](RELEASING-beta.5.md), all older tags/assets, production feed, installed data, stock icon, and separate `feature/windows-beta4` branch. Lyrics editing and Windows distribution remain deferred. Only DEB, RPM, AppImage, and AppImage signature belong in this release.

## Source and CI

- [ ] Review the Stats ranking-page and Settings history-transfer change, including view history, selected-period filtering, incremental full-list access, and unchanged import/export behavior. Record a manual UI recheck separately; the earlier beta.5 packaged check does not cover this layout.
- [ ] Confirm clean reviewed `main` equals `origin/main`, GitHub-linked author/committer identities, exact active `0.1.0-beta.5.1` versions and selectors, and absent `v0.1.0-beta.5.1` tag/release. Never move `v0.1.0-beta.5`.
- [ ] Run version/release-policy and LF/CRLF checks. Preserve production updater public key and endpoint, Ubuntu 24 signed packaging, draft/prerelease creation, exact four-asset allowlist, scanner/signature/LICENSE gates, host Wayland-client exclusion, and library/plugin exclusions. Ordinary CI remains AppImage-free and signing-credential-free.
- [ ] Require Linux and Windows CI success for the exact release commit. Windows CI is regression coverage, not Windows desktop qualification. Keep the 13 real-media tests ignored.

## Signed candidate

- [ ] After exact-target authorization and CI success, create an annotated `v0.1.0-beta.5.1` tag at the reviewed commit; never retag or overwrite assets. Require the Ubuntu 24 signed workflow and draft-only creation to pass.
- [ ] Download the exact draft artifact by ID; record run, tag, commit, release/asset IDs, sizes, and SHA-256 values. Independently scan package identity, LICENSE, payloads, Wayland exclusion, and verify the genuine production-key AppImage signature and disposable tamper rejection. Never run altered files.
- [ ] Perform a focused isolated packaged beta.4 → beta.5.1 migration/Stats/history-transfer check with synthetic media and real FUSE. Reuse existing native import/export and other feature qualification; do not mark unreported manual checks passed. Physical-device transfer and real-library restoration remain unverified.

## Publication — separate authorization

- [ ] Recheck draft identity, immutable tag target, four assets/hashes, and [release notes](RELEASE-NOTES-0.1.0-beta.5.1.md). Add only the exact beta.5.1 Pages tag rule if needed.
- [ ] Only after approval, publish as a prerelease with `make_latest=false`, wait for feed deployment, and verify anonymous downloads/hashes, production feed URL/version/notes/signature, and independent signature verification.
- [ ] Keep the [published beta.4 download instructions](../README.md#beta-status) usable until beta.5.1 publication. Report clean-system installation, audible playback, uninstall, and Windows desktop checks as pending unless performed for these packages.
