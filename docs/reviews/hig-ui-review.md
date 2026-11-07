# Yuhina — Apple HIG review of the Material 3 Expressive UI

| | |
| --- | --- |
| Date | 2026-11-07 |
| Repository | `C:\Users\youzilm\Documents\projects\yuhina` |
| Branch / HEAD | `feat/rewrite-m3e` / **`e786039650dad51b3637ce87fdc97c70d5243ab8`** |
| Working tree | HEAD plus uncommitted brand/icon/dev-capture work (`src/brand_assets.rs`, `build.rs`, `resources/icons/yuhina.ico`, `src/main.rs`, `src/ui/mod.rs`, `Cargo.toml`). That work is treated as in progress and is not graded as shipped. |
| Kit | `gpui-kit-material` `26c0cb1713311280919113e14b552034e492d573` |
| Constraint | Keep Material 3 Expressive and `gpui-kit-material`; use the Apple HIG as the quality bar, not as a request to replace the visual language or add macOS-only chrome. No `src/`, `Cargo.toml`, or other docs were modified. No commit or push. |
| Evidence | Code at the HEAD above, `design/reviews/rewrite-m3e/*.png`, `design/reviews/md3e-audit-fix/*.png`, `design/reviews/merged-m3e/*.png`, and the official HIG pages cited inline. No new screenshots are part of this report. |

## How to read this review

Apple's Liquid Glass, bezelled macOS toolbars, traffic lights, and Mac-only window chrome are explicitly out of scope. The HIG is used for the parts that are platform-neutral and measurable: clarity of hierarchy, deference to content, consistency, direct manipulation, feedback, the spatial model, text scaling, contrast, hit targets, modality, and window behaviour. Every HIG rule below is translated into a concrete Material 3 Expressive component from the kit the product already ships.

**Scale:** 0 = unusable; 5 = functional but visibly below the bar; 8 = release quality; 10 = exemplary. Scores are for the *interaction and craft quality*, not for whether the Material theme is attractive.

## Executive summary

This is a real Material 3 Expressive launcher, not a themed mock-up. The tonal-surface hierarchy is coherent, the Expressive teal scheme is applied through semantic roles, the adaptive rail → bar model now keeps five destinations reachable, and the kit's buttons, chips, fields, tabs, switch, avatar, progress and snackbar are used consistently where they appear. The previous audit's release blockers (Baseline scheme, seven destinations, drawer on desktop, empty create dialog) are genuinely fixed.

The HIG bar exposes three systemic problems that the Material-only audits did not:

1. **The app still has no reachable text-size setting.** `init_component_theme(..., 1.0, ...)` is hard-coded, there is no 100/150/200% control, and page geometry is in `px`. HIG Accessibility asks for at least 200% enlargement; the product spec itself requires the layout to tolerate 200%. At 200% the text grows (page text is rem-based) while page padding and row geometry stay in `px`.
2. **The primary-action hierarchy is flat.** Home has a filled Create + filled Play in the hero, a filled Play on every recent card, and a global FAB. Instances has a filled header Create, a tonal Import, a rail FAB, and a filled Play on every row. HIG Buttons asks for one or two prominent buttons per view. The compact FAB also physically covers a row's Play button at 500 px.
3. **A bespoke control layer remains on top of a complete component kit.** Instances, accounts, servers, library results, backups and Java runtimes are hand-built `div` rows; Library is a text field plus a Search button; Settings and dialogs use `FilterChip` as radio buttons; the only `ListItem` in the product is in the Files tab. This is why row height, focus, press feedback and truncation vary from page to page.

**Release call: block on HIG-01 and HIG-02. Fix HIG-03 through HIG-18 before calling the phase complete.** The work is concentrated in `pages.rs` and `dialogs.rs`; no new dependency is needed.

## Scores

| Area | Score | Justification |
| --- | ---: | --- |
| 1. Information architecture & navigation | **5 / 10** | Five primary destinations, a correct rail → bar adaptation, and labelled compact navigation. But every non-Home page repeats its title in the bar and as an H1; four global icons compete with the page title; Settings and Activity collapse into an icon-only More menu at compact; leaving a section discards its instance detail and tab. |
| 2. Visual hierarchy | **4 / 10** | One primary action is not respected (four to six filled buttons on Home/Instances); "Installed" is chipped on every row; the news section renders one card stretched to the full 1120 column; the 2+1 recent grid makes the third card a different object; list rows are heavy cards instead of scannable list items. |
| 3. Typography | **6 / 10** | Good adoption of `TypeRole`; line heights are present. The root family is a hard-coded, unbundled `"Roboto"`; primary metadata is 12 px `BodySmall`; there is no text-scale setting; descriptions and MOTDs are unbounded; logs are not monospaced. |
| 4. Color and materials | **7 / 10** | Excellent role discipline in both themes; the Expressive teal scheme is correct; text contrast pairs are strong; no hex literals. Status uses three container roles for one concept, the active account uses two, and the light-theme native caption is dark. |
| 5. Controls and component choice | **3 / 10** | Only 1 of ~10 row/card surfaces uses a kit list/card. Rows, cards, status chips, search, single-select settings, empty states and inline errors are bespoke. Long choice lists are chips. |
| 6. Interaction and feedback | **4 / 10** | Kit hover/press/focus/ripple is good. Rows are not clickable; the compact FAB overlaps a primary action; snackbars overlay the bottom bar, auto-dismiss after a fixed 4 s and hold only one message; field errors go to a toast; destructive account/server removal has no confirm or undo; several commands block the UI thread with no busy state. |
| 7. Window chrome | **4 / 10** | The native caption stays dark in the light theme; the window is titled with the app name only; no explicit minimum size and a fixed 1440×900 initial position can exceed a laptop work area. The in-progress app-icon work is a real improvement and should be kept. |
| 8. Motion & accessibility | **4 / 10** | Kit focus rings and accessible names are generally present. "System" reduce-motion behaves like "Off"; there are no page transitions or scroll restoration; 200% text is unreachable; snackbars are time-boxed; the only switch is visually unlabeled. |
| **Overall** | **4.6 / 10** | A strong Material 3 Expressive foundation with a thin, inconsistent interaction layer and two foundation-level accessibility failures. |

---

## Findings

### P0 — release blockers

#### HIG-01 — Text cannot be enlarged to 200%, and the layout is fixed in pixels

**Evidence**

- `src/main.rs:50` — `theme::init_component_theme(scheme, 1.0, cx);` The kit's rem scale is pinned at 1.0.
- `src/main.rs:51` — only `ReduceMotion::On` reaches `cx.set_reduce_motion`; there is no analogous text-scale path.
- `src/ui/pages.rs:33-64` — the page frame uses fixed `px(32.)` gutters, `max_w(px(1120.))`, and a fixed `pb(px(160.))` at compact. `pages.rs` contains 158 `px()` occurrences for geometry; only the text roles and the icons (`mod.rs:2269-2271`) are rem-based.
- `src/ui/pages.rs:2423-2742` — the Settings page has theme, language, motion, download source, memory and Java sections, but no text-size control. The product spec (`docs/product-spec.md:119-122`) and `docs/design-system.md:42-44` both require 200% tolerance.
- The committed captures are all at 100%; there is no 150% or 200% capture in any review folder.

**HIG rule**

> "Support larger text sizes. … Ideally, give people the option to enlarge text by at least 200 percent (or 140 percent in watchOS apps)."
> — <https://developer.apple.com/design/human-interface-guidelines/accessibility>

> "Be prepared for text-size changes. … Support Dynamic Type by adjusting your layout to accommodate text at larger sizes. For example, horizontally adjacent views may need to stack vertically to provide more space for text; table rows or other containers may need to grow in height so that text isn't cropped or doesn't overlap other content."
> — <https://developer.apple.com/design/human-interface-guidelines/layout>

**Why this deviates**

The app cannot reach the required enlargement at all: the scale is a literal `1.0` and there is no user-facing control. Page text is rem-based through `TypeRole`, so raising the scale would grow the text, but the gutters, card/row heights, dialog widths and the fixed `pb(160.)` are `px`, so the text would grow inside geometry that cannot follow it. That is the exact clipping/overlap the HIG warns about. This is a foundation failure, not a missing preference.

**Concrete fix**

1. Add `text_scale: u16` (100/125/150/200) to `core::settings::Settings`, persisted like the theme.
2. Pass `scale` to `theme::init_component_theme(scheme, scale, cx)` at startup and to `theme::advance_component_theme(scheme, scale, window, cx)` in `animate_theme`; apply the change without a restart.
3. Convert page geometry that must grow with text to `rem(...)` (gutters, card padding, row gaps, dialog body padding). Keep structural widths (rail 96/220, window minimum) in `px`.
4. In the Settings page, add a `ConnectedButtonGroup` (segmented) with 100/125/150/200% and a "Reset" text button.
5. Capture 100/150/200% at 800×600 and 500×700 in zh-CN and en-US; add a predicate test that no text or action rect exceeds the viewport.

**Acceptance:** 200% is selectable, persists across restart, applies immediately, and every page remains operable at 800×600 and 500×700 with no clipped controls or overlapping text.

---

#### HIG-02 — The compact FAB covers a row's primary action

**Evidence**

- `design/reviews/md3e-audit-fix/home-500-dark-en.png` — the global FAB sits directly over the Play button of the first recent card; `design/reviews/merged-m3e/merged-home-500-dark-en.png` shows the same overlap.
- `src/ui/mod.rs:2186-2193` — the compact FAB is `absolute().bottom(px(96.)).right(px(16.))` over the scrolling content column.
- `src/ui/pages.rs:54` — the scroll frame reserves `pb(px(160.))` at compact, which clears the bottom bar but does **not** reserve the FAB's trailing footprint over the last visible row.
- `src/ui/pages.rs:328-363` — the card's Details/Play cluster has no trailing inset, so it scrolls under the FAB.

**HIG rule**

> "A safe area defines the area within a window that isn't covered on the edge by a hardware feature or another view within the window, like a toolbar, tab bar, or status bar. Respecting the safe area is essential to make sure system UI and hardware features … don't obstruct content and controls."
> — <https://developer.apple.com/design/human-interface-guidelines/layout>

> "Offer sufficiently sized controls. … Consider spacing between controls as important as size."
> — <https://developer.apple.com/design/human-interface-guidelines/accessibility>

**Why this deviates**

The FAB is not merely floating over empty background: it covers a tappable Play button, so the user can hit the wrong action or fail to hit the intended one. This is a blocked primary control, not cosmetic overlap. It also adds a third way to create an instance on Home (FAB, hero Create, and, on Instances, the header Create).

**Concrete fix**

- Prefer the kit's `BottomAppBar` with a docked `Fab` at compact (`BottomAppBar::fab`) so the FAB occupies its own lane above the navigation bar.
- If the FAB stays floating, scope it to pages without an inline primary (Library, Accounts, Servers, Activity, Settings) and hide it on Home and Instances, where the hero/header already owns Create. Reserve a trailing `padding_right` of at least 80 dp on the last row and end the scroll content with the FAB's diameter plus 16 dp.
- Use `Fab` with a 56 dp target (already) and keep the accessible label.

**Acceptance:** at 500×700 the last row's Details and Play are fully visible and tappable; no control is covered by the FAB at any scroll position; there is exactly one "New instance" entry point on Home and one on Instances.

---

### P1 — fix before the phase is called complete

#### HIG-03 — One view has four to six equally prominent actions

**Evidence**

- `src/ui/pages.rs:404-442` — Home hero: two `ButtonVariant::Filled` buttons ("New instance", "Play").
- `src/ui/pages.rs:328-363` — every recent card: another `Filled` Play.
- `src/ui/mod.rs:2111-2118` — the global `Fab` is also "New instance".
- `src/ui/pages.rs:547-569` — Instances header: tonal "Import instance" + filled "New instance".
- `src/ui/pages.rs:662-671` — every instance row: filled Play.
- `src/ui/pages.rs:789-878` — detail: filled Play, tonal Install/Repair, plus folder, duplicate, export and delete icon buttons.
- `design/reviews/md3e-audit-fix/home-1440-dark-zh.png`, `home-1440-light-zh.png`, `instances-1000-dark-zh.png` — the filled mint buttons repeat 4–7 times per screen.

**HIG rule**

> "In general, use a button that has a prominent visual style for the most likely action in a view. … Keep the number of prominent buttons to one or two per view. Presenting too many prominent buttons increases cognitive load, requiring people to spend more time considering options before making a choice."
> — <https://developer.apple.com/design/human-interface-guidelines/buttons>

> "Use style — not size — to visually distinguish the preferred choice among multiple options."
> — same page

**Why this deviates**

Every filled button has the same visual weight, so the eye has no entry point. On Home, "Play the most recent instance" and "Create an instance" compete; on Instances, the page-level Create, the rail FAB and every row's Play compete. The destructive Delete icon is also the same weight as Duplicate/Export in the detail header.

**Concrete fix**

- Home: hero Create = `Filled`; hero Play = `Tonal`; card Play = `Tonal` (or a trailing `IconButton` with a play symbol); remove the FAB from Home.
- Instances: keep the header Create = `Filled`; Import = `Text`; row Play = `Tonal` or `IconButton`; remove the FAB from Instances.
- Detail: Play = `Filled`, Install/Repair = `Tonal`, and move Folder/Duplicate/Export/Delete into a `MaterialMenu` overflow (see HIG-18).
- Add a render test or a small screenshot assertion: at most one `Filled` and one `Tonal` button per viewport.

**Acceptance:** each view has one obvious primary action; row-level Play is secondary to the page action; no more than two prominent buttons are visible at once.

---

#### HIG-04 — The page title is rendered twice and the toolbar carries four global icons

**Evidence**

- `src/ui/mod.rs:2105-2108` — `TopAppBar` title = `self.t(page.title_key())`.
- `src/ui/pages.rs:66-106` — `page_header` renders the same title as `HeadlineLarge` (32/40).
- `design/reviews/md3e-audit-fix/instances-1000-dark-zh.png` and `instances-800-dark-en.png` — "实例 / Instances" appears in the bar and again as the H1.
- `src/ui/mod.rs:2035-2103` — theme, language, Activity and Settings are four separate 48 dp icon buttons on wide layouts; at <600 dp theme/language stay and Activity/Settings move into an icon-only More menu.
- Home is the exception: it uses the hero instead of `page_header`, so the duplication is on Instances, Library, Accounts, Servers, Activity and Settings.

**HIG rule**

> "Provide a useful title for each window. A title helps people confirm their location as they navigate your app…"
> — <https://developer.apple.com/design/human-interface-guidelines/toolbars>

> "Choose items deliberately to avoid overcrowding. People need to be able to distinguish and activate each item, so you don't want to put too many items in the toolbar."
> — same page

> iOS: "Use a large title to help people stay oriented as they navigate and scroll. By default, a large title transitions to a standard title as people begin scrolling…"
> — same page

**Why this deviates**

The title is the screen's top-level heading, and it is printed twice with ~88 px of vertical cost. The bar also puts preferences (theme, language) next to page destinations (Activity, Settings), so the trailing group has no single meaning; two of the four icons duplicate what the Settings page already controls. The duplicate H1 is most visible at 1000 and 800, where the bar title and the H1 sit within 100 px of each other.

**Concrete fix**

- Pick one model and apply it everywhere:
  - **Recommended:** `TopAppBarVariant::LargeFlexible` (or `MediumFlexible`) with `title` + `subtitle`; delete the H1 block in `page_header`, keep only the action cluster and the body. The bar title collapses to a standard title on scroll.
  - Alternative: keep the content H1 and remove the bar title (`TopAppBar` with no title), using the bar only for actions.
- Move theme and language to Settings (they are already there in chip form). Keep Activity and Settings in the bar as one `MaterialMenu` overflow labelled by the localized "More" string; add a `Badge` to the Activity icon when a game is running or an install is active.
- Put the actions in `TopAppBar::actions(...)`, not an absolutely positioned overlay (see HIG-12).

**Acceptance:** the page name appears once; the bar has at most two trailing controls plus More; theme/language change only from Settings; scroll collapses the large title without a layout jump.

---

#### HIG-05 — Dialogs use chips as radio buttons, are 312 dp wide on desktop, and clip the body

**Evidence**

- `src/ui/dialogs.rs:107-120` — up to 12 version `FilterChip`s.
- `src/ui/dialogs.rs:128-140` — 5 loader `FilterChip`s.
- `src/ui/dialogs.rs:174-207` — Forge/NeoForge build chips (unbounded in principle).
- `src/ui/dialogs.rs:224-237` — 4 memory `FilterChip`s.
- `src/ui/dialogs.rs:301-318` — `DialogSize::Small` + `.max_width(560.0)`. `Small` is 312 dp by default; `max_width` is an upper bound, not a width.
- `design/reviews/md3e-audit-fix/dialog-create-800-dark-en.png` — the dialog is ~308 px wide; the last visible line of the memory hint is cut in half at the footer divider, and the body has no visible scroll affordance.
- `design/reviews/md3e-audit-fix/dialog-create-forge-1440-dark-en.png` — the same narrow column at 1440, with 21 chips stacked over a long scroll.

**HIG rule**

> "Limit the number of segments in a control. Too many segments can be hard to parse and time-consuming to navigate. Aim for no more than about five to seven segments in a wide interface…"
> — <https://developer.apple.com/design/human-interface-guidelines/segmented-controls>

> "Aim to keep modal tasks simple, short, and streamlined. If a modal task is too complicated, people can lose track of the task they suspended when they entered the modal view…"
> — <https://developer.apple.com/design/human-interface-guidelines/modality>

> "Make sure your app's layout adapts to all font sizes. Verify that your design scales, and that text and glyphs are legible at all font sizes."
> — <https://developer.apple.com/design/human-interface-guidelines/typography>

**Why this deviates**

`FilterChip` is a filter affordance, not a single-choice form control. A 12-item version list becomes 12 tab stops with a checkmark that looks like multi-select. The dialog then combines three such groups into one scroll inside a 312 dp column, which is both hard to scan and taller than an 800×600 window; the bottom of the body is clipped by the footer. On a desktop the 312 dp width wastes the available 800/1440 space and forces the chips to wrap three per row.

**Concrete fix**

- `MaterialDialog::size(DialogSize::Medium)` (560 dp) or `.width(560.0)` on wide windows; at <600 dp use `DialogSize::FullScreen` with a `close_button`.
- Minecraft version: `ExposedDropdown` with `options(version_options(app))`, `.selected(...)`, `.on_select(...)`, `.menu_max_height(320.)`. Keep the curated order; add a search field only if the list becomes the full manifest.
- Loader: `ConnectedButtonGroup` with `GroupSelection::Single`, `.segmented()`, 5 options. This is the correct "one of five" control.
- Forge/NeoForge build: `ExposedDropdown` (the list can be long); put the recommended build first.
- Memory: `MaterialSlider` driven by a `SliderState` with a 1024–16384 range and 512 MB step, plus a live "4096 MB" label; or `ExposedDropdown` for presets. Do not use four chips.
- Call `.track_body_scroll(&handle)` and add `pb(px(24.))` to `dialog_content` so a line is never cut at the footer; keep the action row visible.

**Acceptance:** create dialog ≤ 560 dp wide at 800+ and full-screen at <600; at most 8 focus stops before the footer actions; no half-clipped line at 800×600 or 200% text; version and loader values remain visible after selection.

---

#### HIG-06 — Library search is a text field plus a duplicate submit button

**Evidence**

- `src/ui/pages.rs:1751-1768` — `TextField::new("library-search", …)` next to `MaterialButton("library-search-button")` with a search icon and the same "Search" label.
- `src/ui/pages.rs:1770-1826` — results only update after `search_library`; there is no clear affordance, no search-leading icon in the field, and no result count.
- `design/reviews/rewrite-m3e/library-native.png` — the paired field/button is visible (pre-fix scheme, same layout).

**HIG rule**

> "A search field is an editable text field that displays a Search icon, a Clear button, and placeholder text where people can enter what they are searching for."
> — <https://developer.apple.com/design/human-interface-guidelines/search-fields>

> "If possible, start search immediately when a person types. Searching while someone types makes the search experience feel more responsive because it provides results that are continuously refined as the text becomes more specific."
> — same page

> "Put a search field at the trailing side of the toolbar for many common uses."
> — same page

**Why this deviates**

Two adjacent controls perform the same submit, the button consumes roughly 130 px that the result column needs, and the field lacks the search affordance (leading search glyph, trailing clear, immediate refinement). The user must click a button after typing, which is the slowest of the three search patterns in the HIG.

**Concrete fix**

- Replace the row with `SearchField::new(&app.search_input)` with a leading `search` icon and a trailing `IconButton` clear; `.field_style(SearchFieldStyle::Standard)`.
- Subscribe to the input's change event and debounce 250 ms; keep Enter as an immediate submit. Wire the current `search_library` to the same path.
- Keep the type chips as a scope bar (they are filters, so `FilterChip` is correct here) and add a `LabelLarge` result count ("24 results").
- When Library is the active page, consider placing the field at the trailing edge of the top bar; the inline placement is acceptable because it filters this page only.

**Acceptance:** typing filters without a button; clearing restores the previous results; Enter and the debounce both work; the field is reachable and labelled by keyboard/screen reader.

---

#### HIG-07 — Rows and cards are bespoke, and rows are not clickable

**Evidence**

- `src/ui/pages.rs:600-674` — instance row: hand-built `div`, fixed 48 dp icon tile, chip, Details button, Play button; no row click, no focus ring, no disclosure.
- `src/ui/pages.rs:270-366` — recent card: hand-built `card_shell`; no card interaction; only the inner buttons act.
- `src/ui/pages.rs:1950-2005` — account row: hand-built, delete `IconButton`, active chip.
- `src/ui/pages.rs:2090-2172` — server row: hand-built, ping/delete icons + Join.
- `src/ui/pages.rs:1777-1826` — library result row: hand-built.
- `src/ui/pages.rs:1283-1356` — backup row: hand-built card nested inside a panel.
- `src/ui/pages.rs:2527-2553` — Java runtime row: hand-built.
- `src/ui/pages.rs:1457-1473` — the **only** `ListItem` in the product is the Files tab.

**HIG rule**

> "Provide appropriate feedback when people select a list item. … In general, a table that helps people navigate through a hierarchy persistently highlights the selected row to clarify the path people are taking."
> — <https://developer.apple.com/design/human-interface-guidelines/lists-and-tables>

> "Keep item text succinct so row content is comfortable to read."
> — same page

> "Always include a press state for a custom button. Without a press state, a button can feel unresponsive…"
> — <https://developer.apple.com/design/human-interface-guidelines/buttons>

**Why this deviates**

Each page reimplements row height, padding, icon tile radius, hover/press state and text truncation. None of the rows is itself interactive, so the only way to open an instance is the small "Details" text button; the row has no focus ring and no keyboard path. The result is inconsistent density (48/44/40/56 px tiles, 14/16/20/24/18 radii) and no selection semantics. The kit's `ListItem` already provides 56/72/88 dp heights, one/two/three-line layout, selected/disabled states, ripple, focus and leading/trailing slots.

**Concrete fix**

| Current | Replace with |
| --- | --- |
| Instance row (`pages.rs:600-674`) | `ListItem` (two-line) with `.leading(ListLeading::Tinted(...))`, `.on_click(open_instance)`, plus a sibling trailing `MaterialButton` for Play |
| Recent card (`pages.rs:270-366`) | `MaterialCard::new(id, CardVariant::Filled)` with `.interactive()` / `.on_click(open_instance)`; trailing Play `Tonal` |
| Account row (`pages.rs:1950-2005`) | `ListItem` `.selected(active)` + `Badge` "Active"; row click sets active; remove in an overflow menu |
| Server row (`pages.rs:2090-2172`) | `ListItem` (two-line: name/address, MOTD) + `Badge` player count; trailing Join; ping/remove in a `MaterialMenu` |
| Library result (`pages.rs:1777-1826`) | `ListItem` (three-line: title, clamped description, author · downloads · categories) + trailing Install `Tonal` |
| Backup row (`pages.rs:1283-1356`) | `ListItem` inside the existing panel, with `.divider(true)`; remove the nested card fill |
| Java runtime (`pages.rs:2527-2553`) | `ListItem` `.trailing_text(path)` + `Badge` "Managed" |

Use `ListDensity::Compact` only where the information is genuinely secondary; keep the standard 56/72 dp row for touch-sized targets. Add `.divider(true)` rather than a separate card gap where rows form a list.

**Acceptance:** every list row is one keyboard-activatable navigation target with a visible focus ring and a predictable focus order to its trailing action; text truncates to the declared number of lines; rows with the same amount of text have the same height.

---

#### HIG-08 — Back is a text button, the detail loses its place, and section state is discarded

**Evidence**

- `src/ui/pages.rs:738-748` — the instance detail "Back" is a `MaterialButton` with a back icon and the visible word "Back", placed inside the page content.
- `src/ui/mod.rs:739-741` — `set_page` sets `self.selected_instance = None` on every destination change, so leaving Instances and returning drops the open detail.
- `src/ui/mod.rs:788-797` — `open_instance` resets the tab to Overview and clears backups/files; the scroll handle is per-page, not persisted.
- `src/ui/mod.rs:2105-2108` — the top bar keeps the section title ("Instances"), not the instance name, while the detail is open. There is no history stack and no Esc/Alt+Left handling.
- `design/reviews/md3e-audit-fix/instances-1000-dark-zh.png` — the bar and H1 both say "实例"; the detail would still say "实例" in the bar.

**HIG rule**

> "Use the standard Back and Close buttons. People know that the standard Back button lets them retrace their steps through a hierarchy of information… Prefer the standard symbols for each, and don't use a text label that says Back or Close."
> — <https://developer.apple.com/design/human-interface-guidelines/toolbars>

> "They also let people quickly switch between sections of the view while preserving the current navigation state within each section."
> — <https://developer.apple.com/design/human-interface-guidelines/tab-bars>

> "Preserve a person's context. … Keep content and controls in consistent, predictable positions…"
> — <https://developer.apple.com/design/human-interface-guidelines/design-principles>

**Why this deviates**

The detail is a level of hierarchy, but the back affordance lives inside the scrolling content (it scrolls away) and is a worded button. The top bar never confirms which instance is open. Switching to Library to install a mod and back loses the detail, the selected tab and the scroll position, which breaks the spatial model the HIG asks a navigation model to preserve.

**Concrete fix**

- When `selected_instance` is `Some`, set `TopAppBar.navigation_icon` to an `IconButton` with the standard back arrow and an accessible "Back" label; set the bar title to the instance name and the subtitle to `version · loader`.
- Keep `selected_instance` and `instance_tab` per destination instead of clearing them in `set_page`; restore the scroll offset with a stored `ScrollHandle` per page.
- Add Escape and Alt+Left as back, and only clear the detail after the user explicitly returns to the list.
- Put the back control in the bar, not in the content; the content starts with the instance identity row.

**Acceptance:** the back arrow is always visible in the bar while in detail; Escape/Alt+Left return to the list; switching to Library and back restores the instance, tab and scroll; the bar names the open instance.

---

#### HIG-09 — Server and account removal are immediate; the delete-instance dialog does not name the target

**Evidence**

- `src/ui/pages.rs:2152-2160` — server remove `IconButton` calls `remove_server` directly; no dialog or undo (`mod.rs:1378-1391`).
- `src/ui/pages.rs:1993-2002` — account remove `IconButton` calls `remove_account` directly (`mod.rs:1348-1358`).
- `src/ui/dialogs.rs:607-627` — `Dialog::DeleteInstance(id)` renders a generic `dialog.delete_confirm`; the instance name is not in the dialog.
- `src/ui/dialogs.rs:732-763` — `DeleteBackup` does include `label`, so the pattern exists but is not applied to instances.
- `src/ui/dialogs.rs:629-656` — Remove Content includes the filename (good).

**HIG rule**

> "Warn people when they initiate a task that can cause data loss that's unexpected and irreversible."
> — <https://developer.apple.com/design/human-interface-guidelines/feedback>

> "Write a title that clearly and succinctly describes the situation. You need to help people quickly understand the situation…"
> — <https://developer.apple.com/design/human-interface-guidelines/alerts>

> "If there's a destructive action, include a Cancel button to give people a clear, safe way to avoid the action."
> — same page

**Why this deviates**

Deleting a saved server or account is irreversible from the UI and is triggered by a small trailing icon next to Ping/Join, so a misclick loses data with no recovery. The instance delete dialog is better, but it asks "Delete this instance?" without saying which one; with several similarly named instances the confirmation is not informative. The app is inconsistent: high-risk instance delete confirms, lower-risk server delete does not.

**Concrete fix**

- Extend `Dialog::DeleteInstance` to carry both `id` and `name`; render the name in `supporting_text` alongside the consequence ("This removes the game directory and cannot be undone").
- Add a `Dialog::DeleteServer { id, name }` and `Dialog::RemoveAccount { id, name }` with the same `destructive_button` tone; or show a `Snackbar` with an Undo action immediately after removal.
- Keep Cancel as a text button; use a `MaterialMenu` for row overflow so Delete is not adjacent to Join/Ping.

**Acceptance:** every irreversible removal has either a named confirmation or an Undo; destructive actions use the error tone and are never the default/filled action; Cancel is always available.

---

#### HIG-10 — Snackbars overlay navigation, expire on a fixed timer, and hold only one message

**Evidence**

- `src/ui/mod.rs:2195-2213` — `div().absolute().left(px(24.)).bottom(px(24.))` wraps the `Snackbar`; at compact this sits over the bottom navigation bar.
- `src/ui/mod.rs:717-732` — `show_toast` stores one `Toast` and always clears it after 4 s, regardless of `SnackbarDuration`.
- `src/ui/mod.rs:734-737` — `show_error` routes every failure through the same transient toast.
- There is no queue; a second error replaces the first.

**HIG rule**

> "The most effective feedback tends to match the significance of the information to the way it's delivered. … In contrast, a warning about possible data loss needs to interrupt people so they have a chance to avoid the problem."
> — <https://developer.apple.com/design/human-interface-guidelines/feedback>

> "Minimize use of time-boxed interface elements. Views and controls that auto-dismiss on a timer can be problematic for people who need longer to process information, and for people who use assistive technologies that require more time to traverse the interface. Prefer dismissing views with an explicit action."
> — <https://developer.apple.com/design/human-interface-guidelines/accessibility>

**Why this deviates**

The snackbar is the app's only error channel, yet it is positioned over the primary navigation, disappears before a slow reader can finish, and is overwritten by the next message. An install or login error can be gone before the user knows what happened. The 4 s timer ignores the component's own `Long` (10 s) and `Indefinite` options.

**Concrete fix**

- Place the snackbar above the active navigation form: bottom offset = bar height + 16 dp at compact, or 24 dp at rail widths; use `Snackbar::position(SnackbarPosition::Center)` with a max width of 672 dp.
- Give `Yuhina` a `VecDeque<Toast>` and render one at a time; drop duplicates and advance on dismiss.
- Errors: `SnackbarDuration::Indefinite` with a Retry action and a Close action. Success confirmations: `Short`. Do not clear errors with the timer.
- Keep inline error panels for page-level failures; the snackbar is for transient, actionable feedback, not for replacing them.

**Acceptance:** no snackbar covers a navigation item; errors stay until dismissed or retried; two failures in a row are both readable; long messages are not cut off.

---

#### HIG-11 — Window chrome: a dark caption in the light theme and an app-name title

**Evidence**

- `src/main.rs:59-61` — native title bar with `title: Some("Yuhina")`, `appears_transparent: false`; no appearance sync.
- `design/reviews/rewrite-m3e/home-light.png` and `settings-native-light.png` — the Windows caption is near-black above a near-white app. The client changed to Expressive after these captures, but `main.rs` still uses the same native title-bar options.
- `src/main.rs:55-58` — a fixed 1440×900 window at `(24, 24)`; no explicit minimum size.
- The uncommitted `build.rs` + `resources/icons/yuhina.ico` work now supplies an app icon; keep it.

**HIG rule**

> "Don't title windows with your app name. Your app's name doesn't provide useful information about your content hierarchy or any window or area in your app…"
> — <https://developer.apple.com/design/human-interface-guidelines/toolbars>

> "Make sure custom windows use the system-defined appearances. … When you use system-provided components, a window's background and button appearances update automatically when the window changes state…"
> — <https://developer.apple.com/design/human-interface-guidelines/windows>

> "Ensure that your app looks good in both appearance modes."
> — <https://developer.apple.com/design/human-interface-guidelines/dark-mode>

**Why this deviates**

The caption is the highest-contrast band on the light screen, so the window reads as a dark app with a light body; the theme switch looks broken. The title "Yuhina" repeats the brand and tells the user nothing about the current page, and the fixed 1440×900 origin can place the window partly off-screen on a 1366×768 laptop. The lack of a minimum size lets the shell be resized below the width at which the compact bar works.

**Concrete fix**

- Prefer the kit's `TitleBar`/`CaptionLabels`/`Caption` with `appears_transparent: true` so the caption is painted with `ColorRole::SURFACE` in both themes; it already provides the window controls. If the native frame is kept, sync the platform appearance to the theme where the windowing layer exposes it. Do not hand-draw caption buttons or add a platform-specific dependency for this.
- Set the window title to the current page or instance (e.g. "Instances", or the instance name in detail), keeping the app name only in the executable/taskbar identity.
- Set a minimum window size (at least 480×600; better 800×600 if the product wants the promised desktop floor) and clamp the initial bounds to the work area.
- Keep the app icon work; it is a HIG-positive change.

**Acceptance:** light theme shows a light caption, dark shows dark; the title reflects the page/instance; the window opens fully on-screen at 1366×768 and cannot be resized below the usable floor.

---

#### HIG-12 — Two different compact-width formulas and hard-coded breakpoints

**Evidence**

- `src/ui/mod.rs:2017-2019` — `let scale = f32::from(window.rem_size()) / BASE_PX; let viewport_width = f32::from(window.viewport_size().width) / scale; let compact_actions = viewport_width < 600.;`
- `src/ui/pages.rs:39` — `let compact = f32::from(window.viewport_size().width) < 600.;` (no scale).
- `src/ui/mod.rs:2110-2121` — `compact = viewport_width < 600.`, `expanded = viewport_width >= 840.`, rail 96/220.
- The kit already exposes `WindowWidthClass`/`WindowSizeClass` (`crates/material/src/components/adaptive.rs`).

**HIG rule**

> "Determine layout based on size classes, not device type or orientation. Size classes describe the actual space available…"
> — <https://developer.apple.com/design/human-interface-guidelines/layout>

> "Design a layout that adapts gracefully and consistently. People expect their experience to remain familiar when they rotate their device, resize a window…"
> — same page

**Why this deviates**

The shell and the page frame can disagree about whether the window is compact. With a non-1.0 rem scale (exactly what HIG-01 introduces), `mod.rs` divides the viewport by the scale while `pages.rs` does not, so one can choose the bottom bar while the other keeps the 32 dp bottom padding. At 100% they agree, which is why the current captures hide the defect; at 150/200% the mismatch becomes a real clip.

**Concrete fix**

- Add one `fn layout_class(window: &Window) -> WindowWidthClass` in `mod.rs` using the kit's `WindowSizeClass::current(window)` or a single scaled-width helper.
- Pass the class to `pages::render`; use it for the rail/bar choice, the page gutter, and the compact bottom padding. Remove the literal `600.`/`840.` from both files.
- Add tests for 599/600/839/840 and for 150%/200% text scale.

**Acceptance:** the navigation form and page frame always agree; resizing across each breakpoint produces one clean transition; 200% text does not change the breakpoint semantics.

---

#### HIG-13 — Blocking commands run on the UI thread and buttons have no busy state

**Evidence**

- `src/ui/mod.rs:986` — `instances::create(&paths, new)` is called synchronously from the Create button.
- `src/ui/mod.rs:1255` — `accounts::add_offline` synchronously.
- `src/ui/mod.rs:1364` — `servers::add` synchronously.
- `src/ui/mod.rs:1844` — `modrinth::remove_file` synchronously.
- `src/ui/mod.rs:1037-1053` — `open_folder` calls `create_dir_all` on the UI thread.
- `src/ui/dialogs.rs:301-318` — the Create button is never disabled or put into a loading state; a slow disk can be clicked twice.

**HIG rule**

> "Configure a button to display an activity indicator when you need to provide feedback about an action that doesn't instantly complete. … the label 'Checkout' could change to 'Checking out…' while the activity indicator is visible."
> — <https://developer.apple.com/design/human-interface-guidelines/buttons>

> "Show people when a command can't be carried out and help them understand why."
> — <https://developer.apple.com/design/human-interface-guidelines/feedback>

**Why this deviates**

A create/add/remove that touches the filesystem or network can freeze the render thread with no spinner and no disabled button, which reads as a hang and invites a second click. It also violates the repository's own rule that blocking work runs on `cx.background_executor()` (`AGENTS.md:52-53`).

**Concrete fix**

- Wrap each of these in the existing `spawn_task`; keep the dialog open until the result arrives.
- Track a `busy: bool` per dialog; pass `.loading(busy, t("common.creating"))` to the confirm `MaterialButton` and `.disabled(busy)` to both actions.
- On failure, set `dialog_error` and keep the dialog open; focus the first invalid field. On success, close and toast.
- Use the kit's `ButtonProgressPlayback`/`.loading()` for the visual; do not invent a spinner.

**Acceptance:** no filesystem/network call runs on the render thread; the confirm button cannot be double-submitted; success and failure both end in a visible state.

---

#### HIG-14 — System theme, system locale, and system reduced motion are not honoured

**Evidence**

- `src/core/settings.rs:128-134` — `ThemePreference::System => true` (always dark).
- `src/core/settings.rs:138-145` — the locale comes from `LANG`/`LC_ALL`, normally unset on Windows.
- `src/main.rs:51` — `matches!(settings.reduce_motion, ReduceMotion::On)`; `System` resolves to false.
- `src/ui/pages.rs:2477-2504` — the motion chips save the value and call `set_reduce_motion(value == On)`; selecting System sets it to false.
- `src/ui/pages.rs:2426-2441` — only Dark and Light theme chips are offered, even though the enum has System.
- `src/ui/mod.rs:803-813` — `toggle_theme` always flips and persists an explicit preference.

**HIG rule**

> "Avoid offering an app-specific appearance setting. An app-specific appearance mode option creates more work for people because they have to adjust more than one setting to get the appearance they want."
> — <https://developer.apple.com/design/human-interface-guidelines/dark-mode>

> "Ensure that your app looks good in both appearance modes."
> — same page

> "When this setting is active, ensure your app or game responds by reducing automatic and repetitive animations, including zooming, scaling, and peripheral motion."
> — <https://developer.apple.com/design/human-interface-guidelines/accessibility>

**Why this deviates**

The UI advertises "Follow system" for motion but does not read the OS, and the theme has no System option at all. A Windows user with a light OS, Chinese locale and "Show animations" off still gets a dark, English, animated app on first launch. The "System" values exist in the data model, so this is a broken control rather than a product decision.

**Concrete fix**

- Resolve `ThemePreference::System` from the OS appearance (Windows `AppsUseLightTheme`, macOS `NSApp.effectiveAppearance`) at startup and when the setting changes; add a third `ConnectedButtonGroup` segment "System".
- Resolve `LanguageMode::System` with the platform locale API (Windows `GetUserDefaultLocaleName`), not `LANG`.
- Resolve `ReduceMotion::System` from the OS ("Show animations" on Windows, "Reduce motion" on macOS) and call `cx.set_reduce_motion` immediately when the chip changes.
- Keep explicit Dark/Light and zh/en overrides, but default new installs to System for all three.

**Acceptance:** a fresh profile with a light, zh-CN, reduced-motion OS starts light, Chinese and without transforms; changing the OS setting is reflected on the next launch or immediately; all three chips apply without a restart.

---

#### HIG-15 — The only switch in the app has no visible label

**Evidence**

- `src/ui/pages.rs:2641-2647` — `MaterialSwitch::new("java-auto-download", app.t("settings.java_auto_download")).checked(...).on_change(...)`.
- The kit switch defaults `show_label: false` and only paints the label when `.show_label(true)` is called (`crates/material/src/components/switch.rs:63-73, 105-110`).
- `design/reviews/rewrite-m3e/settings-native-light.png` shows the Settings page above the Java section; the code path is the only switch in the product. The label string is used as the accessible name but is not drawn.

**HIG rule**

> "Make sure the meaning of each control is clear. Don't make people guess or experiment to figure out what a toolbar item does."
> — <https://developer.apple.com/design/human-interface-guidelines/toolbars>

> "Label interface elements appropriately."
> — <https://developer.apple.com/design/human-interface-guidelines/accessibility>

**Why this deviates**

A bare toggle in the "Java runtime" panel gives no visible clue whether it downloads Java automatically, enables Java detection, or something else. The user must toggle it to learn, and a screen-reader user gets a name with no visible parity.

**Concrete fix**

- Prefer the M3 settings-row pattern: `ListItem::new("java-auto-download", t("settings.java_auto_download")).supporting(t("settings.java_auto_download_hint")).trailing(ListTrailing::Switch(checked)).on_click(toggle)`. This gives a 56/72 dp row, a visible label, a supporting line and a full-row target.
- If the bare switch is kept, add `.show_label(true).supporting_text(...)` and place it in a flex row with the label leading.

**Acceptance:** the switch has a visible label and supporting text, the whole row toggles it, and the accessible name matches the visible label in both languages.

---

#### HIG-16 — One status concept is painted with three container roles, and active uses two accents

**Evidence**

- `src/ui/pages.rs:581-599` — running = `TERTIARY_CONTAINER`, installed = `SECONDARY_CONTAINER`, not installed = `SURFACE_CONTAINER_HIGHEST`.
- `src/ui/pages.rs:310-327` — the same three-way chip on Home cards.
- `src/ui/pages.rs:1953-1979` — the active account row uses `SECONDARY_CONTAINER` while its "Active" chip uses `PRIMARY_CONTAINER`.
- `design/reviews/md3e-audit-fix/instances-1000-dark-zh.png` — three "已安装" mauve chips, one per row, competing with the mint Play button.
- `design/reviews/rewrite-m3e/accounts-native.png` — the active row and the "当前" chip use two different accent families (layout unchanged after the fix; the raw UUID and ghost button were fixed).

**HIG rule**

> "Avoid using the same color to mean different things. Use color consistently throughout your interface, especially when you use it to help communicate information like status or interactivity."
> — <https://developer.apple.com/design/human-interface-guidelines/color>

> "Avoid relying solely on color to differentiate between objects, indicate interactivity, or communicate essential information."
> — same page

> "Aim for sufficient color contrast in all appearances."
> — <https://developer.apple.com/design/human-interface-guidelines/dark-mode>

**Why this deviates**

"Mauve", "pink" and "grey" are used for installed/running/not-installed, while mauve also means "selected navigation" and "active account row". The eye cannot learn a status vocabulary. Painting "Installed" on every row also makes the normal state look exceptional, so the one exceptional state (Running) does not stand out. The active account encodes one state with two different containers, which weakens the selected-row signal.

**Concrete fix**

- Use the kit's semantic status layer: `feedback::status_chip(StatusKind)` or `Badge::large(...).color(BadgeColor::Success/Warning/Info)` for exceptional states. Show nothing for the default "Installed"; show a `Badge` only for Running (Success) and Not installed (Warning).
- Use one accent for the active account: `ListItem.selected(true)` (secondary container row) with no separate chip, or a single `Badge` "Active" on a neutral row.
- Keep the text label on every status so color is never the only signal.
- Add a visual regression capture of Instances + Accounts in both themes.

**Acceptance:** each state has one role and one text label; the default state is quiet; Running is the only green status; the selected account row and its badge do not use two accent families.

---

#### HIG-17 — Home's feed grid stretches the last item and the news card is one giant empty panel

**Evidence**

- `src/ui/pages.rs:227-229` — `grid()` is `flex_wrap` with a 16 px gap.
- `src/ui/pages.rs:231-241` — `card_shell()` is `flex_1().min_w(260.)`, so the last card in a row grows to fill it.
- `src/ui/pages.rs:369-402` — news cards use `flex_1().min_w(260.)`; with one item the card spans the full column.
- `design/reviews/md3e-audit-fix/home-1440-dark-zh.png` — the single news card spans the entire 1120 column; the "Read more" action sits centered in a large empty field.
- `design/reviews/rewrite-m3e/home-light.png` — the recent grid is 2 + 1; the third card is full-width (layout code unchanged).

**HIG rule**

> "Use consistent spacing. When content isn't consistently spaced, it no longer looks like a grid and it's harder for people to scan."
> — <https://developer.apple.com/design/human-interface-guidelines/layout>

> "Align elements to make them easier to scan… People assume that aligned items are related to each other…"
> — same page

> "The system includes predefined layout guides that make it easy to apply standard margins around content and restrict the width of text for optimal readability."
> — same page

**Why this deviates**

The grid changes shape based on item count instead of available width. At 1000 the third recent card is a different object from its siblings; at 1440 the news card is 3–4× the width of a normal feed item, so its content reads as a header with a button floating in the middle. The feed no longer looks like a grid, and the eye cannot compare items.

**Concrete fix**

- Replace `flex_wrap` with a real column model: 3 equal columns at ≥1200, 2 at 840–1199, 1 below 840. Use `flex_basis(px(...))`, `flex_grow(0)`, `flex_shrink(0)`, or the kit's `CardGroup`/`MaterialCarousel` where appropriate.
- Do not let the last item grow; a short last row leaves empty space at the end, which is correct.
- News: either a 3-column card grid (4 items → 3 + 1 with the last not stretched) or a vertical list of compact `ListItem`s with a clamped 2-line summary. Do not render one full-width card.
- Clamp descriptions and news text to 2 lines and put the full text in the details/open action.

**Acceptance:** at 1440, 1000, 800 and 500 there is a consistent number of columns; no last card stretches; a single news item looks like a feed item, not a page banner.

---

#### HIG-18 — Instance detail overloads the header with six actions

**Evidence**

- `src/ui/pages.rs:784-879` — Play, Install/Repair, Folder, Duplicate, Export, Delete in one wrapping row.
- `src/ui/pages.rs:733-748` — the Back button is above them, also in the content.
- `design/reviews/md3e-audit-fix/instances-1000-dark-zh.png` — three row actions already crowd the row at 1000; the detail row would be wider.
- `src/ui/pages.rs:883-910` — six tabs (Overview, Logs, Content, Backups, Files, Settings) with no overflow strategy.

**HIG rule**

> "Choose items deliberately to avoid overcrowding. … To accommodate variable view widths, define which items move to the overflow menu as the toolbar becomes narrower."
> — <https://developer.apple.com/design/human-interface-guidelines/toolbars>

> "Group toolbar items logically by function and frequency of use."
> — same page

**Why this deviates**

Six equal-weight actions flatten the hierarchy, put a destructive Delete one misclick from Export, and crowd the title at 1000/800. The six tabs may also overflow at 500 or 200% text; there is no test or overflow treatment.

**Concrete fix**

- Keep Play (`Filled`) and Install/Repair (`Tonal`) visible.
- Move Folder, Duplicate, Export and Delete into a `MaterialMenu` triggered by a `more_vert` `IconButton` with an accessible label; group Delete at the end after a `divider` and use the error tone.
- Put Back in the `TopAppBar` navigation slot (HIG-08) and the instance name in the bar.
- For the tabs: use `MaterialTabs` with horizontal scrolling and an edge fade at narrow widths, or move Files/Settings into a `MaterialMenu`; verify at 500 and 200%.

**Acceptance:** at most two prominent actions and one overflow in the detail header; Delete is separated and confirmed; tabs remain reachable at 500 and 200% without clipping.

---

### P2 — polish, consistency, and follow-up

#### HIG-19 — Spacing, radii and icon sizes are off the app's own rhythm

**Evidence:** `pages.rs` gaps are 2/4/6/8/10/12/14/16/24; radii are 14/16/18/20/22/24/32/999; icons are 16/18/20/22/24/26/32. `docs/design-system.md:57-64` defines a 24/16/10 rhythm and 32/24/20/14–16 radii; M3's corner scale is 4/8/12/16/20/28/32/48. The 18/22/24 radii and 18/22/26 icons are not steps in either system.

**HIG rule:** "Align elements to make them easier to scan…" and "Use consistent spacing" — <https://developer.apple.com/design/human-interface-guidelines/layout>; "Care about every detail" — <https://developer.apple.com/design/human-interface-guidelines/design-principles>.

**Why this deviates:** The eye reads 18/22/24 as different levels even when they are meant to be the same; the inconsistency is visible when rows from different pages sit together, and it stops the radius from communicating hierarchy the way `docs/design-system.md:46-55` intends.

**Fix:** define `space::{XS,S,M,L,XL} = 4/8/12/16/24` and `radius::{ROW,CARD,DIALOG,HERO}` from the kit tokens; replace literal px spacing in page code; use 20/24 icons for component icons and 32 only for empty-state glyphs; keep 999 for status pills. **Acceptance:** grep shows no literal `gap(px(6|10|14))` or `rounded(px(18|22|24))` in page code; visual regression shows the same hierarchy.

#### HIG-20 — Nested boxes instead of separators

**Evidence:** `pages.rs:1375-1381` wraps backup rows in a panel, and each row (`pages.rs:1283-1287`) is another `SURFACE_CONTAINER` card with an 18 dp radius. The content tab (`pages.rs:1193-1228`) does the same for mod lists. Settings panels contain rows with no dividers (`pages.rs:2523-2554`). This creates "boxes inside boxes" and inconsistent density.

**HIG rule:** "Group related items to clearly express related information or functions. For example, you might use negative space, container shapes, or separator lines…" — <https://developer.apple.com/design/human-interface-guidelines/layout>.

**Why this deviates:** Nested fills flatten the tonal hierarchy: the inner card and the outer panel are both "a surface", so neither reads as the container. The extra 18 dp radius also blurs the distinction between a list row and a card.

**Fix:** one container per section; `ListItem` rows with `divider(true)` inside it; reserve cards for Home's feed. **Acceptance:** no more than one tonal container around a list; dividers separate rows without a second fill.

#### HIG-21 — Empty, loading and error states are inconsistent or dead ends

**Evidence:** `pages.rs:2259-2417` — Activity has two empty states with icons but no action, and finished installs are never dismissed; `pages.rs:514-517` — Home news empty state is a bare sentence in a panel; `pages.rs:1360-1363` and `2650-2651` — bare "Loading…" text instead of a progress component; `pages.rs:157-172` — every error panel appends "check your network" even for local IO/validation errors; `pages.rs:678-712`, `2008-2046`, `2176-2214` — loading is only distinguished when the list is empty.

**HIG rule:** "Feedback helps people know what's happening, discover what they can do next, understand the results of actions, and avoid mistakes." — <https://developer.apple.com/design/human-interface-guidelines/feedback>; "Show people when a command can't be carried out and help them understand why." — same page.

**Why this deviates:** The state model exists but is not consistently rendered, so a first-run or offline user sees an empty list as if it were known to be empty, and a finished install looks the same as a failed one. The generic network hint misdiagnoses local failures, which makes recovery harder rather than easier.

**Fix:** every empty state gets a next action (create, browse, retry); use `LoadingFrame`/`LinearProgress` for loading, `notice_banner`/`error_panel` with a Retry for errors, and an explicit dismiss/clear for finished installs; only show the offline hint for `ErrorCode::Network`/`Http`/`Checksum`. **Acceptance:** no dead-end empty state; no bare "Loading…"; the offline hint appears only for network errors.

#### HIG-22 — Typography family, CJK fallback, metadata size, and logs

**Evidence:** `src/ui/mod.rs:2224` hard-codes `.font_family("Roboto")`; no application font asset is bundled (`design/` has only brand SVGs and `Cargo.toml` adds no font dependency). The kit's `TypeScale` also uses a family; CJK falls back per-glyph. `pages.rs:1033-1035` uses `BodySmall` + 18 dp for the in-app log preview but not `monospace`; only the file preview (`pages.rs:1508`) uses monospace. Instance metadata uses `muted`/`BodySmall` at 12 px (`pages.rs:121-125`) for version, loader and memory, which is the primary identifying information in a list.

**HIG rule:** "Use font sizes that most people can read easily" (macOS default 13 pt, minimum 10 pt) — <https://developer.apple.com/design/human-interface-guidelines/typography>; "Minimize the number of typefaces you use" — same page; "Make sure all your app's colors work well in light, dark…" — <https://developer.apple.com/design/human-interface-guidelines/color>.

**Why this deviates:** The declared family is not guaranteed on Windows, CJK fallback is per-glyph, and 12 px metadata carries the identifying information (version/loader/memory): at 100% it is small, at 200% it is the first thing to collide. A log without a monospaced face is harder to compare column-wise.

**Fix:** use the platform system font stack or bundle the family with a CJK fallback (and declare it in the review doc); remove the root override; use `BodyMedium` 14 for list metadata that identifies the item and `BodySmall` only for tertiary hints; use a monospaced role for logs and file preview. **Acceptance:** the same font renders for zh-CN/en-US at 100–200%; no missing glyphs; metadata is legible at 800.

#### HIG-23 — The compact More menu hides two top-level destinations behind an icon-only menu

**Evidence:** `src/ui/mod.rs:2048-2083` — at <600 dp Activity and Settings move into a `MaterialMenu` behind a `more_vert` icon; the theme/language icons sit outside it. `src/ui/mod.rs:2169-2185` — the bottom bar has five destinations and no Settings/Activity.

**HIG rule:** "Avoid overflow tabs. … The More tab makes it harder for people to reach and notice content on tabs that are hidden…" — <https://developer.apple.com/design/human-interface-guidelines/tab-bars>; "Don't disable or hide tab bar buttons…" — same page.

**Why this deviates:** A top-level destination that is only reachable through an icon-only overflow is easy to forget, and the bottom bar gives no hint that Activity and Settings exist. The HIG explicitly warns that hidden tabs are harder to reach and notice.

**Fix:** keep five primary destinations, but make the More menu discoverable: add the localized "More" label (not just a tooltip), list Activity and Settings with leading icons, and add a `Badge` on Activity when a game is running or an install is active. Alternatively swap Servers for Settings in the bar and put Servers in More, depending on which is used more. **Acceptance:** at 500 the overflow is labelled and both destinations are one tap away; Activity carries a running/install badge.

#### HIG-24 — Motion: no page continuity, and "System" reduce-motion is inert

**Evidence:** `src/ui/pages.rs:4-14` swaps the whole page with no transition; `src/ui/mod.rs:2005-2249` rebuilds the content; `src/main.rs:51` and `pages.rs:2495-2500` only honour `On`. `docs/design-system.md:68-72` promises reduced motion disables transforms; `System` does not.

**HIG rule:** "Add motion purposefully, supporting the experience without overshadowing it." — <https://developer.apple.com/design/human-interface-guidelines/motion>; "Make motion optional." — same page; "Let people cancel motion." — same page.

**Why this deviates:** The instant swap loses the spatial relationship between Home, the list and the detail; the setting that is supposed to make motion optional does nothing for System users, so the preference is not actually honoured.

**Fix:** add a 120–160 ms fade-through on page change and on detail enter/exit (the kit's `token_transition`/`PaneTransition`), disabled under reduced motion; resolve `System` from the OS; keep the theme spring only for theme changes. **Acceptance:** reduced-motion System disables transforms without restart; page changes fade rather than snap; no animation blocks input.

#### HIG-25 — Window sizing: no explicit minimum and a fixed origin

**Evidence:** `src/main.rs:55-58` — `window_bounds` at `(24, 24)` with `1440×900`, no min size. `main.rs:12-20` allows `YUHINA_SIZE` for tests but no clamp to the work area. The merged 1440 capture is host-clipped, which is a host limitation; the app still has no policy.

**HIG rule:** "Choose a minimum and maximum size for each window to help keep your content looking great. … If you don't set a minimum and maximum size … people could make it so small that UI elements overlap…" — <https://developer.apple.com/design/human-interface-guidelines/windows>; "Avoid placing controls or critical information at the bottom of a window." — same page (macOS).

**Why this deviates:** A 1440×900 window at (24, 24) is larger than a 1366×768 work area, and without an explicit minimum the compact shell can be resized into overlap. The app's own definition of done names 800×600 as the floor, so the window should enforce a matching minimum.

**Fix:** clamp the initial bounds to the work area (and center them); set a minimum of 480×600 (or 800×600 if the product wants the promised floor); keep the 500×700 capture working. **Acceptance:** the window is fully on-screen at 1366×768 and cannot be resized into overlap.

#### HIG-26 — i18n fallback, hard-coded guest, and string concatenation

**Evidence:** `src/ui/i18n.rs:5-15` returns `"…"` for a missing key, which hides defects; `src/ui/pages.rs:257` hard-codes `"Steve"` instead of `home.greeting_guest`; `pages.rs:630-634` concatenates `"{} · {} · {} MB"`; `pages.rs:298-301` concatenates version/loader; `pages.rs:1629-1636` concatenates backup time/bytes/files; `pages.rs:2118-2125` concatenates MOTD/counts/latency/version. The key-parity test exists (`i18n.rs:837-850`) but no test fails on a missing key at runtime.

**HIG rule:** "Be concise. … Choose exactly the words you need to convey a concept or label a control." — <https://developer.apple.com/design/human-interface-guidelines/design-principles>; "Localization … use … resources" is also a product rule (`AGENTS.md:30`).

**Why this deviates:** A missing key renders as an ellipsis in production, so a translation gap looks like a layout bug; concatenated English fragments cannot be reordered by a translator and produce broken zh strings when the value order differs.

**Fix:** make `tr` panic in debug or log a visible warning for a missing key; add `home.greeting_guest`, `common.unit_mb`, `servers.status_format`, `backups.summary_format` and use `.replace` with placeholders; never concatenate translated fragments. **Acceptance:** debug builds fail on a missing key; no user-facing string is assembled from English fragments; +30% English passes at 800.

#### HIG-27 — Dense secondary text: raw UUID, MOTD blob, unclamped descriptions

**Evidence:** `pages.rs:1949, 1970` — the account secondary line is `kind · short_uuid`; the 8-char UUID is still not meaningful. `pages.rs:2117-2127` — server status is a single `MOTD · 12/100 · 34 ms · 1.21.4` line with no clamping. `pages.rs:1803-1813` — Modrinth descriptions are unbounded and can run to 2–3 long lines. `pages.rs:385-391` — news body unbounded.

**HIG rule:** "Keep item text succinct so row content is comfortable to read." and "Consider ways to preserve readability of text that might otherwise get clipped or truncated." — <https://developer.apple.com/design/human-interface-guidelines/lists-and-tables>; "Keep text truncation to a minimum as font size increases." — <https://developer.apple.com/design/human-interface-guidelines/typography>.

**Why this deviates:** Variable-length text makes rows jump and destroys the scan rhythm; a raw UUID and a four-field MOTD line are not scannable, and they are the primary secondary information on those rows. A 90-character description also exceeds a comfortable measure.

**Fix:** hide the UUID behind a details row; make the server supporting line a 2-line clamp with latency in a `Badge`; clamp library/news descriptions to 2 lines with a details view; add a tooltip with the full text. **Acceptance:** every row has a stable height at 100–200% and both languages; full text is reachable without layout shift.

#### HIG-28 — Six tabs have no overflow strategy

**Evidence:** `pages.rs:883-910` — six `MaterialTabs` items; at 500 the content width is ~436 dp and the English labels total more than that. No capture of the detail at 500 exists; the risk is untested. At 200% it is certain.

**HIG rule:** "Avoid overflow tabs." — <https://developer.apple.com/design/human-interface-guidelines/tab-bars>; "Limit the number of segments…" — <https://developer.apple.com/design/human-interface-guidelines/segmented-controls>.

**Why this deviates:** Six labels exceed the 436 dp content width at 500, and the risk is worse at 200%; a clipped tab is a hidden destination, which is the failure mode the HIG calls out for overflow tabs.

**Fix:** allow horizontal scrolling with an edge fade, or move Files/Settings into a `MaterialMenu`; test at 500 and 200%. **Acceptance:** all six tabs are reachable at 500 and 200% with no clipped label or hidden tab.

---

## Component mapping

The kit at revision `26c0cb1` already provides every component below; no new dependency is required.

| Current bespoke pattern (file:line) | Kit component | Why |
| --- | --- | --- |
| Instance row `div` (`pages.rs:600-674`) | `ListItem` (two-line) + sibling trailing `MaterialButton` | Standard 56/72/88 dp height, ripple, focus, selection, truncation; row becomes the navigation target |
| Recent card `card_shell` (`pages.rs:231-241, 270-366`) | `MaterialCard::new(id, CardVariant::Filled)` + `CardGroup` | Correct card padding/elevation/interaction; consistent radius from tokens |
| News card `div` (`pages.rs:376-401`) | `MaterialCard` in a 3-column `CardGroup` | Prevents the full-width empty banner; gives press/focus states |
| Library result `div` (`pages.rs:1777-1826`) | `ListItem` (three-line) + trailing Install | Scannable rows, 2-line clamp, separate row and action targets |
| Account row `div` (`pages.rs:1950-2005`) | `ListItem.selected(active)` + `Badge` | One accent for active, radio/selection semantics, no duplicate chip |
| Server row `div` (`pages.rs:2090-2172`) | `ListItem` + `Badge` + trailing Join | MOTD/counts in supporting text; remove/ping into a menu |
| Backup row `div` (`pages.rs:1283-1356`) | `ListItem` with `divider(true)` | Removes the nested card and the 18 dp off-token radius |
| Java runtime row `div` (`pages.rs:2527-2553`) | `ListItem` `.trailing_text(path)` + `Badge` | Settings-style row; no bespoke 6 px gaps |
| File list `ListItem` (`pages.rs:1457-1473`) | Keep | The one correct list row in the app |
| Library `TextField` + Search button (`pages.rs:1751-1768`) | `SearchField` + `IconButton` clear | Search icon, clear button, immediate filtering |
| Settings theme/language/motion chips (`pages.rs:2426-2505`) | `ConnectedButtonGroup` (segmented, single) | These are one-of-N choices, not multi-select filters; HIG segmented guidance |
| Memory chips (`pages.rs:2507-2521`, `dialogs.rs:224-237`) | `MaterialSlider` or `ExposedDropdown` | A numeric value should not be a row of chips; fewer tab stops |
| Account type chips (`dialogs.rs:329-342`) | `ConnectedButtonGroup` (segmented) or `MaterialTabs` | Three related form subviews; radio semantics |
| Version chips (`dialogs.rs:107-120`) | `ExposedDropdown` | Long list, one focus stop, menu max height |
| Loader chips (`dialogs.rs:128-140`) | `ConnectedButtonGroup` (5 segments) | At the segmented limit, equal widths |
| Loader build chips (`dialogs.rs:174-207`) | `ExposedDropdown` | Unbounded list; recommended first |
| Status chip helper (`pages.rs:131-139`) | `feedback::status_chip` / `Badge` | Semantic status roles; one vocabulary |
| Error panel (`pages.rs:157-172`) | `feedback::notice_banner` + Retry | Consistent banner, correct role pair, retry action |
| Loading panel (`pages.rs:174-190`) | `LoadingFrame`/`LoadingIndicator` + `LinearProgress` | Loading affordance instead of text + progress |
| Empty state (`pages.rs:192-225`) | `MaterialCard` container + icon + action | Keeps the expressive empty state but on a tokenized card |
| Top-bar action overlay (`mod.rs:2226-2235`) | `TopAppBar::actions(...)` | Correct trailing-edge layout, overflow, title truncation, focus order |
| Compact FAB (`mod.rs:2186-2193`) | `BottomAppBar` + docked `Fab` | Safe-area lane; no overlap with content |
| Snackbar wrapper (`mod.rs:2195-2213`) | `Snackbar::position(...)` + root inset | Above navigation, centered, max width, queue |
| Back text button (`pages.rs:738-748`) | `TopAppBar.navigation_icon(IconButton arrow_back)` | Standard back symbol in the expected place |
| Detail action row (`pages.rs:784-879`) | 2 buttons + `MaterialMenu` overflow | One/two prominent actions, destructive separated |
| Switch without label (`pages.rs:2641-2647`) | `ListItem` + `ListTrailing::Switch`, or `.show_label(true)` | Visible label, full-row target, accessible parity |
| `div` logo (`mod.rs:2273-2285`, now an SVG mark) | Keep the SVG, mark it decorative | Current brand mark is good; no pseudo-button background |

## Prioritized implementation plan

### Phase 1 — foundations (P0, then the top P1s)

**`src/ui/mod.rs`**

1. Add a single `layout_class(window)` helper (kit `WindowSizeClass`/`WindowWidthClass`); use it for rail/bar and page compact mode; delete the duplicate `600.`/`840.` math at `2017-2021` and `pages.rs:39`.
2. Move the trailing actions into `TopAppBar::actions`; remove the absolute overlay at `2226-2235`.
3. Define the FAB policy per page; on compact, use `BottomAppBar` with a docked FAB or hide the FAB where an inline primary exists (`2111-2193`).
4. Replace the toast slot with a queue and place the `Snackbar` above the navigation form; errors use `Indefinite` + Retry/Close (`717-737`, `2195-2213`).
5. Do not clear `selected_instance` in `set_page` (`739-741`); store per-page selection/tab/scroll.
6. Cancel/ignore stale async results by generation id (the loader already has `loader_versions_key`; apply the same pattern to search/accounts).
7. Add the running/install `Badge` to the Activity action.

**Acceptance:** one breakpoint model; title and actions never overlap; no control under the FAB; errors persist; returning to a section restores its detail; compact and wide captures pass.

**`src/main.rs`**

1. Persist and pass `text_scale` to `init_component_theme`/`advance_component_theme`; add the 100/125/150/200 control in Settings.
2. Resolve `ReduceMotion::System` and `ThemePreference::System` from the OS; keep explicit overrides.
3. Use the kit `TitleBar`/`Caption` with `appears_transparent: true` (or sync the native appearance if the windowing layer supports it); keep the caption on `SURFACE` in both themes.
4. Set a minimum window size and clamp the initial bounds to the work area; title = page/instance.
5. Keep the app icon/build-resource work.

**Acceptance:** 200% reachable; System values behave; light caption light; window fully on-screen at 1366×768; no startup activation side effects.

### Phase 2 — component pass (P1)

**`src/ui/pages.rs`**

1. Replace the duplicate H1 in `page_header` with a `LargeFlexible` bar title/subtitle, or drop the bar title. Remove theme/language from the bar.
2. Make Home/Instances/Detail follow one-primary: hero/card/row variants per HIG-03; detail back in the bar and actions in a menu.
3. Convert instance, account, server, library, backup and Java rows to `ListItem`/`MaterialCard`; make the row the navigation target; add disclosure/press/focus.
4. Replace Library's field + button with `SearchField` + debounce + Enter; keep the type chips as a scope bar.
5. Convert Settings theme/language/motion/download to `ConnectedButtonGroup`; memory to `MaterialSlider`; label the Java switch with a `ListItem` row.
6. Replace the status chips with `feedback::status_chip`/`Badge`; quiet the default state.
7. Fix the Home grid to 3/2/1 columns with no last-item growth; make news a feed, not a banner; clamp descriptions.
8. Add explicit Activity actions (launch/browse) and dismiss/clear for finished installs; only show the offline hint for network errors.
9. Verify the six instance tabs at 500 and 200%; add overflow treatment.
10. Replace the 2/6/10/14 gaps and 18/22/24 radii with the spacing/radius tokens; use 20/24 icons.

**Acceptance:** every page has one primary action, one title, one status vocabulary, and kit list/card rows; the 500/800/1000/1440 captures show no clipping or overlap; tab/row focus works with the keyboard.

**`src/ui/dialogs.rs`**

1. Use `DialogSize::Medium` (560) on desktop and `FullScreen` below 600; `.width(560.)` if the kit's `Small` default is kept.
2. Replace version/loader/memory chips with `ExposedDropdown`/`ConnectedButtonGroup`/`MaterialSlider` as above.
3. Add `.track_body_scroll(&handle)` and `pb(24)`; never cut a line at the footer.
4. Carry the target name into `DeleteInstance`; add named confirmations for server/account removal; keep `destructive_button` for Delete.
5. Add `busy` state and `.loading()` to every confirm; validate fields inline with `TextField::error(true).supporting_text(...)`, not only a toast.
6. Make dividers/actions consistent across all dialogs; keep Escape/scrim dismissal.

**Acceptance:** the create dialog fits 800×600 without a cut line; every destructive action names its target; no dialog can be double-submitted; errors point at the field.

**`src/ui/i18n.rs`**

1. Add keys for every new label: text scale, System, More, Back, Active, Running, Not installed, Retry, install phases, delete-target copy, busy labels, status formats, units.
2. Replace fragment concatenation with placeholder strings (`{version}`, `{loader}`, `{mb}`, `{motd}`, `{players}`, `{latency}`).
3. Remove the `"…"` fallback in debug: log/panic so a missing key cannot ship.
4. Add `home.greeting_guest` usage instead of `"Steve"`.
5. Extend the parity test to assert no runtime fallback in a test pass over all `tr` keys.

**Acceptance:** zh-CN and en-US parity; no hard-coded user string; no `"…"` at runtime; +30% English fits 800.

### Phase 3 — polish and verification

1. 8 pt spacing/radius tokens; icon 20/24/32 scale.
2. Page fade-through, disabled under reduced motion; scroll restoration.
3. Window minimum/initial clamp; title bar appearance.
4. CJK font fallback and 200% matrix.
5. Screenshot matrix: 1440×900, 1000×800, 800×600, 500×700, light/dark, zh/en, 100/200%, every page and the create/delete dialogs.
6. Keyboard/a11y pass: tab order, focus rings, Enter/Escape, screen-reader names, no mouse-only action.

## Keep

These are working and must not regress:

- `ThemeId::Expressive` with semantic `ColorRole`s in both themes; no hex literals; the teal identity.
- Five primary destinations and the correct rail → bar adaptation: expanded 220 dp at ≥840, collapsed 96 dp at 600–839, bar below 600; labels always visible.
- The FAB as a compact primary action (keep one, docked above the navigation, not overlapping content).
- The 32/24/20 dp radius hierarchy on the hero/panels/rows (normalize the outliers, keep the hierarchy).
- `TypeRole` usage with explicit line heights; `HeadlineLarge` page titles, `DisplaySmall` hero, `LabelLarge` sections, `BodyMedium` body.
- Kit `MaterialButton`, `IconButton`, `FilterChip`, `TextField`, `MaterialSwitch`, `MaterialTabs`, `MaterialDialog`, `Snackbar`, `LinearProgress`, `Avatar`, `Fab` usage where it exists.
- The dialog padding wrapper (`px(24)/pb(8)`) and the create dialog's sticky footer; Escape/scrim dismissal.
- Destructive dialog text tone (`destructive_button`) and Create/Delete labels (not "OK").
- Loading/empty/error panels with Retry on Instances, Accounts, Servers, Library and Home; error-code localization.
- The compact shell's five labelled destinations and the 160 dp bottom clearance.
- i18n key parity test; error-code mapping; phase mapping; no raw core strings.
- Focus rings, icon-only accessible labels, reduced-motion `On` behavior, theme transition.
- No AI/chat/agent/MCP code; no telemetry.
- The brand mark SVG and app icon work in progress.

## Remaining functional gaps to fix in this round

1. No OS offline detector; error panels show an offline hint without probing connectivity.
2. No persisted text-scale setting or 150/200% captures.
3. `System` theme, locale and reduced motion are not read from the OS.
4. No explicit minimum window size and no work-area clamp; the native caption does not follow the theme.
5. No undo/confirm for server/account removal; the instance delete dialog does not name the target.
6. No busy/disabled state for create/add/remove; several filesystem calls run on the UI thread.
7. Section/detail/tab/scroll state is discarded on navigation; no history or Esc/Alt+Left.
8. No running/install badge on the Activity action; finished installs cannot be dismissed.
9. No 200% / 500 px / English screenshot coverage for Settings, Accounts, Servers, Library, Activity and the detail tabs.
10. No screen-reader/keyboard audit of the new list rows and the compact More menu.
11. Six instance tabs have no overflow treatment at 500/200%.
12. Library has no global shortcut/search entry point; search is page-local (acceptable, but document the decision).
13. Activity does not surface installer stdout (known core gap) and does not distinguish finished vs failed in the list once the install entry remains.
14. The `tr` fallback `"…"` can hide missing keys; no debug assertion.

## Verification

- Read: `AGENTS.md`, `docs/design-system.md`, `docs/product-spec.md`, `docs/architecture.md`, `docs/reviews/md3e-ui-audit.md`, `docs/reviews/md3e-audit-fix.md`, all of `src/ui/**`, `src/main.rs`, `Cargo.toml`, and the relevant kit component sources.
- Inspected: `design/reviews/rewrite-m3e/` (light theme, settings, accounts, home, instances), `design/reviews/md3e-audit-fix/` (1440 dark/light, 1000, 800, 500, create/add-account dialogs), `design/reviews/merged-m3e/` (1000/800/500 shell).
- Fetched and quoted: HIG `layout`, `typography`, `color`, `motion`, `accessibility`, `navigation-and-search`, `toolbars`, `lists-and-tables`, `modality`, `buttons`, `segmented-controls`, `search-fields`, `tab-bars`, `sidebars`, `windows`, `dark-mode`, `feedback`, `alerts`, and `design-principles`.
- No source, `Cargo.toml`, or other docs were modified. No commit or push. No new screenshots are committed; this report is the only file added.
