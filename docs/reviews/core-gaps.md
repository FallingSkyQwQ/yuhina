# Core gaps: Forge modpacks, LittleSkin skins, text scale

- Branch: `feat/core-gaps`
- Base: `e786039` (`fix(ui): harden shell widths and action placement`)
- Date: 2026-11-07

## Scope

| Task | Priority | Status |
| --- | --- | --- |
| Forge/NeoForge `.mrpack` import | P0 | Done |
| LittleSkin/authlib skin textures | P1 | Done |
| `text_scale` setting | P1 | Done |
| Stream installer output | P2 | Done in core; UI wiring left to the UI agent |

No UI files, `src/main.rs`, `build.rs` or brand assets were touched.

## 1. Forge and NeoForge modpacks

`src/core/modpack.rs`

- `select_loader` now accepts `dependencies.forge` and
  `dependencies.neoforge` instead of returning `UNSUPPORTED`.
- Forge and NeoForge versions are validated with `paths::assert_safe_id`
  before the vanilla install starts, because the value becomes a Maven
  artefact segment and a version-directory id. Unsafe values return
  `INVALID_INPUT` without creating an instance.
- An empty `forge`/`neoforge` value is treated as "no version"; the installer
  pipeline resolves the newest compatible build.
- `import_into` now installs vanilla, then calls
  `install::install_loader_with_output` for every non-vanilla loader. Fabric
  and Quilt keep the existing profile pipeline; Forge and NeoForge run their
  official installer. `launch_version_id` is set to the id the loader produced.
- New additive entry points:
  - `import_local_with_java(paths, zip_path, source, java_settings, cancel, on_progress)`
  - `install_modrinth_with_java(paths, project_id, version_id, source, java_settings, cancel, on_progress)`
  - The existing `import_local` / `install_modrinth` signatures are unchanged
    and delegate with `JavaSettings::default()`.

`src/core/install.rs`

- `install_loader` delegates to the new
  `install_loader_with_output(paths, request, cancel, on_progress, on_output)`.
- Missing Forge/NeoForge versions are resolved through `loader::discover`
  (Forge promotions / NeoForge Maven metadata) instead of failing with
  "choose a version first".
- `LoaderInstallRequest` is unchanged, so existing UI construction still
  compiles.

`src/core/loader.rs`

- `run_installer` now spawns the process and reads stdout and stderr on two
  scoped threads while `wait()` runs, so a chatty installer cannot deadlock on
  a full pipe.
- `install_with_output` forwards every non-empty line to a caller callback.
  Lines are trimmed and capped at 512 characters; the last 16 KiB is retained
  for the failure message. `install` keeps its old signature and delegates
  with a no-op output callback.
- If the installer fails, the typed error still carries the captured tail
  (1500 characters).

## 2. LittleSkin / authlib skin textures

`src/core/accounts.rs`

- `fetch_authlib_skin_url(base, uuid)` requests
  `<authlib_url>/sessionserver/session/minecraft/profile/<compact-uuid>` and
  reads the `textures` property.
- `decode_skin_textures(value)` decodes the base64 property and returns
  `textures.SKIN.url`; missing skins return `Ok(None)`.
- `validate_skin_texture_url` rejects anything that is not an absolute
  HTTP(S) URL with a host (`file:`, `javascript:`, `data:` and relative paths
  are rejected).
- Malformed base64 or JSON returns `YuhinaError(Parse)`; an unsafe URL returns
  `YuhinaError(InvalidInput)`.
- A non-success profile response (including 404/204) is treated as "no skin":
  some authlib servers disable the public session endpoint, and a cosmetic
  skin must not block an otherwise valid login.
- `add_authlib` stores the fetched URL in `AccountRecord.skin_url`.
- New `refresh_authlib(paths, account_id)` refreshes the access token through
  `<authlib_url>/authserver/refresh`, updates name/uuid from
  `selectedProfile`, re-fetches the skin and persists the new token in the
  keychain. Tokens are never logged and never added to a summary.

`src/core/skins.rs`

- `ensure_head` already downloads through `DownloadTask`, validates
  HTTP(S), crops the face/hat and caches a PNG. LittleSkin URLs now flow
  through the same path because `AccountSummary.skin_url` is populated.
- Added `ensure` as an alias for `ensure_head` so callers can treat every
  account kind uniformly.

## 3. Text scale setting

`src/core/settings.rs`

- New `Settings::text_scale: f32` (`textScale` in JSON).
- `TEXT_SCALE_DEFAULT = 1.0`, `TEXT_SCALE_MIN = 0.8`, `TEXT_SCALE_MAX = 2.0`.
- The field uses `#[serde(default = "default_text_scale")]`, so settings files
  written before this branch load with `1.0`.
- `clamp_text_scale` clamps to the supported range and maps `NaN`/infinities
  to the default, so a corrupt file cannot break layout.
- `settings::load` clamps after reading; `settings::save` clamps before
  writing. In-memory callers can use `clamp_text_scale` directly.
- The UI was intentionally not wired.

## Tests

All tests are network-free.

- `core::modpack`
  - dependency mapping for vanilla, Fabric, Quilt, Forge and NeoForge;
  - empty Forge version → `None`;
  - unsafe Forge/NeoForge versions rejected before install;
  - two loaders rejected;
  - a failed import deletes the half-installed instance (seeded local version
    and a zip-slip override, no downloads).
- `core::accounts`
  - base64 textures decoding, missing skin, malformed base64/JSON;
  - `file:`, `javascript:`, `data:` and relative texture URLs rejected;
  - compact UUID validation.
- `core::skins`
  - HTTP(S)-only skin URL validation (including LittleSkin URLs).
- `core::loader`
  - stdout/stderr line streaming, trimming, blank-line skipping and
    over-long-line truncation.
- `core::settings`
  - default `1.0`, clamping and non-finite handling;
  - a pre-existing settings JSON without `textScale` loads with the default;
  - load/save clamping through the real settings file.

## API notes for the UI agent

- **Text scale:** read `settings.text_scale` and pass it to
  `init_component_theme` / the component theme scale. Persist changes with
  `settings::save`; it clamps automatically. `settings::clamp_text_scale` is
  public for live slider previews. This is a new field on a public struct, so
  any `Settings { .. }` literal in UI code must add `text_scale` or use
  `..Settings::default()`.
- **Modpack Java settings:** switch the Library import calls from
  `modpack::import_local` / `modpack::install_modrinth` to the `_with_java`
  variants if the import should honour the user's Java path/auto-download
  settings. The old functions still compile and use defaults.
- **Installer output:** `install::install_loader_with_output` and
  `loader::install_with_output` accept an extra `Fn(&str)` callback. Modpack
  imports already forward those lines through
  `InstallProgress.message` while `phase == Loader`; wire that to the
  Activity log. `install_loader` and `loader::install` are unchanged for
  existing callers.
- **LittleSkin refresh:** `accounts::refresh_authlib(paths, account_id)` is
  available but not called by the UI yet. Call it before launching a
  LittleSkin account (or on demand) when the session may be stale. A successful
  refresh updates `skin_url`, so UI code that reloads `accounts::list` will
  pick up a changed skin.
- **Skin heads:** the UI can keep calling `skins::ensure_head`; the
  `skins::ensure` alias is also available. Both take any HTTP(S) texture URL.
- **Error strings:** the new core errors are English like the rest of
  `src/core`; the UI maps error codes to localized toasts as before.

## Commands

Run inside this worktree with
`CARGO_TARGET_DIR=C:\Users\youzilm\Documents\projects\yuhina\target`:

| Command | Result |
| --- | --- |
| `cargo fmt --all` | pass |
| `cargo fmt --all --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, no warnings |
| `cargo test` | pass, 56 tests |
| `cargo build` | pass |

## Remaining gaps

- The UI does not yet pass `JavaSettings` to modpack imports (it uses the
  default) and does not call `refresh_authlib`.
- Installer stdout/stderr reaches the core callback and `InstallProgress`
  messages, but the Activity log does not render them yet.
- A LittleSkin server that disables its session endpoint yields no skin
  texture; the monogram fallback remains.
- NeoForge 1.20.1 still uses the legacy `net/neoforged/forge` artefact and is
  not offered by the version picker.
- Forge/NeoForge installs are exercised only through unit tests for URL,
  detection and streaming logic. No installer is downloaded in tests or while
  producing this branch.
- No UI screenshots: this worktree is core-only and the app was not launched.
