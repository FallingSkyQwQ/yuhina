# Phase 1 — Full rewrite to Rust + GPUI + Material 3 Expressive

## Summary

Yuhina was rewritten from the Electron/React/TypeScript launcher into a single
Rust application using GPUI and `gpui-kit-material`.

Per the request:

- The old `.git` directory was deleted and a new repository was initialised.
- GitHub releases and tags `v0.1.0`–`v0.1.3` were deleted.
- The local `release/`, `out/`, `node_modules/` and test artifacts were deleted.
- **All AI/chat/agent/model-provider functionality was removed.** No prompt,
  tool-call, provider, MCP or conversation code remains.
- The UI/UX follows Material 3 Expressive through the shared component kit.

The only legacy copy kept is outside the repository, in the session temp folder,
for feature-porting reference. It is not part of the new tree.

## What changed

### Engine (`src/core`)

A headless, GPUI-free launcher engine:

| Module | Responsibility |
| --- | --- |
| `paths` | data root discovery, safe relative paths, path guards |
| `storage` | atomic JSON writes |
| `settings` | typed settings with tolerant migration |
| `instances` | instance records, auto memory, playtime |
| `version` | manifest/version JSON, rules, library resolution, inheritance merge, BMCL mirror |
| `install` | client/libraries/natives/assets, SHA-1 verification, loader profiles |
| `download` | bounded-concurrency queue, retries, `.part` files, progress |
| `launch` | argument building, placeholders, authlib injection, process spawn, log pump, crash classification |
| `java` | runtime detection, Temurin download and extraction |
| `accounts` | offline, Microsoft device code, LittleSkin/authlib, keychain secrets |
| `servers` | saved servers, address parsing, Java status protocol |
| `modrinth` | search, project/version lookup, dependency-aware install, file removal |
| `news` | Mojang launcher news with cache/fallback |
| `updates` | GitHub release check and semver comparison |
| `http` | `ureq` wrapper with consistent User-Agent/timeouts |

### UI (`src/ui`)

- `NavigationSuite` adaptive navigation: drawer at ≥ 840 px, rail below, bottom
  bar on compact windows.
- `TopAppBar` with logo, page title, theme, language and settings actions.
- Pages: Home, Instances (list + detail with Overview/Logs/Content/Settings),
  Library, Accounts, Servers, Activity, Settings.
- Dialogs: create instance (version/loader/memory), add account (tabbed:
  offline/Microsoft/LittleSkin), add server, delete confirmation, remove
  content, Microsoft device-code display.
- `Snackbar` for success/error feedback.
- Full zh-CN and en-US string tables.
- Light and dark schemes, animated scheme transitions, reduced-motion setting.

### Documentation and CI

- `AGENTS.md`, product spec, architecture, design system and git workflow were
  rewritten for Rust/GPUI/MD3E.
- CI is now `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo build` on
  Windows, Linux and macOS.
- Release workflow builds and archives the Rust binary per platform.

## Feature portability

| Feature | Status |
| --- | --- |
| Instances (list, create, duplicate, delete, detail) | Ported |
| Vanilla install / repair | Ported |
| Fabric / Quilt profile install | Ported |
| Forge / NeoForge | Ported (NeoForge 1.20.2+; 1.20.1 legacy artifact not offered) |
| Launch, classpath, natives, assets, quick-play | Ported |
| Logs and crash classification | Ported (basic classifier) |
| Microsoft device-code login | Ported |
| Offline accounts | Ported |
| LittleSkin / authlib login | Ported (password + authlib-injector) |
| Keychain secret storage | Ported (`keyring`) |
| Java detection | Ported |
| Java auto-download (Temurin) | Ported |
| Download sources (auto/BMCL/official) | Ported |
| Modrinth search/install/dependencies | Ported |
| Resource packs / shaders | Ported |
| Modrinth modpacks (`.mrpack`) | Ported for Fabric/Quilt; Forge/NeoForge return Unsupported |
| Server list, ping, join | Ported with SRV lookup and fallback |
| Mojang news | Ported (10-minute cache + fallback) |
| Update check | Ported (links to the release page; no in-app installer) |
| Settings (theme, language, motion, downloads, Java, memory, about) | Ported |
| Backups / restore | Ported (zip + safety copy on restore) |
| Export / import | Ported (instance zip) |
| Per-instance file browser | Ported (read-only, 64 KB text preview) |
| Skin-head rendering | Ported for Microsoft accounts; monograms elsewhere |
| Drag-and-drop import | Not yet |
| Command palette | Not yet |
| Tray / close-to-tray | Not yet |

## Security

- Renderer/main split no longer exists; account tokens live only in the engine
  and the OS keychain. The UI receives `AccountSummary` without secrets.
- Every remote-controlled path is normalised and resolved inside a root.
- Modrinth download URLs are checked for `http(s)` before use.
- External links use `App::open_url`; local folders use the platform opener.
- No telemetry, no remote code, no AI endpoints.

## Dependencies added

- `gpui-kit`, `gpui-kit-material` — pinned git revisions required by the kit.
- `ureq` — small blocking HTTP client (no Tokio runtime in the UI).
- `keyring` — OS credential storage, replacing Electron `safeStorage`.
- `zip`, `tar`, `flate2` — native jars and Java runtimes.
- `sha1`, `sha2`, `md-5`, `hex` — checksums and offline UUIDs.
- `chrono`, `uuid`, `dirs`, `url`, `regex`, `percent-encoding`, `serde`,
  `serde_json`, `base64` — standard data handling.

`anyhow`, `thiserror` and `image` were removed after the first build because the
final code does not use them.

## Verification

```bash
cargo fmt --all --check        # clean
cargo clippy --all-targets     # no warnings
cargo test                     # 42 passed
cargo build                    # debug binary built
```

The binary was launched on Windows 11 at 125% DPI and captured for review.
Screenshots live in `design/reviews/rewrite-m3e/`:

- `home-native.png`, `home-light.png`
- `instances-native.png`, `instances-light.png`, `instances-rail.png`
- `library-native.png`
- `accounts-native.png`
- `servers-native.png`
- `activity-native.png`
- `settings-native-dark.png`, `settings-native-light.png`
- `dialog-create-instance.png`, `dialog-add-account.png`
- `dialog-create-forge.png`, `instance-backups.png`, `instance-files.png`

### Screenshot notes

- The system title bar is native; its colour follows the OS, not the app theme.
  This is the remaining visual mismatch in the light theme.
- The drawer at 900 px physical still shows labels because GPUI's size class
  uses the window's pixel width; the rail/bar switch is available but was not
  captured in this environment.
- The empty state, dialogs, light theme, and network-backed version list and
  Modrinth results were verified.

## Known gaps

1. NeoForge 1.20.1 uses the legacy `net/neoforged/forge` artifact and is not
   offered.
2. Forge/NeoForge modpacks return a typed unsupported error.
3. Modpack import shows an importing state, not per-file Activity progress.
4. LittleSkin/authlib skin textures are not fetched (monograms fallback).
5. Installer output is surfaced on failure, not streamed into Activity.
6. Drag-and-drop import and a command palette.
7. In-app update installation (the release page opens in the browser).
8. Custom title bar to theme the caption area with the app.
9. The Logs tab only shows live logs for a game started in this session; the
   last log file is not reloaded after restart.

## Git

- Branch: `feat/rewrite-m3e`.
- Rewrite commit: `b9682ae90b383e256875ee39db5099eb49556e2a`; Forge/NeoForge,
  backups/export/file browser and modpack/SRV/skin branches were merged
  afterward.
- The old `.git` history, tags `v0.1.0`–`v0.1.3`, GitHub releases and local
  release artifacts were deleted.
- Remote `main` and `develop` were force-updated to the rewrite commit, so the
  previous history is no longer reachable from those branches.
- No backup was kept.
