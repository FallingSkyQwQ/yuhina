# Backups, export/import and file browser

Branch: `feat/backups-export` (worktree `feat-backups-export`)

## What changed

### Core

- `src/core/backup.rs`
  - `create` archives the instance's `minecraft/` directory into
    `<root>/backups/<instanceId>/<timestamp>.zip` and writes a sidecar
    `<timestamp>.json` manifest (`id`, `instanceId`, `label`, `createdAt`,
    `bytes`, `entries`).
  - `list` reads manifests newest-first and ignores entries whose archive is
    missing or whose manifest belongs to another instance.
  - `restore` validates every archive entry through `normalize_relative`
    before touching disk, creates a `before-restore` safety backup, then
    replaces the game directory contents.
  - `delete` removes both the archive and its manifest.
  - All work reports through `InstallProgress` with the new
    `Backup`/`Restore` phases.
- `src/core/export.rs`
  - `export_instance` writes
    `<root>/exports/<sanitized-name>-<timestamp>.zip` containing
    `yuhina.instance.json` and every game file under `minecraft/`.
  - `import` validates entry names first, requires the manifest, rejects
    unexpected files, creates a fresh UUID instance, copies game files, and
    removes the partial instance if extraction fails.
  - Imported metadata is normalised: memory is clamped, the fresh id and
    creation time are applied, playtime/last-played are reset and the
    machine-specific `javaPath` is dropped.
  - Progress uses the `Export` and `Import` installer phases.
- `src/core/files.rs`
  - Read-only instance file listing rooted at the game directory, with
    directories first and symlinks skipped.
  - `read_text_preview` caps previews at 64 KB, rejects NUL/binary and
    non-UTF-8 files, and reports truncation.
  - All relative paths go through `normalize_relative` and every resolved
    path is checked with `Paths::resolve_inside` / `is_inside`.
- `src/core/paths.rs`: added `backups_dir`, `instance_backups_dir` and
  `exports_dir`.
- `src/core/install.rs`: added `Backup`, `Restore`, `Export` and `Import`
  install phases with labels and progress ranges.
- `src/core/mod.rs`: registered `backup`, `export` and `files`.

### UI

- `InstanceTab::Backups` and `InstanceTab::Files` added to the instance tab
  strip. Switching tabs lazily refreshes the relevant data.
- Backups tab: optional label field, create action, list with local date,
  size and file count, restore/delete confirmations. Creating or restoring is
  blocked while the game is running.
- Files tab: breadcrumb navigation from the game-directory root, folder/file
  rows using `ListItem`, file sizes, and a read-only monospace preview with a
  truncated marker.
- Export action (icon button with accessible label) on the instance detail
  header; Import action on the Instances page header with a path `TextField`
  dialog. No native file dialog dependency.
- All new strings are in `src/ui/i18n.rs` for zh-CN and en-US. Colours use
  semantic `ColorRole`s; no new animation or hard-coded colours.

### Docs

- Removed the now-implemented file-browser/backup gap from
  `docs/architecture.md` and mentioned backups/export/files in
  `docs/product-spec.md`.

## Tests

Added seven network-free tests using temporary directories:

- `core::backup::tests::backup_round_trip_restores_files` — zip round-trip,
  safety backup, list and delete.
- `core::backup::tests::restore_rejects_traversal_entries` — `../` entry is
  rejected and nothing escapes.
- `core::backup::tests::missing_backups_list_as_empty`.
- `core::export::tests::export_import_round_trip_copies_metadata_and_files`.
- `core::export::tests::import_rejects_traversal_and_foreign_files`.
- `core::files::tests::listing_rejects_traversal_and_orders_directories_first`.
- `core::files::tests::previews_are_text_only_and_bounded`.

## Commands run

All from the `feat-backups-export` worktree with
`CARGO_TARGET_DIR=C:\Users\youzilm\Documents\projects\yuhina\target`:

```text
cargo fmt --all                          ok
cargo test                               ok (21 passed)
cargo clippy --all-targets               ok
cargo build                              ok
cargo check                              ok
```

## How to verify

1. `cargo run`.
2. Open an instance → **Backups**, type a label, create a backup. Confirm the
   row shows date, size and file count.
3. Change or delete a file under the instance game directory (or use the
   **Files** tab), restore the backup, confirm the confirmation dialog, and
   check that the original file is back and a `before-restore` backup appears.
4. Delete a backup and confirm it disappears from the list.
5. Open **Files**, navigate into `mods/` and back via the breadcrumbs, and
   click a text file such as `options.txt`. The preview is read-only.
6. From the instance detail header, export the instance. The toast names the
   created zip under `<root>/exports/`.
7. On the Instances page, choose **Import instance**, paste the export path,
   confirm, and check that a new instance with the same files opens.
8. Repeat with the light theme and at 1000×800 / 800×600 widths.

## Known gaps

- No screenshots were captured in this headless subagent session; UI was
  reviewed by compilation and by reading the existing component APIs. The
  review folder `design/reviews/backups-export/` is therefore empty.
- Archive extraction has no explicit zip-bomb size/count limit beyond the
  filesystem; imports are user-initiated local files.
- Export/import currently preserves `jvmArgs` and `gameArgs`; only
  `javaPath` is dropped as machine-specific.
- Backups are local only (no cloud sync), matching the product non-goals.
- Imported instances keep any game files present in the archive, but the
  global version/libraries/runtime are not bundled; the user still runs
  Install/Repair if the version is not present.
- UI file preview treats files larger than 64 KB as truncated rather than
  offering paging.
- Backup/export/import report `InstallProgress` from core, but the UI only
  disables the action and shows a completion toast; it does not yet render a
  live progress bar for these operations.
