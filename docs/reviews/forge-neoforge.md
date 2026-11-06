# Forge and NeoForge support

## What changed

- `src/core/loader.rs` (new)
  - Forge version discovery from
    `promotions_slim.json`: recommended build first, latest second.
  - NeoForge version discovery from `maven-metadata.xml`, filtered by the
    Minecraft series (`1.21.1` → `21.1`, `1.21` → `21.0`, `1.20.6` → `20.6`).
    Stable builds are preferred and the newest eight are offered.
  - Official installer URLs for Forge and NeoForge Maven.
  - Installer command construction: `java -jar <installer> --installClient`.
  - Installer Java selection: at least Java 17 for modern Forge, at least
    Java 21 for NeoForge 21.x, otherwise the game version's Java major.
  - Download into `downloads/`, run with `minecraft/` as the working directory,
    capture stdout/stderr, and return a typed `YuhinaError` on failure.
  - Produced version-id detection (`1.20.1-forge-47.3.0`,
    `neoforge-21.1.209`, plus a directory scan fallback).
- `src/core/install.rs`
  - New `InstallPhase::Loader` progress phase and `install_loader` dispatcher.
  - Fabric and Quilt keep their existing profile pipeline; Forge and NeoForge
    run the installer.
- `src/core/instances.rs`
  - `set_launch_version_id` persistence helper.
- `src/ui/mod.rs`
  - Loader-version discovery state, refresh logic, and Java settings passed to
    the background install.
  - The produced loader profile is persisted as `launchVersionId`.
- `src/ui/dialogs.rs`
  - Forge and NeoForge chips are enabled.
  - A loader-version chip row appears for both loaders with loading, empty and
    error states plus a retry action.
  - Changing the Minecraft version reloads the loader versions.
- `src/ui/i18n.rs`
  - New keys in zh-CN and en-US.
- `docs/product-spec.md`, `docs/architecture.md`
  - Forge/NeoForge are no longer described as disabled or a known gap.

No dependencies were added.

## How to run / verify

```bash
cargo fmt --all --check
cargo clippy --all-targets
cargo test
cargo build
cargo run
```

Manual check:

1. Open **New instance**.
2. Pick a release version, for example `1.20.1`.
3. Pick **Forge**; the loader-version row shows the recommended and latest
   builds. Pick a build and confirm.
4. The instance installs vanilla, runs the Forge installer, and the instance
   becomes installed with `launchVersionId = 1.20.1-forge-...`.
5. Repeat with `1.21.1` and **NeoForge**; expect `neoforge-21.1.x`.
6. Launch the instance and confirm the game reaches the main menu.
7. Repeat with Fabric, Quilt and Vanilla to confirm no regression.
8. Check the dialog in dark and light themes, at 1440×900 and 1000×800.

Offline behaviour: loader discovery shows the network error with a retry
button; install fails with a toast and leaves the instance marked not
installed.

## Tests

All tests are network-free:

- Forge promotion parsing and recommended/latest ordering.
- NeoForge metadata parsing, series filtering and stable-first ordering.
- Minecraft-series prefix mapping.
- Installer URL construction and headless argument construction.
- Version-id candidates and produced-version detection (temporary directory).
- `set_launch_version_id` persistence (temporary directory).

## Known limitations

- NeoForge 1.20.1 is not offered: its builds live under the legacy
  `net/neoforged/forge` Maven artifact and use the Forge version scheme. The
  dialog supports NeoForge 1.20.2+ (series `20.2` and newer).
- Forge and NeoForge installers are not sandboxed. They write into the shared
  `minecraft/versions` and `minecraft/libraries` directories, exactly like the
  official launcher. The vanilla version is installed first so the installer
  has a parent to inherit from.
- Installer output is captured and surfaced on failure, but it is not streamed
  line-by-line to the Activity log. The UI shows the loader phase and installer
  status text while it runs.
- Installer Java requirements depend on the loader build. Yuhina picks a safe
  minimum and downloads Temurin when needed; very old Forge builds may still
  require a specific Java 8 vendor.
- Screenshots were not captured in this headless worktree. Run `cargo run` and
  use the manual check above; add screenshots under
  `design/reviews/forge-neoforge/` when reviewing the UI.
