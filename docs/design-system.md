# Design System — Material 3 Expressive + Apple HIG

Yuhina keeps the Material 3 Expressive visual identity and uses Apple HIG for
hierarchy, spacing rhythm, interaction and accessibility. The kit is
`gpui-kit-material`; the palette is `ThemeId::Expressive`.

## Identity

- Expressive teal primary, tonal container surfaces, M3E shape and springs.
- One brand mark: `design/brand/yuhina-mark.svg` (currentColor).
- App icon: `resources/icons/yuhina.ico` generated from
  `design/brand/yuhina-icon.svg`.
- No gradients, glassmorphism, neon or game-texture backgrounds.

## Colour

Paint with semantic roles only:

```rust
div().bg(color(ColorRole::SURFACE_CONTAINER_LOW))
```

- Page ground: `SURFACE`.
- Navigation/top bar: `SURFACE` / `SURFACE_CONTAINER_LOW`.
- Cards and panels: `SURFACE_CONTAINER_LOW` / `SURFACE_CONTAINER`.
- Primary action: `PRIMARY` on `ON_PRIMARY`.
- Secondary surfaces: `SECONDARY_CONTAINER` on `ON_SECONDARY_CONTAINER`.
- Tertiary identity marks: `TERTIARY_CONTAINER` on `ON_TERTIARY_CONTAINER`.
- Errors: `ERROR_CONTAINER` on `ON_ERROR_CONTAINER`.
- Supporting text: `ON_SURFACE_VARIANT`.
- Dividers: `OUTLINE_VARIANT`.

Status uses one vocabulary: `feedback::status_chip`/`Badge` semantic kinds, not
three container roles for the same concept.

## Typography

Use kit type roles and explicit line heights:

| Role | Use |
| --- | --- |
| DisplaySmall | Home hero |
| HeadlineLarge | Page title |
| TitleMedium / TitleSmall | Card titles, row headlines |
| BodyLarge / BodyMedium | Body, primary metadata |
| BodySmall | Supporting text, never the only identifier |
| LabelLarge | Section labels and buttons |

- Family: the kit's family with platform CJK fallback; bundle a font only if it
  is licensed and documented.
- Logs and code: monospace, 12/18.
- Primary metadata in list rows is BodyMedium, not 12px BodySmall.
- Clamp long descriptions; never let a row grow without a line limit.

## Spacing and shape

- Grid: 8dp; 4dp sub-steps only for icon/text alignment.
- Page gutter: 32dp (rem-based); content measure: max 1120dp.
- Section gap: 24dp; card gap: 16dp; row gap: 8dp; panel padding: 20–24dp.
- Radii: 8 (small controls), 12 (menus), 16 (cards), 20 (rows), 24 (panels),
  28–32 (hero/large surfaces).
- Icons: 20 (inline), 24 (navigation/rows), 32 (empty states/brand).
- Use `rem(...)` for text-coupled geometry so 200% scaling reflows.

## Layout

```
┌──────────────────────────────────────────────────────────┐
│ TopAppBar: mark · page title · Activity · Settings · …   │
├───────────────┬──────────────────────────────────────────┤
│ Rail 96/220dp │ Page content, max 1120dp, 32dp gutter    │
│ FAB / docked  │                                          │
└───────────────┴──────────────────────────────────────────┘
```

- ≥840dp: expanded 220dp rail with icon+label rows.
- 600–839dp: collapsed 96dp rail with labels.
- <600dp: bottom `NavigationBar`, five labelled destinations.
- Top bar actions: Activity, Settings, theme, language; compact overflow menu.
- One large page title in content; the bar does not repeat it.
- Safe areas: FAB/snackbar never cover a primary control. Use `BottomAppBar`
  with a docked FAB or hide the global FAB where an inline primary exists.
- Window minimum: 800×600; clamp initial bounds to the work area.

## Components

Choose the kit component before writing a `div`:

| Need | Component |
| --- | --- |
| Row / settings item | `ListItem` + `ListLeading`/`ListTrailing` |
| Card | `MaterialCard`, `CardGroup` |
| Primary/secondary action | `MaterialButton` (Filled/Tonal/Outlined/Text) |
| Icon action | `IconButton` |
| Create action | `Fab` / `ExtendedFab` |
| Search | `SearchField` |
| Text input | `TextField` |
| One-of-N | `ConnectedButtonGroup` / `MaterialTabs` |
| Long option list | `ExposedDropdown` |
| Numeric | `MaterialSlider` |
| Toggle | `MaterialSwitch` in a `ListItem` |
| Status | `Badge` / `feedback::status_chip` |
| Error | `notice_banner` + Retry |
| Loading | `LoadingIndicator` / `LinearProgress` |
| Dialog | `MaterialDialog` with scrollable body |
| Feedback | `Snackbar` queue, above navigation |
| Identity | `Avatar` |
| Divider | `MaterialDivider` |

Rows are the navigation target; trailing actions are separate targets. Do not
nest a clickable row inside a clickable card.

## Interaction

- One primary action per view; secondary actions tonal/text; destructive
  separated and confirmed with the target named.
- Hover, press, focus, selection and ripple come from the kit.
- Busy commands disable their trigger and show progress; no double submit.
- Errors are inline for fields and persistent banners for page-level failures.
- Preserve section selection, instance detail, tab and scroll when navigating.
- Snackbar queue: at most one visible, placed above the navigation; errors stay
  until dismissed or retried.

## Motion

- M3E spatial/effects springs from the kit.
- Theme and text-scale changes animate through `advance_component_theme`.
- Page transitions are a short fade-through; disabled under reduced motion.
- `ReduceMotion::System` reads the OS setting; explicit On/Off overrides.

## Accessibility

- Text scale 100/125/150/200%; layout reflows and remains operable at 800×600
  and 500×700.
- Focus rings, tab order, Enter/Space activation, Escape dismissal.
- Accessible names for every icon-only control.
- WCAG AA contrast; System theme follows the OS.
- Snackbars announced; errors not time-boxed.
- No mouse-only action.

## Do not

- Do not use a `FilterChip` row as a radio group; use `ConnectedButtonGroup`.
- Do not use a text field + button for search; use `SearchField`.
- Do not use a chip wall for a numeric value; use `MaterialSlider`.
- Do not use a full-width text button as a rail primary action; use `Fab`.
- Do not let the top bar repeat the page H1.
- Do not hard-code colours, radii, spacing or font sizes outside tokens.
