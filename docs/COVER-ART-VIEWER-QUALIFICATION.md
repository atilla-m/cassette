# Cover-art viewer qualification

Feature branch: `feature/cover-art-viewer` (viewer and close fix through `8954791bf83331ca85287b6a8c8ad43885418d4b`).

- **User-confirmed manual result (2026-09-25): PASS.** The user reported that the cover viewer worked after the close-path fix, then completed a check with the added square-cover fixture. The report confirms the overall viewer test; it does not itemize separate results for each close method or accessibility behavior.
- **Assisted diagnostic check:** three open → Close button → navigate cycles in the isolated native app passed. The outgoing viewer disappeared and navigation responded immediately each time. This is not a substitute for the user's manual result.
- **Automated checks already completed:** `npm run check` (0 errors/warnings), `npm run build`, unsigned `npm run tauri -- build --no-bundle`, diagnostic launcher/fixture hash checks, and Linux and Windows CI on `8954791bf83331ca85287b6a8c8ad43885418d4b` passed. The 13 real-media tests remained ignored.

The isolated fixture is under `src-tauri/target/diagnostics/cover-art-viewer/` and includes labeled wide (1600×900), tall (900×1400), and square (1200×1200) covers. It does not use the user's real library or installed app. Launch it with `src-tauri/target/diagnostics/cover-art-viewer/launch.sh` from the repository root.
