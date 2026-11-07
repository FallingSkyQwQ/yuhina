# Apple HIG implementation review

| | |
| --- | --- |
| Date | 2026-11-07 |
| Branch | `feat/hig-ui` |
| Base | `a775de3` (`merge: core feature gaps`) — the HIG review + core gap merge |
| Scope | Implement the HIG findings in `docs/reviews/hig-ui-review.md` while keeping Material 3 Expressive and `gpui-kit-material` |
| GUI | **Not launched.** No `cargo run`, process start, screenshot or focus-taking command was used. |
| Dependencies | Added `raw-window-handle = "0.6"` (already in the tree through GPUI) to reach the Win32 HWND for the native caption sync. No other dependency was added. |

## What changed

### P0

**HIG-01 — text scale.** `Settings.text_scale` is now the single input to
`theme::init_component_theme` (startup) and
`theme::advance_component_theme` (theme/scale changes). Settings has a segmented
`ConnectedButtonGroup` for 100/125/150/200% plus a Reset action. Changes are
persisted through `settings::save` and applied without a restart. Page gutters,
card/row padding, dialog padding, icon tiles and control-adjacent gaps are
`rem(...)`, so 200% grows the layout; the rail (96/220) and window minimum
(480×600) stay structural `px`. Geometry constants live in `pages.rs`
(`SPACE_*`, `RADIUS_*`, `PAGE_GUTTER`) and are covered by a rem-token test.

**HIG-02 — compact FAB.** The global floating FAB was removed. Home and
Instances already own Create in the hero/header, so there is no third entry
point and no control is covered. The compact content area no longer reserves a
floating-FAB lane.

### P1

| HIG | Result |
| --- | --- |
| 03 | One filled primary per view: Home hero Create, Instances header Create, detail Play. Recent/row Play is Tonal; Import/Repair are Tonal/Text; destructive actions moved into menus + confirmations. |
| 04 | Actions are back in `TopAppBar::actions` (no absolute overlay). The bar has no duplicated title; the page/instance H1 is the only visible title and the native window title names the page/instance. Theme/language controls live only in Settings. Compact Activity/Settings are a labelled “More” menu. |
| 05 | Create/account/server/import dialogs use `DialogSize::Medium` on desktop and `DialogSize::FullScreen` below 600dp (with a close button). Bodies use `track_body_scroll` and a 16dp bottom pad. Version/loader-build are `ExposedDropdown`; loader is `ConnectedButtonGroup`; memory is `MaterialSlider`; account type is `ConnectedButtonGroup`. Confirm buttons are `.loading(...)` + `.disabled(...)`; fields use `TextField::error` + `supporting_text`. |
| 06 | Library uses `SearchField` with a leading search glyph and trailing clear. Typing debounces 250 ms, Enter searches immediately, a generation counter drops stale responses, and a result count is shown. |
| 07 | Instance, account, server, library result, backup, Java and content/file rows use `ListItem` (leading/trailing slots, divider, ripple, focus). The row is the navigation target; the trailing action is a separate sibling focus stop. Recent/news items use `MaterialCard`; the feed is a real 3/2/1 grid. |
| 08 | Detail Back is an `IconButton` in the bar navigation slot; content Back was removed. `set_page` no longer clears `selected_instance`; tab and per-page `ScrollHandle` survive navigation. Escape/Alt+Left are **not** wired (see remaining). |
| 09 | `DeleteInstance { id, name }`, `DeleteServer { id, name }` and `RemoveAccount { id, name }` name their target, use the error tone, and always keep Cancel. Server/account removal moved from a bare row icon to a row overflow menu. |
| 10 | Toasts are a `VecDeque` queue (duplicates dropped). Errors are `SnackbarDuration::Indefinite` with Retry + Close; successes use the short timer. The snackbar is centred, capped at 672dp and sits above the compact navigation bar. |
| 11 | Window minimum 480×600, initial bounds clamped to the primary display and centred, title = page or open instance name. Windows DWM immersive dark mode follows the app theme (startup and on theme change); macOS/Linux window appearance is set through GPUI. |
| 12 | One `WindowWidthClass::of(viewport width)` model is used by the shell and the pages; no page divides the viewport by rem. Columns are 1/2/3 by class. |
| 13 | Create instance, offline account, LittleSkin login, server add, content removal, import, folder open and all installs/launches run through `spawn_task`/`background_executor`. Busy dialogs disable both actions and guard in the handler. |
| 14 | `ThemePreference::System` reads `AppsUseLightTheme` on Windows (fallback: GPUI appearance). `ReduceMotion::System` reads `SPI_GETCLIENTAREAANIMATION`. `LanguageMode::System` reads `GetUserDefaultLocaleName`. Settings expose System for all three. |
| 15 | The Java auto-download switch is now a `ListItem` with a visible headline, supporting text, `ListTrailing::Switch`, and a full-row toggle. |
| 16 | Status is quiet by default: Running is the only green chip, Not installed is the warning chip, Installed renders nothing. Active account uses `selected(true)` plus a secondary-family badge (one accent family). |
| 17 | `card_grid` chunks items into equal columns and pads short rows with empty cells, so the last card never grows. News is a card grid, not a full-width banner, with 2-line clamps. |
| 18 | Detail header keeps Play (Filled) + Install/Repair (Tonal) and moves Folder/Duplicate/Export/Delete into a `MaterialMenu`; Delete is separated and confirmed. Tabs scroll horizontally at narrow widths/200%. |
| 19 | Page code uses 8pt spacing and 16/24/32 radii tokens; icons are 20/24/32. Logs and file preview are monospace. |
| 20 | One tonal container per section; list rows carry dividers, no nested card inside a card. |
| 21 | Loading uses `LoadingIndicator`/`LinearProgress`; errors use `notice_banner` + Retry; empty states gained next actions and the Activity installs can be cleared. |
| 22 | Removed the hard-coded root `"Roboto"`; the kit type scale's family is used. Metadata that identifies a row is `BodyMedium`; log output is monospace. |
| 23 | The compact More trigger is now a labelled text button (“More”) with leading icon and a badge, listing Activity and Settings; the wide bar exposes both destinations. |
| 24 | A 140 ms fade-through on page/detail change is skipped when `reduce_motion` is on; `System` resolves from the OS. Page scroll is restored per destination. |
| 25 | Window minimum + work-area clamp + centre (see 11). |
| 26 | New i18n keys for text scale/System/More/Back/Active/Running/Not installed/Retry/busy/status formats; fragment concatenation replaced by `{...}` placeholders; guest greeting uses `home.greeting_guest`; missing keys panic in debug/test and were added to the parity test. |
| 27 | Raw UUID removed from account rows; server supporting text is two lines with latency in the status; library/news descriptions clamp to 2 lines. |
| 28 | Six detail tabs scroll horizontally (minimum strip width 520rem) instead of clipping at 500/200%. |

## Component mapping results

| Bespoke pattern | Replacement |
| --- | --- |
| Instance/account/server/library/backup/Java/content rows | `ListItem` + `ListLeading`/`ListTrailing`, `container(SURFACE_CONTAINER_LOW)`, `divider` |
| Recent/news cards and empty states | `MaterialCard::new(id, CardVariant::Filled)` + equal-cell grid |
| Library search field + button | `SearchField` + clear `IconButton`, 250 ms debounce + Enter |
| Theme/language/motion/download/text-scale chips | `ConnectedButtonGroup` (segmented, single) |
| Account type chips | `ConnectedButtonGroup` |
| Version / loader build / library target / instance memory | `ExposedDropdown` |
| Memory presets | `MaterialSlider` (settings), `ExposedDropdown` (per-instance) |
| Status chips | `feedback::status_chip` / `Badge` |
| Error panels | `notice_banner(NoticeKind::Danger)` + Retry |
| Loading text | `LoadingIndicator` / `LinearProgress` |
| Top-bar overlay | `TopAppBar::actions` |
| Compact FAB | removed; no safe-area overlap |
| Snackbar wrapper | `Snackbar::position(Center)` + queue |
| Detail Back / action row | bar `IconButton` + 2 actions + `MaterialMenu` |
| Java switch | `ListItem` + `ListTrailing::Switch` |

Deliberate exception: `CardGroup` is a single-select radio/card group, not a
layout. Using it for navigation cards would announce a false selection. The
feed uses equal-column `MaterialCard` cells instead; this is the one mapping
from the report that was not used literally.

## Core APIs wired

- `settings.text_scale` + `clamp_text_scale` — startup, settings control,
  reset, persistence.
- `accounts::refresh_authlib(paths, id)` — called before a LittleSkin account
  is made active, then accounts reload.
- `modpack::import_local_with_java` / `install_modrinth_with_java` — the
  user's `JavaSettings` is passed.
- `install::install_loader_with_output` — installer stdout/stderr updates the
  install task message and the Activity card; the last line survives progress
  ticks.

## Verification (no GUI)

Run from the worktree with `CARGO_TARGET_DIR` pointing at the shared target
directory:

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, no warnings |
| `cargo test` | pass, 61 tests (56 core + 5 UI: grid columns, rem tokens, dialog size, i18n key resolution, placeholder parity) |
| `cargo build` | pass |
| `cargo run` / app start / screenshot | **not run by design** |

## Remaining HIG items and known gaps

1. **No visual 100/125/150/200% matrix.** The code is rem-based and the tokens
   are tested, but no window was launched, so 200% at 500×700 and both themes
   are unverified visually.
2. **Escape / Alt+Left and a real history stack are not implemented.** Back is
   the bar arrow; selecting another destination preserves the detail, which is
   the HIG-08 spatial requirement, but keyboard back is missing.
3. **Live OS appearance changes are not observed.** Theme/locale/reduced-motion
   `System` values are resolved at startup and when the setting is selected;
   there is no `observe_window_appearance` subscription.
4. **Defaults remain core defaults.** A fresh profile still starts Dark /
   zh-CN because `core::settings::Settings::default` says so; System is
   available but not the default. Changing that is a `src/core` edit and was
   out of scope.
5. **OS reduced motion is Windows-only.** Other platforms keep the existing
   “animate” default; the System option is honest but inert there.
6. **Dialog body scroll handle is shared across dialog kinds.** A tall create
   dialog followed by a short confirmation could retain a scroll offset; this
   was not split per dialog.
7. **Trailing row actions at 200%/500dp are dense.** Text truncates inside the
   ListItem and action buttons stay outside it, but no clipped-rect test exists.
   A compact row could stack its action under the row in a follow-up.
8. **Activity does not render modpack-import progress.** Instance installs show
   the installer output; modpack imports still use the Library loading state.
9. **`CardGroup` exception** (see above).
10. **No offline detector.** Error banners keep the existing offline hint; the
    review’s “only show it for network codes” item is not fully addressed.
11. **Library search is page-local.** No global shortcut/search entry point;
    this matches the audit’s “acceptable, document the decision” note.
12. **Native caption sync is FFI.** It is behind `#[cfg(target_os = "windows")]`
    with a documented safety comment, but it was not visually confirmed.
13. **`instances::load` in `start_install` is a small synchronous read** before
    the background install starts; all heavy work (download, hashing, loader)
    is off the render thread.

## Files changed

- `Cargo.toml`, `Cargo.lock` — `raw-window-handle`.
- `src/main.rs` — OS-resolved theme/language/motion, text scale, work-area
  clamp, minimum size, page title, caption sync.
- `src/ui/mod.rs` — state for text scale, toasts, menus, scroll, sliders,
  pages; async/busy actions; authlib refresh; modpack Java settings; installer
  output; top bar, snackbar queue, page transition, Activity badge.
- `src/ui/pages.rs` — page rewrite on kit components, tokens, grids, rows,
  status/badges, dialogs entry points, tests.
- `src/ui/dialogs.rs` — Medium/FullScreen dialogs, dropdown/group/slider
  controls, named destructive targets, busy state, inline errors, tests.
- `src/ui/i18n.rs` — new keys, placeholders, debug assertion, parity/resolution
  tests.
- `src/ui/platform.rs` — OS appearance, locale, reduced motion, DWM caption.
