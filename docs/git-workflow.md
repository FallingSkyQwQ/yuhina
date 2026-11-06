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
- Every phase starts from `develop` in its own worktree.
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
feat(ui): add expressive home cards
fix(java): prefer exact major version
docs(arch): document the installer pipeline
chore(deps): pin gpui-kit-material revision
```

Rules:

- present tense, lower-case subject, <= 72 chars
- body explains why, not what
- one logical change per commit
- no generated files, no `target/`, no secrets, no machine-specific paths

## Subagent contract

Every subagent must:

1. Work only in its assigned worktree and branch.
2. Read `AGENTS.md`, `docs/product-spec.md`, `docs/design-system.md` and the
   relevant architecture doc first.
3. Keep the diff inside the phase scope.
4. Run `cargo fmt --all`, `cargo clippy --all-targets`, and `cargo test`.
5. Add or update tests for non-trivial logic.
6. Write `docs/reviews/<phase>.md` with:
   - what changed
   - how to run/verify
   - screenshots (UI changes) under `design/reviews/<phase>/`
   - known gaps
7. Commit, then report: branch, HEAD SHA, files changed, commands run, open
   questions.
8. Never merge, never push, never touch `main`/`develop`.

## Review checklist

- [ ] scope matches the phase
- [ ] no secrets, no absolute machine paths committed
- [ ] remote-controlled paths validated
- [ ] URLs validated before opening or downloading
- [ ] loading, empty, error and offline states
- [ ] i18n keys for every user-facing string
- [ ] dark + light, 1440×900 and 1000×800
- [ ] keyboard and reduced motion
- [ ] tests for non-trivial logic
- [ ] review doc complete

## UI review gates

The user reviews UI/UX by running:

```bash
cargo run
```

UI-heavy phases do not merge into `main` until the user approves.

## Release

- `develop` → `main` only at milestones.
- Tag `v0.x.y`.
- `cargo build --release` produces the binary; CI packages it per platform.
- Release notes are generated from Conventional Commits.
