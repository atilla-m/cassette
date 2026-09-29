# Cassette 0.1.0-beta.5 Linux release checklist — superseded unpublished draft

Historical candidate: annotated tag `v0.1.0-beta.5` targets `aee8d1dbaeb7dbf32f073c706b1715c238d871ef`; signed workflow run `36548844581` produced draft release `399001604` with the four verified Linux assets. It was never published or deployed to the production feed. Preserve its tag, draft, and assets; [beta.5.1 is the active checklist](RELEASING-beta.5.1.md).

This is the active beta.5 checklist. [Beta.4's checklist](RELEASING-beta.4.md), earlier release notes, and diagnostic results are historical evidence. The planned release is Linux x86_64 only: DEB, RPM, AppImage, and AppImage signature. Leave `feature/windows-beta4`, the stock icon, existing releases, and installed data untouched. Lyrics editing and Windows distribution are deferred.

## Source and CI

- [ ] Verify clean reviewed `main` equals `origin/main`, with the GitHub-linked author and committer identities. Record the exact eligible tag target.
- [ ] Verify npm/Rust/Tauri/runtime versions, workflow triggers, scanner, feed, and tests all select `0.1.0-beta.5`; keep the production updater public key and endpoint unchanged. Confirm the exact `v0.1.0-beta.5` tag and release are absent before any later tagging request.
- [ ] Verify Ubuntu 24 signed packaging, exact four-asset allowlist, draft/prerelease creation, LICENSE payload, signature verification, AppImage host-Wayland-client exclusion, and dependency/library exclusions. Keep the ordinary CI AppImage-free and signing-credential-free.
- [ ] Require Linux and Windows CI to pass for the preparation commit. Windows CI is regression coverage, not Windows distribution qualification. Keep the 13 real-media tests ignored.

## Signed candidate — separate authorization

- [ ] Annotate and push the exact reviewed `main` commit only after authorization; never move an older tag. Require the Ubuntu 24 signed build and scanner/signature gates to pass.
- [ ] Download draft assets by ID. Record run, tag, commit, asset IDs, sizes, and SHA-256 values. Independently verify package identity, LICENSE byte match, required payloads, host-Wayland-client exclusion, and genuine production-key AppImage signature; test tamper rejection without running altered artifacts.
- [ ] Run a focused packaged AppImage smoke with real FUSE, isolated data, and synthetic media. Reuse feature qualification without calling unperformed manual checks passed.
- [ ] Complete remaining manual checks: period boundaries/chart and full lists, export/import across disposable profiles, repeat/older import behavior, and current metadata grouping. Physical-device transfer and real-library restoration remain untested. Preserve undated legacy plays in all-time totals.

## Publication — separate authorization

- [ ] Recheck the exact draft ID/tag target, four asset IDs/hashes, and prepared [release notes](RELEASE-NOTES-0.1.0-beta.5.md). Add only the exact beta.5 Pages tag rule if needed, preserving existing rules.
- [ ] Publish the verified draft as a prerelease with `make_latest=false`; do not replace assets or rebuild merely to clear a later failure.
- [ ] Wait for the release-triggered feed deployment. Verify anonymous asset downloads and hashes, production feed URL/version/notes/signature, and independent public-key verification of the downloaded AppImage/signature pair.
- [ ] Keep the [published beta.4 download instructions](../README.md#beta-status) usable until beta.5 is actually published. Report clean-system DEB/RPM/FUSE, audible playback, uninstall, and Windows checks as pending unless performed for these packages.
