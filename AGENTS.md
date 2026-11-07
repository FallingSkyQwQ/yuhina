# AGENTS.md

Yuhina is a Minecraft: Java Edition launcher written in Rust with GPUI and the
Material 3 Expressive component kit. The visual identity is M3E; Apple HIG is
the quality bar for hierarchy, spacing, interaction and accessibility.

## Before you touch code

Read:

1. `docs/product-spec.md`
2. `docs/design-system.md`
3. `docs/architecture.md`
4. `docs/git-workflow.md`

## Non-negotiables

- Rust 2024 edition, `rust-version = "1.99.1"`. No `unsafe` without a documented
  reason and a safety comment.
- The UI is GPUI only. Use `gpui-kit-material` components and semantic
  `ColorRole`s. Never hard-code a colour that a role already provides.
- **No AI.** No chat, agent, MCP, prompt, model-provider or hidden AI networking
  code lives in this application.
- Secrets never reach the UI layer. Account tokens live in the OS keychain via
  `keyring`; the UI receives summaries only.
- Every filesystem path derived from remote metadata goes through
  `core::paths::normalize_relative` or `Paths::resolve_inside`.
- Every outbound URL is `http`/`https` and validated before use. Never open
  `file:` or `javascript:` from remote data; local folders use the platform
  opener.
- All user-facing strings go through `src/ui/i18n.rs` (zh-CN + en-US), with
  placeholders rather than fragment concatenation.
- Dark and light themes must both work, including the window chrome.
- Support text scaling to at least **200%** without clipping or overlapping
  controls.
- Respect reduced motion, including the OS `System` setting.
- **Never launch the GUI from an agent or test command.** No `cargo run`,
  `Start-Process`, `SetForegroundWindow` or topmost capture. The user has
  reported focus theft. Build/test only; manual visual checks are done by the
  user. A future headless screenshot harness may be added, but only if it is
  focus-free by construction.

## UI rules

- Keep the Material 3 Expressive identity: tonal surfaces, the Expressive
  colour scheme, 24–32dp large surfaces, expressive type and springs.
- Apply Apple HIG for hierarchy, restraint, the 8pt spacing rhythm, one primary
  action per view, direct manipulation, clear feedback, safe areas and
  platform-consistent interaction.
- Use the kit component that fits the job:
  - rows and settings: `ListItem` (+ `ListLeading` / `ListTrailing`)
  - cards: `MaterialCard`, `CardGroup`
  - search: `SearchField`
  - one-of-N: `ConnectedButtonGroup` / `MaterialTabs`
  - long option lists: `ExposedDropdown`
  - numeric values: `MaterialSlider`
  - status: `Badge` / `feedback::status_chip`
  - errors: `notice_banner` + Retry
  - loading: `LoadingIndicator` / `LinearProgress`
  - dialogs: `MaterialDialog` with a scrollable body
  - feedback: `Snackbar` with a queue, above the navigation
- Do not build a bespoke `div` row/card/menu/segmented control when a kit
  component exists. If an exception is necessary, document why in the review.
- Navigation: adaptive rail → bar. Expanded rail 220dp at ≥840, collapsed 96dp
  at 600–839, bottom bar below 600. Five primary destinations; secondary
  destinations live in the top bar/overflow.
- Brand: use `design/brand/yuhina-mark.svg` (currentColor) in the app and the
  generated `resources/icons/yuhina.ico` for the executable. No text or emoji
  substitutes.
- No gradients, glassmorphism, neon, or Minecraft-green accent outside the
  Expressive scheme. No emoji as icons.
- Test at 1440×900, 1000×800, 800×600 and 500×700; both themes and languages;
  100% and 200% text.

## Code style

- Small modules; one responsibility per file where practical.
- Typed domain errors (`core::YuhinaError`); user-readable messages; no silent
  catches.
- Blocking work runs on `cx.background_executor()`. Never block the render
  thread.
- Use `rem(...)` for text-coupled geometry and `px(...)` for structural sizes.
- Prefer the kit's type roles and spacing/radius tokens over literals.

## Commands

```bash
cargo fmt --all
cargo clippy --all-targets
cargo test
cargo build
```

Never run `cargo run` or start the binary in an agent session.

## Git

- Work only on your assigned branch/worktree.
- Conventional Commits; present tense, ≤72 chars.
- Do not merge, push, or touch `main`/`develop` unless the coordinator asks.
- Write `docs/reviews/<phase>.md` with what changed, verification, screenshots
  (only if captured focus-free by the user), and known gaps.
- Report HEAD SHA, commands run, and known gaps when done.

## Definition of done

- `cargo fmt --all --check`, `cargo clippy --all-targets`, `cargo test`,
  `cargo build` pass.
- Loading, empty, error and offline states covered.
- Keyboard focus, reduced motion, light theme and 200% text checked.
- No secrets or absolute machine paths committed.
- No AI code, no GUI launches, no focus theft.
- Review doc complete.
