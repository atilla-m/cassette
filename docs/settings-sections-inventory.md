# Settings section inventory

Inventoried before moving controls from approved sidebar-icons commit
`8cef212c857f8891163ac8279cc5cbe02687c0d5`, whose parent is released beta.5.1
commit `4399ac3756b724e6fdd3efd40c4addb1564da6cb`.

| Destination | Existing controls, conditional states and explanatory text |
| --- | --- |
| Appearance | Cassette Teal, Glacier and Obsidian theme buttons and descriptions; Songs list numbers; album, artist and genre play totals; album-track play counts; always-shown album track numbers note. |
| Playback | Playing/paused/idle indicator; shuffle, repeat, volume and queue readouts; track-change notification preference when Linux notifications are supported, otherwise the unavailable explanation; conditional Test Notification and result/error; Clear Queue; Keyboard Shortcut Help; disabled Reset Playback State placeholder; auto-find lyrics preference and Auto/Manual status. |
| Library | Cache status; selected folder; track, album, artist and genre totals; last scan; Scan/Rescan Library and Change Library Folder, including scanning-disabled states; Open Library Health; Library Health, Keyboard Shortcuts and Mix Builder utilities; disabled Clear Library Cache placeholder. |
| Listening history | Full-history export explanation and export; import picker and preview; busy/error/result messages; matched/unmatched/ambiguous track totals, new/duplicate events and undated plays; overall and per-track conflicts; audio-identical candidate selection and retained-history association, including import-first explanation; load more references; Import new history with conflict/busy guards; Cancel preview. |
| Updates | Version; automatic-check preference and 24-hour explanation; installation type and package-specific explanation; manual check and busy state; conditional AppImage launcher status/current path, add/refresh/remove, busy guards and results/errors; runtime-unavailable explanation; last successful check; available version and notes; download/install progress; signed AppImage confirmation/install or package download; Later; status/errors and signature/package-manager/user-data explanation. |
| About | Existing version, application description, platform and technology stack; existing audio-file editing explanation, retained verbatim. |

Native/platform conditions remain unchanged:

- Notifications retain `platformCapabilities?.linuxNotifications !== false`.
  Test Notification also requires the saved notification preference to be on.
- AppImage menu integration retains
  `updateRuntime?.packageKind === "appimage" && !updateRuntime.development`;
  removal also requires an installed launcher. Native command checks remain
  unchanged.
- Update checking keeps runtime/busy guards, and install versus download keeps
  `canSelfInstall`, progress and confirmation behavior.
- History import retains busy/conflict guards, approval-token confirmation,
  candidate matching and retained-history requirements.

No controls, defaults, disabled placeholders or explanatory copy are removed.
Cassette Teal remains the fresh-profile default; existing preference storage is
unchanged. The approved icons and all other screens stay unchanged.

The selected section is page state for this application session. All six panels
stay mounted during tab switches; hidden panels are removed from navigation and
the accessibility tree. Arrow keys, Home/End and native Enter/Space work within
the tablist. Playback shortcuts yield only the tab keys to this new control;
existing help and modal dismissal handling remain available.

Validation performed:

- `npm run check`: zero errors and zero warnings. `npm run build`: passed.
- Existing player-navigation and album-browse regressions: 11 passed.
- Playwright against the production frontend with mocked native calls: ordered
  tabs and ARIA relationships; one visible/six mounted panels; roving focus,
  arrow wrapping, Home/End, Enter/Space and Tab; help/Escape; session selection;
  unchanged playing track and queue; no native actions from switching tabs.
- History: hidden async preview/import/export completion, 55-reference preview
  pagination, retained association choices, cancellation, matching, global and
  per-reference conflicts, import-first/busy guards, approval token, results
  and errors. All data and native effects were synthetic/mocked.
- Conditional controls: supported/unsupported notifications and Test
  Notification; lyrics preference; Library Health/Mix Builder return navigation;
  scanning across tabs; AppImage add/refresh/remove versus package/development
  guards; update confirmation cancellation, progress and errors; update-notice
  Review opens Updates even from Library Health. Native effects were mocked;
  no real launcher or update was written.
- Three themes at 1280×720 and 800×600: screenshots inspected; all tabs fit
  (two rows at 800px), with no page-width overflow. Visibility preferences and
  retained themes survive navigation; a new app session starts on Appearance.
- Preservation audit: all eight original Settings section blocks match the
  approved icon commit verbatim. Existing page CSS, sidebar icons, player bar,
  native source and package metadata are unchanged. All 13 real-media tests
  remain ignored. Earlier worktree changes and polish/Windows refs are intact.
- `git diff --check`: passed. `npm run tauri -- build --no-bundle`: passed for
  the isolated diagnostic; startup smoke check passed using synthetic media.

The diagnostic is `/tmp/cassette-settings-tabs-review-20261018/launch.sh`.
It uses isolated XDG directories and a copy of the verified published AppImage
runtime. Only its Cassette binary is replaced. A native probe confirmed the
desktop font (Comfortaa Medium 11), GTK 3.24.41, WebKit 2.52.6 and Pango 1.52.1,
matching the approved icon review's rendering environment.

The user approved committing and pushing this Settings-only checkpoint on
`feature/settings-section-tabs`, based on approved icon commit `8cef212`.
Individual manual-check results were not supplied; native dialog/notification
delivery is not claimed as automatically verified. CI is checked against the
resulting checkpoint commit.
