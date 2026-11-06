# Modpack import, SRV lookup and skin heads

- Branch: `feat/modpack-srv`
- Base: `117ac71` (`fix(launch): stop the game process when the app exits`)
- Date: 2026-11-07

## Scope

| Task | Priority | Status |
| --- | --- | --- |
| `.mrpack` import as a new instance | P0 | Done |
| `_minecraft._tcp` SRV lookup for saved servers | P1 | Done |
| Skin head avatars | P2 | Done |

## 1. `.mrpack` modpack import

### Core

New module `src/core/modpack.rs`:

- `parse_index` / `read_index` parse `modrinth.index.json`:
  `formatVersion`, `game`, `versionId`, `name`, `summary`, `files[]`
  (`path`, `hashes.sha1`, `hashes.sha512`, `env`, `downloads[]`,
  `fileSize`) and `dependencies`.
- `ModpackIndex::validate` rejects:
  - format versions other than `1`;
  - unsafe file paths through `core::paths::normalize_relative`
    (absolute paths, drive prefixes, `..`, NUL bytes);
  - non-HTTP(S) download URLs;
  - files with no download URL or no checksum.
- `select_loader` maps `minecraft`, `fabric-loader`, `quilt-loader`,
  `forge` and `neoforge`. Multiple loaders and missing Minecraft versions
  are `INVALID_INPUT`; Forge and NeoForge return the typed
  `UNSUPPORTED` error with a clear message because their installer
  pipelines live on another branch.
- `plan_files` applies `env.client == "unsupported"` filtering and
  produces normalised game-relative destinations.
- `extract_overrides` extracts `overrides/` first and
  `client-overrides/` second, so a client-specific file wins.
  `server-overrides/` is ignored by this client launcher. Every zip entry
  name is normalised and `Paths::resolve_inside` is used before writing,
  so zip-slip is rejected instead of being silently skipped.
- `import_local` creates the instance, installs the vanilla version and
  the Fabric/Quilt loader through the existing installer, sets
  `launch_version_id`, downloads the pack files with SHA checks, extracts
  overrides, and removes a half-installed instance if anything fails.
- `install_modrinth` resolves the newest version of a Modrinth project,
  downloads its `.mrpack` to `downloads/modpacks/`, and reuses the same
  import path.

`src/core/download.rs` gained an optional `sha512` on `DownloadTask`.
The downloader verifies SHA-1 when present and otherwise SHA-512, so
packs that only publish the latter are still covered. All existing
download call sites keep SHA-1 behaviour.

`src/core/install.rs` gained the `Modpack` and `Overrides` progress
phases used by the import pipeline.

### UI

- The Library page now exposes the `Modpack` filter. Installing with
  `project_type == "modpack"` never writes into `mods/`: it calls
  `install_modpack_project`, creates a new instance, refreshes the list
  and opens the new instance.
- A new `Import .mrpack` action on the Library header opens a path dialog
  (`Dialog::ImportModpack`) that accepts an absolute local `.zip` /
  `.mrpack` path and imports it on the background executor.
- New strings are in `src/ui/i18n.rs` for zh-CN and en-US.

## 2. SRV lookup

`src/core/servers.rs` now:

- builds `_minecraft._tcp.<host>.` with `srv_lookup_name`, skipping IP
  literals;
- performs the lookup only for the default port `25565`; an explicitly
  entered `host:port` is used verbatim;
- selects targets by lowest priority, then highest weight, then a stable
  lexical tie-break, ignoring the RFC 2782 `. ` "service unavailable"
  target;
- falls back to the literal host and port on NXDOMAIN, an empty answer,
  an IP literal, or any resolver failure;
- connects the status ping to the SRV target host/port while writing the
  original host and port into the Minecraft handshake;
- resolves before the one-click join on a background thread. The game
  client performs its own SRV lookup for `--quickPlayMultiplayer`, so the
  original hostname is what must reach the launch arguments.

### DNS dependency justification

Added `hickory-resolver = "0.26"` and a direct `tokio` dependency for its
Tokio-backed resolver.

- `hickory-resolver` is the maintained Hickory DNS stub resolver. It has
  native typed SRV records (`RData::SRV`), follows the platform resolver
  configuration through `system-config`, and avoids a hand-written DNS
  packet/SRV decoder.
- `system-resolver 0.2.6` was considered because it uses the OS resolver
  directly, but it deliberately returns raw RDATA and would require a
  hand-written SRV decoder, which this task explicitly discourages.
- The `srv` crate is version `0.1.0`, from 2016, with old `libc`/`rand`
  dependencies and no maintenance signal.
- `Cargo.lock` adds 19 packages. Most are platform-specific
  (`ipconfig`, `system-configuration`, `jni`, `resolv-conf`) or already
  present transitively (`tokio`, `socket2`, `mio`). Runtime cost is one
  current-thread Tokio runtime per lookup on a background thread.
- DNS failures never fail the server action; they only disable the SRV
  shortcut.

## 3. Skin heads

- `src/core/skins.rs` downloads a Microsoft account's `skin_url` through
  the existing `DownloadTask` path, validates the URL as HTTP(S), crops
  the 8×8 face and hat layer (scale-aware for 128×128 skins), and caches
  a 64×64 PNG under `cache/skins/<account>-<url-hash>.png`.
- `Avatar::image(ImageSource)` in the Material kit renders the cached
  PNG. Until the head is ready, or when an account has no skin URL
  (offline, or a LittleSkin account whose skin is not exposed through the
  profile response), the existing monogram remains.
- The cache key includes a short SHA-1 of the skin URL, so changing a
  skin does not keep showing the old head.
- `image = { version = "0.25", default-features = false,
  features = ["png"] }` is a direct dependency for decoding and cropping.
  The crate is already in the dependency graph through GPUI; enabling
  only PNG keeps the direct addition small. No new image codec is
  compiled in.

## Security review

- Every remote-controlled path (pack files, override entries) passes
  through `normalize_relative` and `Paths::resolve_inside`.
- Pack download URLs and skin URLs must be `http`/`https` with a host.
- No secret reaches the UI: `AccountSummary` carries only `skin_url`,
  which is a public texture URL; tokens remain in the keychain-backed
  record.
- The DNS dependency does not receive credentials or player data; it
  only receives the saved server hostname.

## Tests

No test touches the network.

- `core::modpack`
  - index parsing, file plan and Fabric dependency mapping;
  - Quilt, Forge, NeoForge, vanilla and missing-Minecraft mapping;
  - `..`, absolute and Windows drive path rejection;
  - `env.client == "unsupported"` skipping;
  - override extraction order and `server-overrides` exclusion;
  - missing-Minecraft planning failure.
- `core::servers`
  - SRV name construction for hostnames and IP fallback;
  - default-port-only rule;
  - priority/weight selection and `.` rejection;
  - literal endpoint fallback and handshake-host preservation.
- `core::skins`
  - stable, URL-specific cache names;
  - face crop dimensions and pixels.
- `ui::i18n` key parity still passes for zh-CN and en-US.

## Commands

Run inside the worktree with
`CARGO_TARGET_DIR=C:\Users\youzilm\Documents\projects\yuhina\target`:

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy --all-targets` | pass, no warnings |
| `cargo test` | pass, 26 tests |
| `cargo build` | pass |

## Known gaps

- Forge and NeoForge modpacks return `UNSUPPORTED`; their installers are
  owned by another branch.
- Modpack import has no per-file progress UI; the Library shows an
  importing state and a completion/error toast. Core progress reporting
  is wired for a future Activity entry.
- SRV is only attempted for the implicit port `25565`. Saved records do
  not currently remember whether the player typed a port explicitly.
- Join relies on Minecraft's own SRV support for the actual game
  connection; the launcher's lookup is used for status pings and as a
  pre-join preflight.
- Only Microsoft accounts expose `skin_url`. LittleSkin/authlib skin
  textures would need an additional profile request.
- No UI screenshots were captured in this headless worktree; the layout
  follows the existing Library and Accounts patterns and uses only kit
  components and semantic colour roles.
