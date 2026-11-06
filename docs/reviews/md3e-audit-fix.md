# Material 3 Expressive audit — fix pass

| | |
| --- | --- |
| Branch | `fix/md3e-audit` |
| Base | `d36599a` (`docs: add the Material 3 Expressive UI audit`) |
| Scope | `src/main.rs`, `src/ui/**`, `src/core/instances.rs` (localization only), `docs/design-system.md`, `docs/product-spec.md` |
| Audit | `docs/reviews/md3e-ui-audit.md` |
| Screenshots | `design/reviews/md3e-audit-fix/` (gitignored) |

## Fixed findings

### P0-1 — Responsive shell

- Replaced the wide `MaterialButton` primary action with a `Fab` named for the
  localized “New instance” action.
- `NavigationSuite` now uses `NavigationSuiteType::RailExpanded` and expands at
  `WindowWidthClass::Expanded` (840dp), so 600–839 is a collapsed rail and
  840+ is an expanded rail.
- The navigation model is five primary destinations: Home, Instances, Library,
  Accounts and Servers. Activity and Settings are top-bar actions.
- At compact widths (< 600dp) the top bar keeps theme and language visible and
  puts Activity/Settings in one `MaterialMenu` overflow, so nothing is clipped
  at 500px.
- `page_frame` now computes an explicit content width from the viewport, the
  navigation form and the gutters, caps it at 1120dp, and clips horizontally.
  This prevents padded surfaces from widening the shell. The page header and
  its action slot have `min_w_0` and the actions wrap below the title.
- The root and top-bar wrappers have `w_full`/`min_w_0`/`overflow_hidden`.
- At compact widths the suite's own bar lane was wider than the viewport and
  widened the page column, so the compact form renders the kit's
  `NavigationBar` directly in a clipped, viewport-width container.

Verified at 1440×900 (dark and light, zh), 1000×800 (dark, zh with expanded
rail), 800×600 (dark, en with collapsed rail) and 500×700 (dark, en with bottom
bar).

### P0-2 / P1-1 — Expressive scheme and expanded rail

- Added one source of truth in `src/ui/mod.rs`:
  `APP_THEME = ThemeId::Expressive` and `APP_CONTRAST = Contrast::Standard`.
  Both `src/main.rs` startup and `Yuhina::animate_theme` use it. No
  `ThemeId::Baseline` remains.
- The expanded navigation is an expanded rail rather than the deprecated
  drawer. `docs/design-system.md` and `docs/product-spec.md` now describe the
  rail.

### P1-2 — Destination count

- The bar/rail model is capped at five destinations. Activity and Settings are
  global top-bar actions, so the 600px rail fits and the compact bar does not
  exceed five items.

### P1-3 — Dialogs

- Every `MaterialDialog::content(...)` is wrapped in a `px(24)`/`pb(8)` body.
  The create dialog uses `dividers(true)`; the body scrolls and the footer
  actions stay visible.
- The create-instance dialog fits 800×600 (body scroll includes the loader
  version and memory sections; Cancel/Create remain visible).
- Add Account's primary action moved into the action row; the offline helper
  text is no longer clipped and chips have dialog padding.
- The Microsoft device-code dialog got the missing Copy affordance, and its
  code uses the headline type role.

### P1-6 — Localisation

- Added zh-CN/en-US keys for top-bar labels and accessible names, the
  navigation landmark, instance tab labels, “managed”, progress labels, phase
  names, playtime, loader names, empty/loading copy, and every `ErrorCode`.
- `InstallPhase` is stored in `InstallTask` as the enum and mapped to
  `instances.phase.*` at render instead of showing `label()`.
- The duplicate suffix is passed from the UI (`instances.copy_suffix`) instead
  of core appending `" (copy)"`.
- Loader names and playtime are localized in the UI; core still owns the
  machine-readable enum/data.
- `show_error` maps `YuhinaError::code` to localized copy, falling back to the
  core message only for an unmapped key.

### P1-4, P1-8, P1-9 — Typography, hero CTA, destructive actions

- Page titles use `HeadlineLarge.emphasized()` (32/40), the hero uses
  `DisplaySmall.emphasized()` (36/44), section labels use
  `LabelLarge.emphasized()`, body copy uses `BodyMedium`, supporting copy uses
  `BodySmall`, and logs use `BodySmall` with an 18dp line height. Cards/lists
  use `TitleSmall`/`TitleMedium`. The hand-written `text_size(px(...))` pairs
  in pages and dialogs are gone.
- The hero Play action is `Filled` (primary on the primary-container hero)
  instead of a tonal button with a 1.003:1 container boundary.
- Delete instance, remove content and delete backup use an error-coloured text
  button with “Delete”/“Remove” labels. Create uses “Create”, not “Confirm”.

### P1-7 — States

- Instances, Accounts and Servers distinguish loading from empty and show an
  inline error panel with Retry; existing data is retained when a refresh
  fails.
- Library shows a loading panel, a visible inline error with Retry (including
  the offline hint), then the empty state only after a successful empty
  search.
- Home news has separate loading, error/Retry and empty states; the recent
  instances section also waits for loading.
- Activity uses distinct running/install empty states, localizes phases, shows
  install errors and offers Retry.

### Extra fixes outside the audit

- Forge/NeoForge builds load when the create dialog opens with a preselected
  loader and Minecraft version (startup and dialog-open refresh).
- The version chip list puts the curated modded versions (1.21.1, 1.20.1,
  1.19.2, 1.18.2, 1.16.5) and the selected version first, then fills from the
  latest releases. The create dialog with Forge/1.20.1 shows the loaded
  `47.4.28 · Recommended` build.
- Settings motion chips now apply reduce-motion immediately.

## Screenshots

All captured from the release candidate with a dev data profile; host work area
is 1536×864 logical, so the 1440×900 request is clamped to a 1440×845 client.
That limit is the host display, not app content.

| File | Size | Checks |
| --- | --- | --- |
| `home-1440-dark-zh.png` | 1440×845 | Expressive teal, expanded rail, 3 equal cards, top-bar actions, right gutter |
| `home-1440-light-zh.png` | 1440×845 | Light scheme, all roles, top-bar actions, hero CTA |
| `instances-1000-dark-zh.png` | 1000×800 | Expanded rail, FAB inside rail, header actions fit |
| `instances-800-dark-en.png` | 800×600 | Collapsed 96dp rail, 5 destinations, top-bar actions, English copy |
| `home-500-dark-en.png` | 500×700 | Bottom bar, 5 destinations, overflow action, FAB, gutters |
| `dialog-create-800-dark-en.png` | 800×600 | Padded dialog body, scroll, footer visible, version chips |
| `dialog-create-forge-1440-dark-en.png` | 1440×845 | Full create dialog: curated versions and loaded Forge build |
| `dialog-add-account-800-dark-en.png` | 800×600 | Padded form, unclipped helper text, Add in action row |

## Commands and results

```text
cargo fmt --all                    # pass
cargo fmt --all --check            # pass
cargo clippy --all-targets         # pass, no warnings
cargo test                         # 42 passed, 0 failed
cargo build                        # pass
```

`CARGO_TARGET_DIR` was set to
`C:\Users\youzilm\Documents\projects\yuhina\target`.

## Remaining audit items

- P1-5: `System` reduced motion is still not resolved from the OS; runtime
  `On`/`Off` now apply immediately. No OS-preference reader was added.
- P1-4 follow-up: no persisted text-scale setting and no 100/150/200% capture;
  page geometry now uses the kit's rem type roles, but the setting is not
  exposed.
- P2-1 duplicate page title / static logo affordance.
- P2-2 shape literals (24/22/14/10) are still not tokenized.
- P2-3 native title bar still uses the OS dark caption in light mode; the
  screenshots capture the client area only.
- P2-4 Home grid is still a wrapping flex row; the 1440 layout happens to have
  three equal columns but the last row can still stretch at other widths.
- P2-5 Library still uses a text field plus a Search button, not `SearchField`.
- P2-9 snackbar placement/queueing is unchanged.
- P2-10 instance detail still has five header actions; P2-11 server removal is
  still unconfirmed; P2-12 raw UUID was shortened but active-row role pairing
  and hidden “Set active” are only partly addressed (the disabled button is now
  hidden).
- P2-13 Modrinth descriptions are not clamped; P2-14 icon sizes and contrast
  setting are unchanged; P2-15 page transitions are unchanged.
- No offline detector exists; inline error panels show an offline hint and
  Retry but do not probe connectivity.

## Deviations

- Compact navigation renders the kit `NavigationBar` directly instead of using
  `NavigationSuite`'s bar form. The suite's bar lane measured wider than the
  compact viewport and widened the page column; the direct bar is clipped to
  the viewport and keeps all five destinations reachable.
- `page_frame` uses an explicit width. GPUI/Taffy adds a padded surface's
  horizontal padding outside its `w_full` width, so percentage-width-plus-
  padding overflowed at 500px. The explicit width subtracts the navigation and
  gutters plus one extra 32dp, which yields symmetrical 48dp visible gutters at
  compact widths and keeps the 1120dp cap at 1440.
- Section/supporting text use the M3 roles `LabelLarge` (14/20) and
  `BodySmall` (12/16) rather than the design doc's old 13/18 pair, which is not
  on the 15-role scale.
- No dependencies were added.
