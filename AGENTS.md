# AGENTS.md

You are working on Yuhina 2, a Minecraft launcher written in Rust with GPUI and
the Material 3 Expressive component kit.

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
- No AI, chat, agent, MCP, prompt or model-provider code lives in this
  application. Do not reintroduce it.
- Secrets never reach the UI layer. Account tokens live in the OS keychain
  through `keyring`; the UI only ever receives summaries.
- Every filesystem path derived from remote metadata goes through
  `core::paths::normalize_relative` or `Paths::resolve_inside`.
- Every outbound URL is `http`/`https` and is validated before use. Never open a
  `file:` or `javascript:` URL from remote data (local folders use the platform
  opener directly).
- All user-facing strings go through `src/ui/i18n.rs` (zh-CN + en-US).
- Dark and light themes must both work. Respect reduced motion.
- Do not add a dependency without justifying it in the review doc.

## UI rules

- Material 3 Expressive: tonal surfaces, 24–32dp radii on large surfaces,
  expressive type, generous spacing, state layers and ripples from the kit.
- Use `NavigationSuite` for adaptive navigation (bar → rail → drawer).
- Use `TopAppBar`, `MaterialButton`, `IconButton`, `TextField`, `FilterChip`,
  `MaterialSwitch`, `MaterialDialog`, `Snackbar`, `Avatar`, `LinearProgress`
  and `ListItem` rather than bespoke controls.
- Icons come from Material Symbols through `icons::Symbol`. No emoji as icons.
- Test at 1440×900 and 1000×800; the app must remain usable at 800×600.
- No gradients, glassmorphism or neon. No Minecraft-green accent.

## Code style

- Small modules: one responsibility per file. Keep files under ~600 lines where
  practical; `src/ui/pages.rs` is allowed to grow because it is a page router.
- Typed domain errors (`core::YuhinaError`) and user-readable messages. No silent
  catches; background tasks must surface failures as a toast.
- Blocking work (network, filesystem, hashing) runs on
  `cx.background_executor()`. Never block the render thread.
- Prefer explicit `px(...)` spacing and the `ColorRole` palette over magic
  values.

## Commands

```bash
cargo run
cargo fmt --all
cargo clippy --all-targets
cargo test
cargo build --release
```

## Git

- Work only on your assigned branch/worktree.
- Conventional Commits.
- Do not merge, push, or touch `main`/`develop`.
- Write `docs/reviews/<phase>.md` and screenshots in `design/reviews/<phase>/`.
- Report HEAD SHA, commands run, and known gaps when done.

## Definition of done

- `cargo fmt --all --check`, `cargo clippy --all-targets`, `cargo test` pass.
- Loading, empty, error and offline states covered.
- Keyboard focus, reduced motion and light theme checked.
- No secrets or absolute machine paths committed.
- Review doc complete.
