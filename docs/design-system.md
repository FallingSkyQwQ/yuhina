# Design System — Material 3 Expressive

Yuhina uses the Material 3 Expressive tokens and components from
`gpui-kit-material`. The palette is the `ThemeId::Expressive` scheme, dark by
default.

## Colour

Paint with semantic roles, never stored colours:

```rust
div().bg(color(ColorRole::SURFACE_CONTAINER_LOW))
```

Every role has a light and dark value supplied by the scheme. Rules:

- Page ground: `SURFACE`
- Navigation and top bar: `SURFACE` / `SURFACE_CONTAINER_LOW`
- Cards and panels: `SURFACE_CONTAINER_LOW` or `SURFACE_CONTAINER`
- Primary action: `PRIMARY` on `ON_PRIMARY`
- Secondary surfaces: `SECONDARY_CONTAINER` on `ON_SECONDARY_CONTAINER`
- Tertiary highlights (instance/game marks): `TERTIARY_CONTAINER` on
  `ON_TERTIARY_CONTAINER`
- Errors: `ERROR_CONTAINER` on `ON_ERROR_CONTAINER`
- Supporting text: `ON_SURFACE_VARIANT`
- Dividers: `OUTLINE_VARIANT`

Do not introduce a Minecraft-green accent. The scheme supplies the accent.

## Typography

- Family: Roboto (system fallback), CJK through the platform fallback.
- Page title: 32/40, medium
- Section label: 13/18, medium, `ON_SURFACE_VARIANT`
- Body: 14/20
- Supporting: 13/18
- Hero headline: 36/44, medium
- Code and logs: 12/18 monospace

Text scale follows the kit's rem-based typography. The layout must tolerate a
200% text scale without clipping controls.

## Shape

- Large hero surfaces: 32dp
- Cards and panels: 24dp
- Rows and list items: 20dp
- Control groups: 14–16dp
- Chips and status pills: full (999dp)
- Buttons and icon buttons: from the kit (`ButtonShape`)

Do not round everything to the same radius. The radius communicates hierarchy.

## Spacing

- Page gutter: 32dp
- Content max width: 1120dp
- Section gap: 24dp
- Card gap: 16dp
- Row gap: 10dp
- Inside a panel: 20dp

## Motion

- The kit's Expressive scheme provides spatial and effects springs.
- Theme changes animate through `advance_component_theme`.
- Ripples and state layers come from the components; do not hand-roll them.
- Reduced motion: transforms are disabled, opacity remains. The setting is
  applied at startup with `cx.set_reduce_motion`.

## Components

| Need | Component |
| --- | --- |
| Navigation | `NavigationSuite` (bar/rail/drawer) |
| Page header | `TopAppBar` |
| Actions | `MaterialButton`, `IconButton`, `Fab` |
| Text input | `TextField` |
| Toggles | `MaterialSwitch`, `FilterChip` |
| Tabs | `MaterialTabs` |
| Dialogs | `MaterialDialog` |
| Feedback | `Snackbar`, `LinearProgress`, `CircularProgress` |
| Identity | `Avatar` |
| Lists | `ListItem`, `MaterialCard` |

Icons are Material Symbols resolved through `icons::Symbol`; the code maps a
missing symbol to `SETTINGS` rather than panicking.

## Layout

```
┌──────────────────────────────────────────────────────┐
│ TopAppBar: logo · page title · theme · language · gear│
├──────────────┬───────────────────────────────────────┤
│ Navigation   │ Page content                          │
│ (drawer/rail)│ max 1120dp, 32dp gutter               │
│  New instance│                                       │
└──────────────┴───────────────────────────────────────┘
```

- At ≥ 840dp the suite shows a drawer with labels.
- At 600–839dp it shows a collapsed rail.
- Below 600dp it shows a bottom navigation bar.
- The content column is centred and scrolls independently.

## Do not

- No gradients, glassmorphism, neon or purple-on-black AI styling.
- No emoji as icons.
- No centred body text outside empty states and dialogs.
- No hover layout shift; reserve space or use overlays.
- No hard-coded hex colours in page code.
