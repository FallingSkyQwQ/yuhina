# Yuhina — Material 3 Expressive UI audit

| | |
| --- | --- |
| Date | 2026-11-07 |
| Branch / HEAD | `feat/rewrite-m3e` / **`117ac71516589b8b991e24905cb47a2749b1c059`** (audited) |
| Worktree note | During the audit the shared worktree advanced to `f426d71` and an unresolved `feat/modpack-srv` merge appeared (`src/ui/mod.rs`, `src/core/install.rs`, `src/core/mod.rs`, `docs/architecture.md` conflicted). This report is pinned to the clean `117ac71` tree that produced the screenshots and the cited lines; re-check after the merge is resolved. |
| Kit | `gpui-kit-material` `26c0cb1713311280919113e14b552034e492d573` (`Cargo.toml:12`) |
| Scope | `src/ui/**`, `Cargo.toml`, `docs/design-system.md`, `docs/product-spec.md`, `design/reviews/rewrite-m3e/*.png` |
| Constraint | No application code was modified by this audit. This report is the only file written; the concurrent merge is not part of the audit. |

## Method

1. Read the four project documents, all four `src/ui` files, `Cargo.toml`, and the
   kit's declared M3E coverage (`docs/COVERAGE.md`, `README.md`) at the pinned revision.
2. Opened every committed screenshot and measured colours and layout positions from
   the PNGs. All committed captures are 1536×864; `instances-rail.png` is the
   expanded-width (840–1199 dp) capture.
3. Ran the existing binary against a **temporary data profile** at 1440×900,
   1000×800, 800×600, and 500×700 in English, without touching the real
   `%APPDATA%\Yuhina` state. These live captures are referenced as "live check"; they
   are not committed because this task permits only this report.
4. Recomputed contrast ratios from the baseline scheme values (WCAG formula).
5. Checked the official guidance. Several `m3.material.io` pages are client-rendered
   or gone:
   - `https://m3.material.io/blog/building-with-m3-expressive` returns no body to a
     plain fetch; its indexed content and the official Google I/O transcript were used.
   - `https://m3.material.io/components/navigation-suite/overview` returns **404**.
     Navigation-suite rules are therefore taken from the official
     [navigation rail](https://m3.material.io/components/navigation-rail/overview),
     [navigation drawer](https://m3.material.io/components/navigation-drawer/overview),
     and [breakpoints](https://m3.material.io/foundations/layout/breakpoints) pages,
     plus the kit's own m3e port.

## Executive summary

This is structurally a real Material 3 Expressive application: every visible control
is a kit component, every colour is a `ColorRole`, the tonal-surface hierarchy is
present, and the shape hierarchy is deliberate. That is the good news.

The release is not ready. Two foundations fail globally:

1. **The app ships the Baseline colour scheme, not Expressive.** `src/main.rs:26` and
   `src/ui/mod.rs:572` both build `ThemeId::Baseline`, while
   `docs/design-system.md:4` and `docs/architecture.md:82` promise
   `ThemeId::Expressive`. Every screenshot is baseline purple (`#6750A4` /
   `#EADDFF`) instead of the Expressive teal (`#006B5A` light primary).
2. **The adaptive shell collapses at the sizes the product promises.** At 800×600
   only 4 of 7 destinations are reachable, the rail's "New instance" button spills
   across the content, and the page action is off-screen. At 500×700 the bottom
   navigation bar is not visible at all. In English at 1440×900 the right gutter,
   the last memory chips, and the entire top-bar action group are clipped.

The first is a one-line fix repeated in two places. The second is a layout-contract
bug: `page_frame` and the page rows do not establish `min_w_0` / horizontal
containment, so long content widens the root, and the top bar and navigation are
pushed out with it. Until that is fixed, the app does not meet its own
"usable at 800×600" definition of done.

**Release call: block on P0-1 and P0-2. Fix P1-1 … P1-4 before calling the phase
complete.**

## Foundation scores

| Foundation | Score | Justification |
| --- | ---: | --- |
| Colour | **4 / 10** | Excellent role discipline and no hex literals, but the wrong scheme is active, the hero pairs a tonal button with a primary container at 1.003:1, and there is no contrast-level setting. |
| Typography | **3 / 10** | 11 distinct hard-coded sizes; 20 of 46 explicit `text_size` uses are off the 15-role scale; most styles omit a line height; `px()` text ignores the kit's rem scale and 200% text scaling. |
| Shape | **6 / 10** | Clear 32 → 24 → 20 → 14 hierarchy, correct pills and 28 dp dialogs, but 24/22/14/10 are not M3 corner tokens and cards are not on the card radius. |
| Motion | **5 / 10** | Expressive springs, ripples, state layers and the theme transition all come from the kit. `System` reduced motion is ignored and runtime changes require a restart; no page-level continuity. |
| Components | **5 / 10** | Buttons, chips, text fields, switch, tabs, avatar, progress and snackbar are correctly the kit's. NavigationSuite is configured against M3E, the primary action breaks the rail, and dialogs are unpadded. |
| Layout / adaptive | **2 / 10** | 800×600 and compact fail; 1000×800 is cramped by a 360 dp drawer; English at 1440 clips the right gutter and top-bar actions; content max width is never reached because of the drawer. |
| Accessibility | **4 / 10** | Icon-only labels and kit focus rings are in place, but text scaling, system reduced motion, inline form errors and several accessible names are not. |
| i18n | **4 / 10** | Key parity is complete and tested, but raw English leaks through phases, errors, copy suffixes and accessible names, and +30% English breaks the layout. |
| Visual craft | **5 / 10** | Spacing rhythm and tonal grouping are good; duplicate page titles, 2+1 card rows, clipped panels, a dark native title bar in light mode, and cramped 1000 px rows pull it down. |
| **Overall** | **4.2 / 10** | A good component skeleton with two foundation-level release blockers. |

---

## Severity-ranked findings

### P0-1 — The responsive shell collapses at 800×600 and below; navigation becomes unreachable

**Evidence**

- `design/reviews/rewrite-m3e/instances-rail.png` — the capture named "rail" still
  shows a labeled **drawer** at the expanded width class; the instance meta already
  wraps `4096 MB` onto a second line and the action cluster is cramped.
- Live check at 800×600 (`YUHINA_SIZE=800x600`, English): the collapsed 96 dp rail's
  "New instance" button is wider than the rail, overflows into the page, and is
  clipped at both edges. Only Home, Instances, Library and Accounts are visible;
  Servers, Activity and Settings are below the window with no rail scroll. The page
  header's "New instance" action is a purple sliver at the right edge; the top-bar
  theme/language/settings actions are completely gone.
- Live check at 500×700: no bottom navigation bar is visible; only the theme action
  remains in the top bar; the hero and page content are clipped on the right.
- `src/ui/mod.rs:1366-1392` — a full-width `MaterialButton` is passed as
  `primary_action` to `NavigationSuite`; the suite is then placed inside
  `div().flex_1().min_h_0()` with no width containment.
- `src/ui/pages.rs:20-42` — the scroll frame uses `.w_full()` plus an inner
  `.flex_1().max_w(px(1120.)).p(px(32.))`; the padding is consumed on the right and
  content widens the root instead of being contained.
- `src/ui/pages.rs:44-78` — the page header is a `flex_wrap` row, but the action slot
  has no `min_w_0` and cannot shrink, so it is pushed off-screen at narrow widths.

**Official rule**

- Navigation must change form across breakpoints: bar below 600 dp, rail from 600 dp,
  expanded rail on larger windows.
  <https://m3.material.io/foundations/layout/breakpoints>
- A collapsed navigation rail is 96 dp wide and must not be hidden; a rail's primary
  action is a FAB / extended FAB, not a text button.
  <https://m3.material.io/components/navigation-rail/overview>
- The product's own definition of done requires the app to remain usable at 800×600
  (`AGENTS.md:43`, `docs/product-spec.md:119-122`).

**Why this deviates**

The primary action is treated as a normal 40 dp button in a 96 dp rail. It has no
width constraint, so it paints across the rail boundary. The seven rail items at the
kit's vertical pitch exceed a 600 px window; there is no rail scroll. Separately, the
page content's min-content width expands the flex root, and because the top bar is a
sibling of the suite, the top bar stretches to that wider root and its trailing
actions are pushed beyond the viewport. The result is not a cosmetic squeeze: three
destinations and every global action are unreachable.

**Concrete fix**

1. Replace the `MaterialButton` primary action with a `Fab` (56 dp, icon + accessible
   name). If the kit requires an extended FAB when the rail is expanded, switch it
   with `on_type_change` rather than using a full-width button in every form.
2. Reduce the primary destination count to **5** for the bar (see P1-2), or make the
   rail content vertically scrollable and keep the FAB pinned.
3. Fix the width contract in `page_frame` (this finding): outer scroll container
   `.overflow_x_hidden()`; a full-width wrapper with `.px(px(32.))`; the inner column
   `.w_full().max_w(px(1120.)).mx_auto().min_w_0()`.
4. Add `.min_w_0()` to the page header's title block and action slot; at narrow
   widths let actions wrap onto a second row instead of overflowing.
5. Keep the top bar at the viewport width (`w_full`, `min_w_0`, no shared root
   expansion); verify theme/language/settings remain visible at 500, 800 and 1000.
6. Add 800×600 and 500×700 to the screenshot matrix in both languages.

---

### P0-2 — The active colour scheme is Baseline, not Expressive

**Evidence**

- `src/main.rs:26` — `let scheme = ThemeId::Baseline.scheme(mode, Contrast::Standard);`
- `src/ui/mod.rs:572-580` — `animate_theme` also builds `ThemeId::Baseline`, so even a
  corrected initial scheme would revert on the first theme toggle.
- Screenshots `home-native.png`, `home-light.png`, `settings-native-dark.png` show
  baseline purple: hero `#EADDFF` and primary `#6750A4` in light, `#4F378B` /
  `#D0BCFF` in dark.
- `docs/design-system.md:4` declares "The palette is the `ThemeId::Expressive` scheme";
  `docs/architecture.md:82` says `theme::init_component_theme` is called with
  `ThemeId::Expressive`. The code contradicts both.
- The kit's Expressive light primary is `#006B5A` (teal), with
  `#95F4DC` as the light primary container; none of that is present.

**Official rule**

- M3 Expressive is a distinct, vibrant scheme with updated on-container colours and
  tonal surfaces; applying an expressive theme means selecting the expressive colour
  scheme, not only using expressive components.
  <https://m3.material.io/blog/building-with-m3-expressive>
- M3 colour roles are the contract for light/dark and contrast levels; the selected
  scheme supplies every role.
  <https://m3.material.io/styles/color/roles>

**Why this deviates**

The app is visually and semantically Material 3 baseline. Every contrast pair still
works because baseline is a valid scheme, but the declared product identity,
screenshots, design doc and "Expressive" claim are all false. The accent usage in the
pages (primary purple, secondary lavender, tertiary pink) is baseline's, not
Expressive's teal/pink pairing. This is a global foundation failure, not a component
detail.

**Concrete fix**

```rust
// one source of truth, used by both startup and animation
pub const APP_THEME: ThemeId = ThemeId::Expressive;
const APP_CONTRAST: Contrast = Contrast::Standard; // make configurable later
```

Use `APP_THEME.scheme(mode, APP_CONTRAST)` in `main.rs` and in `animate_theme`.
Re-capture every screenshot after the change. If the team actually wants baseline
purple, change the design doc and stop claiming Expressive; do not leave the mismatch.
Note that Expressive's teal is not a "Minecraft-green accent" — it is the scheme the
doc already names — so the existing no-green rule is not a reason to keep baseline.

---

### P1-1 — Expanded navigation uses the deprecated drawer at the wrong breakpoint

**Evidence**

- `src/ui/mod.rs:1375-1379` — `NavigationSuite` with
  `.expanded_type(NavigationSuiteType::Drawer)` and
  `.expanded_from(Some(WindowWidthClass::Expanded))`.
- `design/reviews/rewrite-m3e/instances-rail.png` — at the expanded width class the
  labeled drawer is still shown; it is not a rail. The file name is misleading.
- Live check at 1000×800 English: drawer takes roughly 44% of the window; the content
  column has no right gutter and the empty-state sentence is clipped.
- `src/ui/mod.rs:1378` — the suite is explicitly pinned to `Drawer`; no
  `RailExpanded` path exists.

**Official rule**

- "The navigation drawer is no longer recommended in the Material 3 Expressive update.
  For those who have updated, use an expanded navigation rail."
  <https://m3.material.io/components/navigation-drawer/overview>
- Breakpoints: compact uses a bar or modal expanded rail; medium uses a collapsed
  rail; expanded uses a modal or standard **expanded rail**; large uses an expanded
  rail.
  <https://m3.material.io/foundations/layout/breakpoints>
- The expanded rail is 220–360 dp, so it can preserve content width where a 360 dp
  drawer cannot. <https://m3.material.io/components/navigation-rail/overview>

> The dedicated `m3.material.io/components/navigation-suite/overview` page currently
> returns 404. The suite's adaptive rule is defined by the navigation rail and
> breakpoints pages above and by the kit's own m3e port.

**Why this deviates**

The app matches `docs/design-system.md:101` but not the official M3E standard the doc
claims to implement. At 840–1199 the drawer consumes 360 of 1000 px, leaving a
cramped content column; at 1440 it consumes 360 where the expanded rail would use
220–360 and leave more room. The "rail" capture demonstrates the squeeze.

**Concrete fix**

- Set `.expanded_type(NavigationSuiteType::RailExpanded)`.
- Expand at `WindowWidthClass::Expanded` (840 dp), not `Large`; that gives collapsed
  rail at 600–839, expanded rail at 840–1199, and expanded rail at 1200+.
- Use a FAB/extended FAB as the rail's primary action (see P0-1).
- Update `docs/design-system.md` and the "instances-rail" screenshot name after the
  change; the current doc is stale relative to May 2025 M3E.

---

### P1-2 — Seven destinations exceed the navigation bar limit and overflow the 600 px rail

**Evidence**

- `src/ui/mod.rs:1428-1441` — seven `NavItem`s: Home, Instances, Library, Accounts,
  Servers, Activity, Settings.
- Live check at 800×600: only four destinations are visible; the rest are outside the
  window. Live check at 500×700: the bar is not visible at all (P0-1).
- `instances-rail.png` shows all seven in the drawer, which is why the problem is
  hidden in the expanded drawer form and appears as soon as the form becomes a rail.

**Official rule**

- Navigation bars "can contain 3–5 destinations of equal importance".
  <https://m3.material.io/components/navigation-bar/overview>
- Navigation rails contain 3–7 destinations, but the rail must fit the window. In the
  800×600 check the rendered item pitch left only four of seven visible; seven items
  plus the primary action and top padding exceed the available height.
  <https://m3.material.io/components/navigation-rail/overview>

**Why this deviates**

The product has seven top-level destinations and uses one adaptive component for all
of them. That works only in the drawer form. In the bar form it violates the 3–5
limit; in the 600 px rail form it overflows. The user loses Servers, Activity and
Settings at the exact sizes the product says must work.

**Concrete fix**

- Keep 5 primary destinations for the bar: Home, Instances, Library, Accounts,
  Servers. Move Activity and Settings into an overflow destination ("More") or into
  the top bar's overflow menu.
- If the desktop rail must keep all seven, make the rail scroll vertically and reduce
  the item pitch; still keep the bar at 5.
- Add a nav-model test: `bar_items.len() <= 5`, `rail_items.len() <= 7`, and a
  600 px height fit test.

---

### P1-3 — Dialog content is unpadded, clipped, and the create dialog cannot fit 800×600

**Evidence**

- `design/reviews/rewrite-m3e/dialog-add-account.png` — the helper text is clipped at
  the left edge ("3–16 位字母…" starts under the dialog border), and the field
  underline runs to the dialog edge.
- `design/reviews/rewrite-m3e/dialog-create-instance.png` — version chips start at the
  dialog's left edge; the dialog is ~685 px tall at 1440, before a subtitle or error.
- `src/ui/dialogs.rs:102-163, 290-295, 380-397` — content is passed directly to
  `MaterialDialog::content` with no horizontal padding.
- The kit documents the content slot as "under the text, full width (e.g. a list of
  items)" (`crates/material/src/components/dialog.rs:255`); padding is the caller's
  responsibility.
- `src/ui/dialogs.rs:166-180` — `DialogSize::Small` + `.max_width(560.)` does not add
  content padding or scrolling.

**Official rule**

- Basic dialogs use 24 dp padding on all sides, 16 dp between headline and content,
  24 dp between content and actions, and a dynamic height that must stay within the
  window. <https://m3.material.io/components/dialogs/specs>
- Error messages about dialog fields "should always appear inline where they occur".
  <https://m3.material.io/components/dialogs/guidelines>
- On compact windows use a full-screen dialog; on medium/expanded use a basic dialog.
  <https://m3.material.io/components/dialogs/guidelines#expanded-breakpoint>

**Why this deviates**

The kit leaves content full width so callers can place a list edge-to-edge; this app
places fields, chips and helper text there and adds no padding. The result is clipped
text and chips touching the dialog edge. The create dialog's 8 version chips + 5
loader chips + 4 memory chips also make it taller than an 800×600 window, and there
is no scroll container.

**Concrete fix**

```rust
let content = div()
    .flex()
    .flex_col()
    .gap(px(16.))
    .px(px(24.))   // M3 dialog horizontal padding
    .pb(px(8.));
```

- Add `.body_scroll(handle)` and `.dividers(true)` to the dialog (the kit supports
  both), or use a `ScrollHandle` and a max height.
- At `< 600` dp width use `DialogSize::FullWidth`; at 800×600 use a scrollable basic
  dialog or full-screen.
- Replace the version chip list with an `ExposedDropdown`/`SearchField`; replace
  memory chips with a slider or stepper. This also fixes the 685 px height.
- Keep action buttons in the action row (see P1-9).

---

### P1-4 — Typography is off the M3 scale, omits line heights, and cannot scale

**Evidence**

- `src/ui/pages.rs:62-74` — page title 32/40 medium, subtitle 14/20.
- `src/ui/pages.rs:96-102` — supporting text 13/18 (not a type role).
- `src/ui/pages.rs:225-234` — card title 16 with no line height; meta 13/18.
- `src/ui/pages.rs:320-323` — news title 15 with no line height.
- `src/ui/pages.rs:548-550` — list title 17 with no line height.
- `src/ui/pages.rs:684-687` — detail title 30/38 (not a type role).
- `src/ui/dialogs.rs:334-335` — device code 34 px (not a type role).
- Grep of `src/ui/pages.rs` + `src/ui/dialogs.rs`: 46 explicit `text_size(px(...))`
  calls across 11 sizes; 13 px is used 13 times, and 15, 17, 20, 30 and 34 are all off
  the 15-role scale. 20 of 46 are off-scale.
- `src/main.rs:27` and `src/ui/mod.rs:580` pass `1.0` as the kit scale; page text is
  `px()`, so `init_component_theme`'s rem scale cannot affect it.
- `src/ui/mod.rs:1420` hard-codes `.font_family("Roboto")` without bundling Roboto;
  Windows does not ship it, so the declared face is not guaranteed.

**Official rule**

- M3 has 15 baseline roles and 15 emphasized roles; each has a defined size, line
  height, weight and tracking. <https://m3.material.io/styles/typography/type-scale-tokens>
- Text must support user scaling; WCAG 1.4.4 requires up to 200% without loss of
  content or functionality.
  <https://www.w3.org/WAI/WCAG21/Understanding/resize-text.html>
- Body text should be limited to roughly 50–75 characters per line.
  <https://m3.material.io/styles/typography/overview>

**Why this deviates**

The UI uses a private type scale (12/13/14/15/16/17/20/30/34/36) instead of the M3
roles. Some roles are close enough to pass at a glance (14/20), but the missing line
heights and the off-scale headings make the hierarchy inconsistent and fragile.
Because everything is `px()`, the kit's rem-based 100–200% scale changes only the kit
controls, not the page text; at 200% the layout will mix 2× kit text with 1× page
text. Roboto's absence on Windows also shifts metrics.

**Concrete fix**

- Replace every `text_size`/`line_height` pair with `TypeRole`:
  - page title: `TypeRole::HeadlineLarge.emphasized()` (32/40, 500)
  - hero: `TypeRole::DisplaySmall` (36/44)
  - section label: `TypeRole::LabelLarge` (14/20)
  - body: `TypeRole::BodyMedium` (14/20)
  - supporting: `TypeRole::BodySmall` (12/16)
  - card/list title: `TypeRole::TitleMedium` / `TitleSmall` (16/24, 14/20)
  - chip: `TypeRole::LabelMedium` (12/16)
- Apply both `font_size` and `line_height` from the `TypeScale`; do not omit line
  height.
- Use `rem()` for page geometry that should scale, and add a persisted text-scale
  setting; pass it to `init_component_theme` and `advance_component_theme`.
- Bundle the family (or use a documented system stack) and test at 100%, 150% and
  200% in zh-CN and en-US at 800×600.

---

### P1-5 — Reduced motion is only partially wired

**Evidence**

- `src/main.rs:30` — `cx.set_reduce_motion(matches!(settings.reduce_motion, ReduceMotion::On));`
  `System` and `Off` both resolve to `false`.
- `src/ui/pages.rs:1944-1971` — the Settings motion chips save the setting but never
  call `cx.set_reduce_motion`; a change requires a restart.
- `docs/design-system.md:68-69` claims "The setting is applied at startup with
  `cx.set_reduce_motion`" and reduced motion disables transforms. Only `On` does.
- No OS preference read is present. On Windows the relevant preference is
  "Show animations"; on macOS "Reduce motion".

**Official rule**

- Expressive motion is spring-based; reduced motion must finish spatial movement
  immediately while keeping fades.
  <https://m3.material.io/styles/motion/overview/how-it-works>
- Accessibility guidance requires respecting the platform's reduced-motion setting
  and providing a non-motion path.
  <https://m3.material.io/foundations/accessible-design/overview>
- WCAG 2.3.3 / 2.2.2: motion can be disabled by the user.
  <https://www.w3.org/WAI/WCAG21/Understanding/animation-from-interactions.html>

**Why this deviates**

`System` is the default setting in `Settings::default()`, but it behaves exactly like
`Off`. A user who has enabled "Reduce motion" in the OS and leaves the app on
"System" still gets every spring, ripple and theme animation. Changing the chip in
Settings appears to do nothing until relaunch, which is a broken control, not just a
missing feature.

**Concrete fix**

- Resolve `System` from the OS at startup and when the OS setting changes; expose a
  small `platform::reduce_motion()` helper.
- In the motion chip handler, call `cx.set_reduce_motion(value)` and `cx.notify()`
  immediately; do not require a restart.
- Add tests for all three values and a screenshot of `System` + OS reduce motion on.
- Keep the kit's policy: spatial finishes instantly, effects fades remain.

---

### P1-6 — i18n key parity is good, but English leaks and +30% English breaks the layout

**Evidence**

- Key parity test passes: `src/ui/i18n.rs:530-544`.
- Hard-coded user-facing strings:
  - `src/ui/mod.rs:1341` — `"Light theme"` / `"Dark theme"`.
  - `src/ui/mod.rs:1350` — `"English"` / `"中文"` as the accessible label.
  - `src/ui/mod.rs:1375` — `"Main navigation"` landmark name.
  - `src/ui/pages.rs:784` — `"Instance sections"` tab label.
  - `src/ui/pages.rs:865, 1833` — `"Installation progress"` progress label.
  - `src/ui/pages.rs:2014` — `"managed"` status chip.
  - `src/ui/pages.rs:331-337` — `home.view_all` is reused for each news item's
    "open this article" action.
- Raw English from core:
  - `src/core/install.rs:35-44` returns `"metadata"`, `"client"`, `"libraries"`,
    `"natives"`, `"assets"`, `"done"`, `"error"`; `pages.rs:833-838` and `1827`
    interpolate them into the zh UI.
  - `src/core/instances.rs:236` appends `" (copy)"` to duplicated instance names.
  - `src/ui/mod.rs:495-497` shows `YuhinaError::message` directly; core errors are
    English-only.
- Live check: English at 1440×900 clips the right gutter and "Default memory" chips;
  the top-bar actions disappear. At 1000×800 the empty-state sentence is clipped.
  The committed zh screenshots do not show this, so the +30% tolerance claim in
  `docs/product-spec.md:115` is not met.

**Official rule**

- Product requirement: every user-facing string is a key in `src/ui/i18n.rs`
  (`AGENTS.md:30`, `docs/product-spec.md:113-115`).
- Localization is a resource concern; do not concatenate English UI fragments.
  <https://developer.android.com/guide/topics/resources/localization>
- Layout must reflow without loss at supported sizes; WCAG 1.4.10 requires content at
  320 CSS px equivalent without two-dimensional scrolling.
  <https://www.w3.org/WAI/WCAG21/Understanding/reflow.html>

**Why this deviates**

The keys exist, but many strings bypass them or come from the core layer. A zh-CN
user sees `安装中 · metadata · 3%`, English core errors in a toast, and `(copy)` in an
instance name. Simultaneously, the longer English strings expose the missing width
containment, so the app is not merely untranslated — it is unusable at the standard
desktop size in one of its two required languages.

**Concrete fix**

- Add keys for top-bar actions, landmarks, tab labels, progress labels, "managed",
  the duplicate suffix, "Read more", and every core error code.
- Map `InstallPhase` to `instances.phase.*` in the UI; do not display
  `InstallPhase::label()`.
- Map `YuhinaError::code` to `error.*`; fall back to `message` only for unknown
  codes.
- Move the duplicate suffix out of core or pass a localized suffix into
  `instances::duplicate`.
- Fix the layout containment in P0-1; then add an English-at-1000×800 snapshot
  and a predicate test that no text or action rect exceeds the viewport.
- Remove or wire the dead keys (`settings.theme_system`, `home.greeting_guest`,
  `common.copy`, `dialog.delete_body`, `common.error`, …); `tr` currently returns `"…"`
  for a missing key, which hides defects.

---

### P1-7 — Loading, empty, error and offline states are not modelled

**Evidence**

- `src/ui/mod.rs:102-112` defines `Loading { instances, accounts, servers, news, … }`.
- `src/ui/pages.rs:601-621` (instances), `1548-1568` (accounts), `1702-1723` (servers)
  branch only on `rows.is_empty()` and show the empty state while the corresponding
  `loading` flag is true.
- `src/ui/pages.rs:448-451` — Home shows "加载中…" whenever `news.items` is empty; a
  failed `refresh_news` leaves `news.items` empty and `loading.news = false`, so the
  text never resolves to an error.
- `src/ui/pages.rs:1411-1424` — Library shows the empty state after an error because
  the error path stores an empty `SearchResult`; the only signal is a 4 s toast.
- `src/ui/pages.rs:1843-1881` — Activity's empty state is bare text, the same
  `activity.empty` string in both sections, with no icon, action or retry; install
  errors are not shown at all.
- `src/ui/mod.rs:478-493` — every toast is cleared after 4 s regardless of
  `SnackbarDuration::Long`; there is no queue.
- No persistent offline banner or retry action exists in any page.

**Official rule**

- Loading, empty and error are first-class states; progress must be communicated
  without blocking. <https://m3.material.io/components/progress-indicators/overview>
- Errors should be shown where they occur, with a path to recover; a snackbar is for
  transient confirmation, not for a failed page load.
  <https://m3.material.io/components/dialogs/guidelines>
- The project definition of done explicitly requires loading, empty, error and
  offline states (`AGENTS.md:79`).

**Why this deviates**

The state model exists in `Yuhina` but is not consumed by the pages. A first-run or
slow launch shows "No instances yet" as if the list were known to be empty. A network
failure in Home or Library looks like an empty result forever. Activity has no
actionable empty state and hides install failures. The 4 s toast is the only error
channel, so users cannot read or retry a failure at their own pace.

**Concrete fix**

- Add per-resource `error: Option<String>` and render:
  - loading: skeleton/`LinearProgress` and a respectful "Loading…" label;
  - error: inline `panel` with `ERROR_CONTAINER`/`ON_ERROR_CONTAINER`, message and a
    `common.retry` button;
  - empty: `empty_state` with icon, title, body and a primary action.
- Keep the last good data and show a stale banner if refresh fails.
- Add a persistent offline state for Home news and Library search, with retry.
- Activity: distinct keys for running/installs, icons, a retry for failed installs,
  and `LinearProgress` for active installs.
- Replace the 4 s toast timer with the Snackbar's own duration; queue messages and
  pause on hover/focus.

---

### P1-8 — Tonal CTA on the primary-container hero has a 1.003:1 container contrast

**Evidence**

- `src/ui/pages.rs:343-399` — the hero is `PRIMARY_CONTAINER` and contains a
  `ButtonVariant::Tonal` "Play" button.
- Sampled from the screenshots:
  - light: tonal fill `#E8DEF8` on hero `#EADDFF` → **1.003:1**;
  - dark: tonal fill `#4A4458` on hero `#4F378B` → **1.003:1**.
- The text itself passes (on-secondary-container on the tonal fill: 13.24:1 light,
  7.19:1 dark), but the container is invisible, so the secondary CTA reads as a
  disabled/text link next to the filled primary.
- The same pattern appears in the dark hero in `home-native.png`.

**Official rule**

- Use paired roles: `on-primary` on `primary`, `on-primary-container` on
  `primary-container`; mismatched container/on-container pairs are not guaranteed to
  meet contrast. <https://m3.material.io/styles/color/advanced/apply-colors>
- Non-text UI components and state indicators need 3:1 against adjacent colours.
  <https://www.w3.org/WAI/WCAG21/Understanding/non-text-contrast.html>

**Why this deviates**

A tonal button is a secondary-container component. Placing it on a
primary-container surface produces two large light (or dark) fills with nearly
identical luminance. The CTA is technically readable but visually disabled and fails
the non-text contrast rule for the button boundary. This is the hero's main action
pair, so it undermines the most important moment on Home.

**Concrete fix**

- Use a `Filled` button for "Play" on the hero (`primary`/`on-primary`, 6.44:1), or an
  `Outlined`/`Text` button whose content is `on-primary-container`; do not place a
  tonal button on a container role.
- If both hero actions must be low emphasis, move the hero to `SURFACE_CONTAINER` and
  keep the tonal button there.
- Add a contrast assertion for every button-on-container pairing in the page tests.

---

### P1-9 — Field errors are only snackbars; destructive confirm looks like a normal action

**Evidence**

- `src/ui/dialogs.rs:219-224` — the offline-account hint is static; there is no error
  state.
- `src/ui/dialogs.rs:426-429` — Delete Instance confirm is
  `ButtonVariant::Filled` (primary), not an error/destructive style.
- `src/ui/dialogs.rs:448-452` — Remove Content confirm is also primary-filled.
- `src/ui/mod.rs:495-497` — validation and core failures go to `show_error` (toast).
- `src/ui/dialogs.rs:298-306` — Add Account and Microsoft Login have only Cancel in
  the action row; the primary action lives inside the content.

**Official rule**

- Dialog field errors must be inline; general errors go in a basic dialog when the
  confirming action fails. <https://m3.material.io/components/dialogs/guidelines>
- Dialog actions are trailing text buttons, at most two, with a confirm label that
  states the action ("Create", "Delete"), not "OK"/"Confirm".
  <https://m3.material.io/components/dialogs/guidelines>
- A destructive confirmation should be visually distinguished; the error role is the
  available semantic signal for that state.
  <https://m3.material.io/styles/color/roles>

**Why this deviates**

A user typing an invalid offline name or authlib URL gets a transient toast after
submit, with no indication of which field is wrong. A delete dialog uses the same
purple filled button as "Create", so it communicates a normal, safe action. Add
Account's primary "Add" is not in the action row, making the dialog pattern
inconsistent with Create Instance and Add Server.

**Concrete fix**

- Wire `TextField`'s error state and supporting text; validate on submit (and on blur
  for the name/URL fields); keep the dialog open and focus the invalid field.
- Prefer trailing text buttons for both dialog actions, as the M3 examples show. If
  the filled primary is kept as an app style, reserve it for Create/Add and never use
  it for destructive actions. Add a `Destructive` button tone (or an error-coloured
  text button) for Delete Instance and Remove Content, with labels "Delete"/"Remove".
- Move the primary Add/Sign in button into the action row; keep the content for the
  form only.
- Add the missing copy affordance to the Microsoft device-code dialog (`common.copy`
  already exists).

---

### P2 findings

#### P2-1 — Duplicate page title and too many top-bar actions

**Evidence:** `src/ui/mod.rs:1333-1364` renders a small `TopAppBar` with the page
title, while `src/ui/pages.rs:44-78` renders the same title again as a 32 px text
block. The bar also has three actions (theme, language, settings), and its navigation
slot holds a static "Y" logo (`mod.rs:1336`, `1451-1462`) that looks like a button but
has no action.

**Rule:** app bars describe the current page and provide 1–2 essential actions; the
bar title is the screen's top-level heading.
<https://m3.material.io/components/app-bars/overview>

**Why:** the screen displays its title twice while the app bar already exposes the
top-level heading; that wastes ~88 px and splits hierarchy. Three global actions add
noise, and two of them duplicate Settings.
<https://m3.material.io/foundations/layout/canonical-layouts>

**Fix:** either use `TopAppBarVariant::LargeFlexible` with the page subtitle and
remove the content H1, or keep the small bar and drop the duplicate H1. Move
theme/language into Settings and keep one settings/overflow action in the bar. This
also recovers ~88 px of vertical space.

#### P2-2 — Corner radii are not M3 tokens

**Evidence:** `pages.rs:80-85` uses 24 for panels, `:162-172` 24 for cards,
`:521-525` 20 for rows, `:530-538` 16 and 14 for icon containers, `:343` 32 for the
hero, `:1451-1455` 10 for the logo.

**Rule:** M3's finite corner scale is 4/8/12/16/20/28/32/48.
<https://m3.material.io/styles/shape/corner-radius-scale>

**Why:** 24, 22, 14 and 10 are not steps in the system, so the app cannot express
"M3 large card" or "M3 extra-large dialog" consistently and future components cannot
reuse the values. <https://m3.material.io/styles/shape>

**Fix:** use constants from `tokens::shape`: cards 12 (or 16/20 for expressive large
cards), rows 20 (large-increased), dialogs/hero 28 (extra-large) or 32
(extra-large-increased); remove 24/22/14/10. Keep the hierarchy, not the literals.

#### P2-3 — Native title bar ignores the light theme

**Evidence:** `home-light.png` and `settings-native-light.png` show a dark Windows
caption above a near-white app. `src/main.rs:45-52` uses `appears_transparent: false`
with no appearance sync; `docs/architecture.md:105` calls native title bar theming a
known gap.

**Rule:** app bars and window chrome share the surface hierarchy and must follow the
active theme. <https://m3.material.io/components/app-bars/overview>

**Why:** in light mode the caption is the darkest, highest-contrast band on screen, so
the window reads as a dark app with a light body; the theme switch looks broken.
<https://m3.material.io/styles/color/overview>

**Fix:** set the window appearance to match the app theme (GPUI window appearance) or
use the kit's `TitleBar`/`CaptionButtons` with `ColorRole::SURFACE`; capture both
themes. Do not leave a dark caption on a light app.

#### P2-4 — Home's 2+1 card grid is uneven

**Evidence:** `pages.rs:191-300, 439` uses `flex_wrap` with `min_w(260)`; three recent
instances produce two cards on the first row and one stretched card on the second
(`home-native.png`, `home-light.png`).

**Rule:** a feed is a configurable grid with consistent columns and spacing across
breakpoints. <https://m3.material.io/foundations/layout/canonical-layouts>

**Why:** the third card's width no longer matches its siblings, so the eye reads it as
a different kind of object; the grid changes shape based on item count instead of
available width.

**Fix:** at ≥1200 use three equal columns (`flex_basis`/grid), at 840–1199 two
columns, below that one column. Do not let the last card stretch to full width.

#### P2-5 — Library uses a text field + button instead of a search bar

**Evidence:** `pages.rs:1291-1308` places a filled `TextField` next to a separate
Filled "Search" button.

**Rule:** search is a search bar with suggestions/results, not a text field plus a
duplicate submit button.
<https://m3.material.io/components/search/overview>

**Why:** two adjacent controls perform the same submit, the button consumes ~130 px
that the result column needs, and the field lacks the search affordance (leading/trailing
search icon, clear).

**Fix:** use `SearchField` with a trailing search icon and submit-on-enter; keep a
single `FilterChip` row for type/target.

#### P2-6 — Activity's empty state is a dead end

**Evidence:** `pages.rs:1843-1881` renders `muted("暂无活动")` twice, with no icon, no
action and no distinct copy for running vs installs. Install errors are silently
omitted.

**Rule:** empty states should explain the state and offer a next step; progress and
failure need a visible status.
<https://m1.material.io/patterns/empty-states.html> (legacy official pattern; no M3
replacement page exists) and
<https://m3.material.io/components/progress-indicators/overview>

**Why:** two identical dead-end messages give the user no way to start an activity and
no idea whether an install failed; Activity is the product's only consolidated status
page.

**Fix:** use `empty_state` with distinct keys and actions (launch an instance, browse
the library), show failed installs with `ERROR_CONTAINER` and a retry, and show the
instance name for each install.

#### P2-7 — Every news item's action is labeled "View all"

**Evidence:** `pages.rs:331-337` uses `home.view_all` for the per-item link. A screen
reader hears several identical "View all" buttons for different URLs.

**Rule:** controls need names that describe their unique action.
<https://developer.android.com/guide/topics/ui/accessibility/apps>

**Why:** identical accessible names make it impossible to choose the right article
without trial and error, and the visible label mis-describes the action (it opens one
article, not the section).

**Fix:** add `home.read_more` ("阅读全文"/"Read more") for items and keep
`home.view_all` for the section link; include the article title in the accessible
name.

#### P2-8 — "System" theme/language are declared but not implemented

**Evidence:** `pages.rs:1893-1908` offers only Dark/Light; `core/settings.rs:112-119`
maps `ThemePreference::System` to dark unconditionally; `core/settings.rs:127-139`
reads `LANG`/`LC_ALL`, which is normally unset on Windows. The keys
`settings.theme_system` and `home.greeting_guest` are unused.

**Rule:** light/dark must follow the user or OS preference; language should use the
platform locale. <https://m3.material.io/styles/color/overview> and
<https://developer.android.com/guide/topics/resources/localization>

**Why:** the enum and translation keys advertise a System mode that the UI never
offers, and the resolver defaults it to dark; on Windows the language resolver often
falls back to English even on a Chinese system.

**Fix:** add a third theme chip "System" and resolve it from the OS appearance; use
the platform locale API (not `LANG`) for language; or remove the System enum values
and keys. Add tests for both.

#### P2-9 — Snackbar placement, timing and queueing

**Evidence:** `src/ui/mod.rs:1394-1412` positions the snackbar
`absolute().left(24).bottom(24)` at the root, so it overlays the bottom navigation bar
on compact layouts and is not centred. `show_toast` clears it after 4 s regardless of
duration and overwrites any previous message.

**Rule:** snackbars appear above the navigation bar, in a safe area, for a duration
that matches their content. <https://m3.material.io/components/snackbar/overview>

**Why:** the bar hides the destinations, the fixed 4 s timer cuts off long errors, and
two failures in a row leave only the last one visible; there is no way to read or act
on a failure.

**Fix:** place it above the navigation form (bottom offset = bar height + 16), centre
it with a max width of 672, use the component's duration, queue messages, and pause
the timer on hover/focus. Keep errors until dismissed or retry.

#### P2-10 — Instance detail overloads the header with five actions

**Evidence:** `pages.rs:696-779` puts Play, Install/Repair, Open folder, Duplicate and
Delete in one row, with Delete as an icon button.

**Rule:** app bars/toolbars keep 1–2 essential actions and move the rest into
overflow. <https://m3.material.io/components/app-bars/overview>

**Why:** five equal-weight actions flatten the hierarchy, make destructive Delete too
easy to hit, and crowd the title at 1000 px.

**Fix:** keep Play + Install/Repair visible; move folder/duplicate/delete into a
`MaterialMenu` overflow. Put the back arrow in the top app bar and the instance name
in the app bar subtitle.

#### P2-11 — Server removal is immediate and unconfirmed

**Evidence:** `pages.rs:1678-1686` deletes a saved server on a single icon click,
unlike instance and content deletion, which confirm. There is no undo.

**Rule:** destructive actions require confirmation, or an immediate undo path.
<https://m3.material.io/components/dialogs/guidelines>

**Why:** an accidental click permanently removes a saved server and its metadata with
no recovery, while the app confirms much lower-risk actions elsewhere; the interaction
is inconsistent and unsafe.

**Fix:** confirm with a small dialog, or show a snackbar with Undo.

#### P2-12 — Account rows expose a raw UUID and use two accent roles for one state

**Evidence:** `pages.rs:1509` prints the full UUID; `pages.rs:1481-1485` paints the
active row with `SECONDARY_CONTAINER` while `:1512-1517` paints the "当前" chip with
`PRIMARY_CONTAINER`. These two containers are nearly the same luminance.

**Rule:** one state should use one role pair; container roles are not interchangeable.
<https://m3.material.io/styles/color/roles>

**Why:** the raw 36-character UUID dominates the secondary line, and two accent
families for "active" make the row and chip look like unrelated states.

**Fix:** show a shortened UUID (first 8 characters) or hide it behind Details; use
one accent family for the active state (e.g. secondary-container row + on-secondary
text, or primary-container only for the chip). Hide "Set active" when the account is
already active instead of showing a disabled ghost action.

#### P2-13 — Modrinth descriptions are unbounded

**Evidence:** `pages.rs:1348-1359` renders the full description; at 1440 it produces
2–3 long lines (~90 characters), over the 50–75 character guidance, and makes result
rows uneven.

**Rule:** body copy should stay within a readable measure of roughly 50–75 characters
per line. <https://m3.material.io/styles/typography/overview>

**Why:** long, variable-length descriptions create ragged rows and make the install
button jump vertically; readers lose their place across a ~90-character line.

**Fix:** clamp to 2 lines with `text_ellipsis`/`max_lines` and a 65–75ch max width;
put the full text in a details view.

#### P2-14 — Off-scale icon sizes and no contrast-level option

**Evidence:** `pages.rs` uses 18/20/22/24/26/30/32/40 px icons; M3 component icons are
20/24/40. `main.rs:26` hard-codes `Contrast::Standard`, while M3 supports
medium/high contrast for low-vision users.

**Rule:** component icons use the 20/24/40 scale; colour roles support Standard,
Medium and High contrast. <https://m3.material.io/styles/color/overview>

**Why:** off-scale icons change optical weight between rows, and the app never offers
the contrast levels that low-vision users need; both are visible quality issues with
an accessibility cost.

**Fix:** use 20/24/40 for component icons and reserve 30/32 for decorative empty-state
glyphs; add a Standard/Medium/High contrast setting and pass it to
`ThemeId::scheme`.

#### P2-15 — No page transition or shared continuity

**Evidence:** `src/ui/pages.rs:4-14` swaps the entire page element; `src/ui/mod.rs:1330`
rebuilds the content with no transition, so navigation snaps instantly.

**Rule:** M3 Expressive motion provides continuity between surfaces; a page change is
a natural place for a short fade-through or shared-axis transition, and it must respect
reduced motion. <https://m3.material.io/styles/motion/expressive/overview>

**Why:** the instant swap makes navigation feel like a web reload and loses the
spatial relationship between sections, especially from Home to an instance detail.

**Fix:** wrap the page body in a short fade-through (the kit's `token_transition` with
`DefaultEffects`/`FastSpatial`); keep it under 200 ms, disable transforms under reduced
motion, and do not animate on first paint.

---

## Component compliance matrix

| Component | Status | Evidence | Note |
| --- | --- | --- | --- |
| `NavigationSuite` | **Fail** | `mod.rs:1375-1392`, `instances-rail.png` | Drawer on desktop; wide button as rail action; 7 destinations; shell overflow. |
| `TopAppBar` | **Partial** | `mod.rs:1333-1364` | Small bar is valid; 3 actions, duplicate H1, static logo, no scroll state. |
| `MaterialButton` | **Pass (context fail)** | `pages.rs` throughout | Correct variants/sizes/round shape; tonal on primary-container fails contrast. |
| `IconButton` | **Pass** | `pages.rs:745-779`, `1534-1542` | Accessible labels supplied; 48 dp target from kit. |
| FAB / primary action | **Fail** | `mod.rs:1366-1373` | No FAB; a full-width Filled button is forced into every form. |
| `FilterChip` | **Partial** | `pages.rs:1253-1289`, `dialogs.rs:49-100` | Visuals correct; used as single-select segmented controls in Settings/dialogs. |
| `TextField` | **Pass (parent bug)** | `dialogs.rs:108`, `pages.rs:1297` | Filled variant, labels and password/email types are correct; dialog padding clips it (P1-3). |
| `MaterialSwitch` | **Pass** | `pages.rs:2109-2115` | Correct kit switch with label. |
| `MaterialTabs` | **Pass** | `pages.rs:784-805` | Controlled, 4 tabs, kit focus; accessible label hard-coded. |
| `MaterialDialog` | **Partial** | `dialogs.rs:166-180`, `298-306` | 28 dp radius, scrim, focus trap, trailing actions; content padding/scroll/destructive style fail. |
| `Snackbar` | **Partial** | `mod.rs:1394-1412` | Correct component; overlays bottom bar, 4 s timer, no queue. |
| `LinearProgress` | **Pass** | `pages.rs:865-869`, `1830-1838` | Determinate, 4 dp, label; install error not visually distinguished. |
| `Avatar` | **Pass** | `pages.rs:1491-1496` | Monogram + account name; no skin image is a known gap. |
| Cards / panels | **Partial** | `pages.rs:80-85`, `162-172` | Tonal hierarchy good; 24 dp is not a card token; not clickable/interactive. |
| List rows | **Partial** | `pages.rs:521-598` | Clear leading/title/meta/actions; meta wraps at 1000, no loading skeleton. |
| Empty / loading / error / offline | **Fail** | `pages.rs:448-451`, `601-621`, `1411-1424`, `1843-1881` | Loading flags unused; error becomes empty; offline absent; Activity bare text. |
| Dark / light | **Partial** | all screenshots | Both render; wrong scheme; dark native caption in light mode; no light dialogs captured. |
| Adaptive layout | **Fail** | `instances-rail.png`, live checks | 800×600 and compact fail; 1000×800 cramped; English clips at 1440. |
| i18n | **Fail** | `i18n.rs`, hard-coded strings above | Key parity passes; runtime leaks and expansion fail. |
| Accessibility | **Partial** | kit components, `main.rs:30` | Labels/focus largely good; text scale, system motion, inline errors, accessible names incomplete. |

---

## Verification results

### Dark / light

| Check | Result | Evidence |
| --- | --- | --- |
| Dark theme renders every role | **Pass** | `home-native.png`, `settings-native-dark.png`, `accounts-native.png` |
| Light theme renders every role | **Pass with caveat** | `home-light.png`, `instances-light.png`, `settings-native-light.png`; native caption is dark |
| Primary/on-primary contrast | **Pass** | `#FFFFFF` on `#6750A4` = 6.44:1 |
| On-container contrast | **Pass** | on-primary-container 13.32:1 light, 7.23:1 dark |
| Supporting text | **Pass** | on-surface-variant 8.88:1 light, 7.20:1 dark |
| Error text | **Pass** | error on container-low 10.06:1 dark |
| Tonal button on hero | **Fail** | 1.003:1 light and dark (P1-8) |
| Scheme identity | **Fail** | Baseline purple, not Expressive teal (P0-2) |

### Window sizes

| Size | Result | Evidence |
| --- | --- | --- |
| 1440×900 zh | **Pass with caveats** | committed screenshots; right gutter is present but grid/duplicate title issues remain |
| 1440×900 en | **Fail** | live check: right gutter and memory chips clipped; top-bar actions off-screen (P1-6) |
| 1000×800 zh | **Fail** | `instances-rail.png`: drawer instead of rail, meta wraps, action cluster cramped (P1-1) |
| 1000×800 en | **Fail** | live check: empty-state text clipped at the right edge (P0-1/P1-6) |
| 800×600 | **Fail (P0)** | live check: 3 nav destinations unreachable, rail action overflows, page/top-bar actions clipped |
| 500×700 compact | **Fail (P0)** | live check: bottom navigation bar absent; top bar only theme action; content clipped |
| 200% text scale | **Fail (predicted)** | all page text is `px()`; `init_component_theme(..., 1.0, ...)`; no setting (P1-4) |

### Keyboard focus

| Check | Result | Evidence |
| --- | --- | --- |
| Kit components have focus rings | **Pass by construction** | Buttons/chips/fields/nav/tabs all kit components; no custom clickable `div`s |
| Tab order | **Pass by structure** | top bar → navigation → page content; kit roving focus in nav |
| Escape / focus trap in dialogs | **Pass by kit** | `MaterialDialog` modal + focus trap; not visually re-tested |
| Visible focus screenshot | **Not captured** | Add a focus-state capture for the follow-up review |
| Static logo in navigation slot | **Fail (affordance)** | it looks like a button but has no action (P2-1) |

### Reduced motion

| Check | Result | Evidence |
| --- | --- | --- |
| `On` at startup | **Pass** | `main.rs:30` maps `On` to `true` |
| `System` | **Fail** | maps to `false`; OS preference never read (P1-5) |
| Runtime change | **Fail** | Settings chips only save; no `cx.set_reduce_motion` (P1-5) |
| Spatial motion under reduced motion | **Pass by kit** | kit finishes spatial immediately, keeps fades |
| Page transitions | **Not implemented** | pages snap; no fade-through/shared-axis (P2-15) |

### i18n

| Check | Result | Evidence |
| --- | --- | --- |
| zh-CN completeness | **Pass** | 100% of used keys present |
| en-US completeness | **Pass** | `i18n.rs:530-544` test passes |
| +30% English tolerance | **Fail** | 1440 en clips right gutter/chips; 1000 en clips empty-state text |
| Hard-coded user strings | **Fail** | top bar, landmarks, phase names, "managed", "(copy)", core errors (P1-6) |
| CJK/Latin fallback | **Pass visually** | screenshots render both; Roboto is not bundled |
| Accessibility names localized | **Fail** | "Main navigation", "Instance sections", "Installation progress" |
| Error localization | **Fail** | `show_error` prints `YuhinaError::message` (English) |
| Missing-key fallback | **Weak** | `tr` returns `"…"` and hides the missing key; add a debug assertion/test |

---

## Prioritized patch plan

### 1. Foundation (must land first)

**`src/main.rs`**

- Introduce `APP_THEME: ThemeId = ThemeId::Expressive` and `APP_CONTRAST` (or a
  settings value); replace `ThemeId::Baseline` at `:26`.
- Replace the `matches!(..., On)` reduced-motion line with a resolver that handles
  `System` from the OS; call `cx.set_reduce_motion` at startup.
- Pass the persisted text scale to `init_component_theme` (P1-4).

**`src/ui/mod.rs`**

- Replace `ThemeId::Baseline` at `:572` with `APP_THEME` so the transition and the
  initial scheme agree.
- Add a settings handler that applies reduced motion and text scale immediately.
- Add a single `APP_THEME` import instead of duplicating scheme construction.

### 2. Shell and navigation

**`src/ui/mod.rs:1320-1441`**

- `expanded_type(NavigationSuiteType::RailExpanded)`; `expanded_from(Expanded)`.
- Primary action: `Fab` (or `ExtendedFab` only in expanded rail); remove the
  full-width `MaterialButton`.
- Reduce the bar destination count to 5; move Activity/Settings to overflow or the
  top bar.
- Top bar: 1–2 actions; move theme/language into Settings; localize action labels;
  remove the duplicate title or switch to `LargeFlexible`.
- Pass the page scroll offset to `TopAppBar` so the scrolled-under surface appears.
- Fix snackbar placement above the navigation bar and queue messages.

**`src/ui/pages.rs:20-78`**

- `page_frame`: `overflow_x_hidden`, wrapper `px(32)`, inner `max_w(1120)`, `mx_auto`,
  `min_w_0`.
- `page_header`: `min_w_0` on title and action slots; allow actions to wrap below the
  title; never let the header widen the root.
- Add a 5-destination/overflow model and a 600 px rail fit test.

### 3. Page-level design fixes

**`src/ui/pages.rs`**

- Replace `px` text sizes with `TypeRole` scale + line height throughout.
- Replace shape literals with token constants (P2-2).
- Hero: `Filled` Play or move the hero to surface-container (P1-8).
- Home: equal 3/2/1 column grid; fallback greeting + CTA when no account.
- Instances: loading skeleton, inline error/retry, meta that wraps deliberately;
  move detail overflow actions into a menu.
- Library: `SearchField`, 2-line description clamp, inline error/retry/offline.
- Accounts: shorten UUID, hide "Set active" when active, destructive confirm.
- Servers: confirm/undo removal, offline status with icon/error role.
- Activity: distinct empty states, install errors and retry.
- Settings: add System theme/language and Standard/Medium/High contrast; apply
  reduced motion immediately; replace single-select FilterChips with
  `ConnectedButtonGroup::segmented` (P2-8, P2-14).

### 4. Dialogs

**`src/ui/dialogs.rs`**

- Add `px(24)` (and `pb(8)`) around every `content`; test the helper text.
- Add `body_scroll`/`dividers`/max height; full-screen dialog below 600 dp.
- Replace version/loader/memory chip walls with dropdown/slider/stepper.
- Inline field errors; keep the dialog open; focus the invalid field.
- Destructive confirm: error-coloured text button, label "Delete"/"Remove".
- Move Add/Sign in into the action row; add copy for the device code.
- Add a supporting line under Create Instance explaining what will be downloaded.

### 5. Localisation and core boundary

**`src/ui/i18n.rs`**

- Add keys for every hard-coded string in P1-6 and P2-7, including accessible names.
- Add `instances.phase.*` and `error.*` maps; remove dead keys or wire them.
- Add a debug test that `tr` never falls back to `"…"` for a key used in code.

**`src/core/install.rs`, `src/core/instances.rs`, `src/core/settings.rs`**

- Keep core language-neutral: return enums, not display strings.
- Remove the `" (copy)"` suffix from core; let the UI localise it.
- Implement `System` theme/language or remove the variants.
- Add `text_scale` and `contrast` settings with migration defaults.

### 6. Verification gate

- `cargo fmt --all --check`, `cargo clippy --all-targets`, `cargo test`.
- Screenshot matrix: 1440×900, 1000×800, 800×600, 500×700; zh-CN + en-US;
  dark + light; 100%, 150%, 200% text; reduce motion on/off; focus state; dialog
  open.
- Acceptance: no text or action rect exceeds the viewport; every destination is
  reachable in every form; top-bar actions visible at every size; no clipped helper
  text; no English phase/error strings in zh; theme is Expressive in both startup and
  toggle.

---

## What is already correct (do not regress)

- **Semantic colour discipline.** `src/ui` contains 75 `ColorRole::` uses and zero
  hex/rgb literals. Keep painting through roles; fix the scheme and pairings, not the
  pattern.
- **Kit component usage.** Buttons, icon buttons, text fields, chips, switch, tabs,
  dialogs, snackbar, progress, avatar and navigation all come from
  `gpui-kit-material`; there are no bespoke controls to maintain.
- **Tonal-surface hierarchy.** Surface → surface-container-low → surface-container
  reads correctly in dark and light; panels and cards are grouped, not flat.
- **Shape hierarchy.** 32 hero → 24 panel/card → 20 row → 14/16 icon container → full
  pill. The radii communicate hierarchy even though the literal values need
  tokenising.
- **Expressive button geometry.** Round buttons, 5 kit sizes, clear Filled/Tonal/Text
  hierarchy per row; primary actions are right-aligned and consistently sized.
- **Status chips.** Installed/running/not-installed use different roles and always
  carry text; colour is not the only indicator.
- **Icon-only labels.** Every `IconButton` receives a label argument; kit focus and
  tooltip behavior is preserved.
- **Text fields.** The filled variant correctly paints
  `surface-container-highest`; labels, email type and password masking are used.
- **Dialog basics.** 28 dp radius, 32 % scrim, modal focus trap, trailing actions,
  headline-small scale, and Cancel-before-confirm order are correct. Keep these while
  fixing padding and destructive styling.
- **Navigation semantics.** `NavigationSuite` is controlled (`selected` +
  `on_change`), so selection survives form changes; the kit handles focus across
  breakpoints.
- **Motion plumbing.** The kit defaults to Expressive springs, ripples and state
  layers; `advance_component_theme` is called from a next-frame loop, which is the
  correct API usage. Keep the plumbing, fix the scheme and the reduced-motion
  resolution.
- **i18n key parity and zh default.** The key parity test is valuable; zh-CN is the
  default and both tables are complete for existing keys.
- **CJK/Latin layout.** The screenshots show clean CJK rendering, mixed-script
  subtitles, and no emoji-as-icon or neon/gradient violations.
- **Avatar and progress.** Monogram avatars and determinate `LinearProgress` are
  correct M3 patterns; keep them when adding loading/error states.
- **Security boundary.** The UI only receives account summaries; do not move token
  data into the page layer while fixing account rows.

---

## Appendix — reproduction notes

The live checks used a temporary profile and did not modify the repository or the
real user data:

```powershell
$env:YUHINA_DATA_DIR = Join-Path $env:TEMP 'yuhina-audit-profile'
$env:YUHINA_SIZE = '800x600'   # then 500x700, 1000x800, 1440x900
$env:YUHINA_PAGE = 'instances' # or home, settings
cargo run
```

The temporary profile contained only `settings.json`, `accounts.json`,
`servers.json`, `instances.json` and the instance metadata directories. English was
selected by writing `"language": "en-US"` and `"languageMode": "en-US"` into that
profile's `settings.json`; the real profile was not edited.

**Known evidence limits**

- The committed screenshots are 1536×864, not exactly 1440×900; they still show the
  relative proportions and the layout failures. The 800×600 and 500×700 live checks
  are described rather than committed because only this report may be written.
- Focus rings, hover state layers and ripples were verified by component source, not
  by a captured focus-state image. Add focus screenshots to the next review.
- No light-theme dialog capture exists in `design/reviews/rewrite-m3e/`; add one.
- The official `navigation-suite` page is 404, so its adaptive rule is cited from the
  official rail/breakpoints/drawer pages and the kit's m3e port. This is a
  documentation gap on Google's site, not a reason to keep the drawer.
