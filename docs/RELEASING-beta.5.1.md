# Cassette 0.1.0-beta.5.1 Linux release checklist — published

This is the active Linux x86_64 checklist. Preserve the [unpublished beta.5 candidate](RELEASING-beta.5.md), all older tags/assets, production feed, installed data, stock icon, and separate `feature/windows-beta4` branch. Lyrics editing and Windows distribution remain deferred. Only DEB, RPM, AppImage, and AppImage signature belong in this release.

## Source and CI

- [x] Review the Stats ranking-page and Settings history-transfer change, including view history, selected-period filtering, incremental full-list access, and unchanged import/export behavior.
- [ ] Record a specific packaged manual UI recheck of the dedicated Stats pages and Settings history-transfer controls. The user's exploratory beta.5.1 report was “it must be fine,” not a confirmation of each listed case.
- [x] Confirm clean reviewed `main` equaled `origin/main`, GitHub-linked author/committer identities, exact active `0.1.0-beta.5.1` versions and selectors, and absent tag/release before tagging. `v0.1.0-beta.5` was not moved.
- [x] Run version/release-policy and LF/CRLF checks. Preserve production updater public key and endpoint, Ubuntu 24 signed packaging, draft/prerelease creation, exact four-asset allowlist, scanner/signature/LICENSE gates, host Wayland-client exclusion, and library/plugin exclusions. Ordinary CI remained AppImage-free and signing-credential-free.
- [x] Require Linux and Windows CI success for exact release commit `4399ac3756b724e6fdd3efd40c4addb1564da6cb` ([run 36558865758](https://github.com/atilla-m/cassette/actions/runs/36558865758)). Windows CI is regression coverage, not Windows desktop qualification. The 13 real-media tests stayed ignored.

## Signed candidate

- [x] After exact-target authorization and CI success, create annotated tag `v0.1.0-beta.5.1` at the reviewed commit. The [Ubuntu 24 signed workflow](https://github.com/atilla-m/cassette/actions/runs/36579832577) and draft-only creation passed; no retagging or asset replacement occurred.
- [x] Download exact signed workflow artifact `11039614139` and draft release `399222757` assets by ID; record provenance, sizes, and SHA-256 values in the retained local verification manifest. Independent package identity/LICENSE/payload/Wayland checks, production-key signature verification, and disposable tamper rejection passed. No altered file was executed.
- [ ] Perform a focused isolated packaged beta.4 → beta.5.1 migration/Stats/history-transfer check with synthetic media and real FUSE. Reuse existing native import/export and other feature qualification; do not mark unreported manual checks passed. Physical-device transfer and real-library restoration remain unverified.

## Publication — separate authorization

- [x] Recheck draft identity, immutable tag target, four assets/hashes, and release notes. Add only the exact beta.5.1 Pages tag rule, retaining all prior rules.
- [x] With separate approval, publish existing release `399222757` as a prerelease with `make_latest=false`. [Feed run 36606022362](https://github.com/atilla-m/cassette/actions/runs/36606022362) passed both validation and Pages deployment. Anonymous downloads of all four assets matched their recorded hashes and draft bytes; the live feed's URL, version, notes, and signature matched the release, and the public AppImage/signature pair independently verified.
- [x] Keep beta.4 download instructions usable until beta.5.1 publication; then update the README to link the new release while retaining historical beta.4 access.
- [ ] Clean-system installation, audible playback, uninstall, physical-device history transfer, real-library restoration, and Windows desktop checks remain pending unless separately performed for these packages.
