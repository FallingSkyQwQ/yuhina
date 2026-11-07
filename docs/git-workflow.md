# Git Workflow

## Branches

```
main        stable, tagged releases only
develop     integration branch
feat/*      one feature or phase
fix/*       bug fixes
chore/*     tooling, docs
```

- Never commit directly to `main` or `develop`.
- Every phase starts from the integration branch in its own worktree.
- One subagent = one branch = one reviewable diff.
- Merge with `git merge --no-ff` after review.

## Worktrees

```bash
git worktree add ../yuhina-wt/<phase> -b feat/<phase> develop
git worktree remove ../yuhina-wt/<phase>
```

## Commit format

Conventional Commits:

```
feat(ui): use ListItem for instance rows
fix(a11y): apply text scale at startup
docs(design): document the HIG component mapping
chore(brand): embed the Yuhina icon
```

Rules:

- present tense, lower-case subject, ≤72 chars
- body explains why, not what
- one logical change per commit
- no generated build output, no `target/`, no secrets, no machine paths

## Subagent contract

Every subagent must:

1. Work only in its assigned worktree and branch.
2. Read `AGENTS.md`, `docs/product-spec.md`, `docs/design-system.md` and the
   relevant architecture doc first.
3. Keep the diff inside the phase scope.
4. Run `cargo fmt --all`, `cargo clippy --all-targets`, `cargo test`,
   `cargo build`.
5. **Never launch the GUI.** No `cargo run`, `Start-Process`, topmost or
   foreground capture. The user performs manual visual checks.
6. Add or update tests for non-trivial logic.
7. Write `docs/reviews/<phase>.md` with:
   - what changed
   - how to verify
   - HIG/M3E component mapping where relevant
   - screenshots only if the user captured them focus-free
   - known gaps
8. Commit, then report: branch, HEAD SHA, files changed, commands run, open
   questions.
9. Never merge, never push, never touch `main`/`develop`.

## Review checklist

- [ ] scope matches the phase
- [ ] no secrets, no absolute machine paths committed
- [ ] remote-controlled paths validated
- [ ] URLs validated before opening or downloading
- [ ] loading, empty, error and offline states
- [ ] i18n keys for every user-facing string, with placeholders
- [ ] dark + light, 1000×800/800×600/500×700
- [ ] keyboard, focus rings, reduced motion, 200% text
- [ ] one primary action per view
- [ ] the most appropriate kit component is used
- [ ] no AI code
- [ ] no GUI launch / focus theft
- [ ] tests for non-trivial logic
- [ ] review doc complete

## UI review gates

The user reviews UI/UX by running the app themselves:

```bash
cargo run
```

Agents do not launch it. UI-heavy phases do not merge into `main` until the user
approves.

## Documentation

- Living docs: `AGENTS.md`, `docs/product-spec.md`, `docs/design-system.md`,
  `docs/architecture.md`, `docs/git-workflow.md`, `README.md`,
  `README.zh-CN.md`, `design/brand/README.md`.
- Phase reviews: `docs/reviews/<phase>.md` for the current phase only.
- Old phase reviews are removed when the phase closes; the living docs are the
  source of truth.

## Release

- `develop` → `main` only at milestones.
- Tag `v0.x.y`.
- `cargo build --release`; CI packages the binary per platform.
- Release notes are generated from Conventional Commits.
