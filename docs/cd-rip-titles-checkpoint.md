# CD-rip title checkpoint

Branch: `fix/cd-rip-titles`, based on approved Settings/icons checkpoint
`d945b1b1aa89fe743977f09899a186d250d44e11`. No newer main or polish changes
are included. The original worktree's earlier uncommitted changes are preserved.

## Reproduction and correction

On this baseline, entering `Ark` and immediately clicking Rip with the input
still focused already sends `Ark` to native code. The field uses `oninput`, not
blur. That case did not reproduce the reported failure by itself.

A pending metadata lookup does reproduce title loss: type `Ark`, then resolve
the lookup with a release whose track 2 is `Track 02`, then click Rip. The old
`selectCdMetadataRelease` replaced the whole editable draft. Browser inspection
showed `Ark` becoming `Track 02` in both the field and the submitted request.
Native filename construction and FLAC tagging both consume that same request
title, so this failure affects both, rather than just the filename.

Automatic lookup now preserves fields manually edited since detection and fills
untouched ones. Explicitly detecting another disc starts a new edit cycle.
Explicitly choosing a release retains the existing behavior of applying that
release. Responses from a lookup invalidated by another detection are ignored.
Rip captures one independent metadata snapshot before entering its busy state;
blank titles normalize to `Track NN`. The native writer also applies that
fallback for blank or missing metadata. Filename sanitization remains separate
from tag text, retaining Japanese/Unicode and unsanitized title punctuation in
the embedded TITLE.

Existing folder suffix behavior is retained with atomic directory creation.
Destination files are checked before reading a track, and the FLAC encoder no
longer receives its overwrite flag. A destination appearing after preflight is
therefore also refused. Native results now carry each actual output path, which
the completed queue retains as the filename tooltip alongside the Last rip
folder. Starting another job clears stale path tooltips.

## Data flow

1. Track inputs call `updateCdTrackMetadata` on each input event and mark the
   edited field. Album fields retain their existing controls and handlers.
2. `handleRipCd` creates the detached, normalized snapshot and disables edits.
3. The unchanged `ripCdToFlac` API invokes `rip_cd_to_flac` with that snapshot.
4. Native code owns the deserialized metadata for the entire job. Its command
   calls the same worker exercised by the synthetic-disc tests.
5. Track-number matching supplies both `cd_track_filename_from_metadata` and
   `write_flac_tags`; only the filename passes through path sanitization.
6. Start/finish events and final results contain the selected filename and
   actual path. The output folder and names match the files created.
7. The existing explicit Scan ripped folder action remains unchanged; no
   automatic rescan is added. Scanner readback is included in native tests.

## Changed files

| File | Purpose |
| --- | --- |
| `src/routes/+page.svelte` | Preserve manual fields during lookup, reject stale responses, submit a snapshot and retain actual paths. |
| `src/lib/utils/cdRipMetadata.ts` | Lookup merge and detached snapshot/fallback logic. |
| `src/lib/types/library.ts` | Actual output path on CD track results. |
| `src-tauri/src/lib.rs` | Injectable disc-reader seam, fallback TITLE, atomic folder reservation and overwrite refusal; return actual paths. |
| `src-tauri/src/cd_rip_tests.rs` | Real FLAC encoding/tag/scanner tests with a synthetic reader. |
| `scripts/test-cd-rip-metadata.mjs` | Merge, snapshot and real frontend invoke regressions. |
| `package.json` | Focused `test:cd-rip` command; version/dependencies unchanged. |
| `.github/workflows/ci.yml` | Run those frontend regressions and install the FLAC CLI required by Linux synthetic native tests. |
| `docs/cd-rip-titles-checkpoint.md` | Diagnosis, validation and review handoff. |

## Validation

- `npm run test:cd-rip`: 5 passed, including the real API invoke wrapper with
  mocked IPC, manual blanks/reverted values, Unicode and snapshot independence.
- `cargo test --manifest-path src-tauri/Cargo.toml cd_rip_tests -- --nocapture`:
  6 passed. The real native job uses a synthetic reader and installed FLAC
  encoder. It verifies `02 - Ark.flac`/TITLE `Ark`, Japanese, whitespace fallback,
  duplicate titles, sanitized filenames versus unchanged tag text, unrelated
  metadata, decoded PCM, scanner titles, events/results paths, existing files,
  repeated jobs, broken links, concurrent folder reservation and a destination
  appearing after preflight. No real CD or real media fixture is used.
- Playwright against the production frontend with mocked native calls: focused
  input immediately followed by Rip; late lookup before and during ripping;
  stable request values and edit guards; Unicode/blank requests; completion
  filename/path; stale lookup after detection; the next disc's lookup; explicit
  release selection; no automatic scan.
- `npm run check`: zero errors/warnings. `npm run build`: passed.
- Existing player-navigation/album-browse regressions: 11 passed.
- `cargo check --manifest-path src-tauri/Cargo.toml` and Rust formatting check:
  passed. `git diff --check`: passed. All 13 existing real-media tests remain
  ignored.
- `npm run tauri -- build --no-bundle`: passed for the diagnostic. Native startup
  and effective-environment probes confirmed the mock reader/stub are selected,
  hardware reading is refused, and the real FLAC encoder/decoder works. The
  diagnostic retains the approved font and GTK/WebKit/Pango runtime versions.

## Isolated manual review

Launch `/tmp/cassette-cd-rip-titles-review-20261018/launch.sh` and choose
`/tmp/cassette-cd-rip-titles-review-20261018/output` as the rip destination.
The diagnostic has a fresh isolated profile/session bus, the published runtime,
a three-track synthetic reader, and a libdiscid stub that refuses lookup without
opening a drive. It does not replace the installed application or create menu
entries. Saved synthetic FLACs remain available for inspection.

1. Detect the mock CD; set album artist to `Diagnostic Artist` and album title to
   `Title Test`. Enter `Ark` for track 2 and click Rip while that input is still
   focused. Check `02 - Ark.flac`, Done, Last rip and the path tooltip.
2. Try Japanese in track 1 and whitespace in track 3. Repeat the rip and confirm
   it uses a new suffixed folder while preserving the first output.
3. Optionally use Scan ripped folder and check chosen titles in Songs. Saved
   TITLE tags can also be read with the diagnostic's `verify-rips.py`.

The automated browser/native tests exercise the two sides of IPC separately;
the complete native GUI flow awaits the user's manual result. Physical discs
are deliberately untested. Native diagnostic lookup is disabled; deferred lookup
responses were exercised in browser tests instead. No claim is made about the
exact original disc/session beyond the reproduced lookup race above.

The Settings/icons remain approved and intact. This CD-only checkpoint is
uncommitted, awaiting manual confirmation before commit, push and exact-commit
CI.
