# Architecture

## Process model

One Rust process, one GPUI window. There is no renderer/main split, no webview,
no IPC and no Node runtime.

```
┌──────────────────────────────────────────────────────────────┐
│ src/ui            GPUI views, M3E components, HIG layout     │
│                   pages, dialogs, i18n, brand assets         │
└───────────────────────────▲──────────────────────────────────┘
                            │ typed Rust calls
┌───────────────────────────┴──────────────────────────────────┐
│ src/core          headless launcher engine                   │
│  paths · storage · settings · instances · version · install  │
│  loader · launch · java · accounts · skins · servers          │
│  modrinth · modpack · backup · export · files · news · updates│
│  download · http                                             │
└──────────────────────────────────────────────────────────────┘
```

`src/core` has no GPUI dependency and is unit-testable. `src/ui` owns rendering,
commands and scheduling; it never implements protocol details.

## UI architecture

`ui::Yuhina` is the single root entity. It owns:

- `Paths`, `Settings`, language, theme and text scale
- cached instances, accounts, servers, news, versions, Java and update state
- running game processes and log rings
- install tasks and progress
- current page, selected instance, tab, dialogs, scroll and section state
- a snackbar queue

Commands are methods on `Yuhina`. Long-running work uses `spawn_task`:
a blocking closure runs on GPUI's background executor, then the result is applied
on the UI thread. Install progress is shared through `Arc<Mutex<InstallProgress>>`
and polled; game logs are read from an `mpsc::Receiver` in a pump.

The shell is a direct `NavigationRail` + content layout (not a suite content lane)
so the rail's intrinsic width cannot widen the page. The top bar uses
`TopAppBar::actions`; the compact form uses `NavigationBar`. Page width is
left-aligned with a 1120dp max measure and clipped overflow.

Theme and text scale are one source of truth (`APP_THEME`, `APP_CONTRAST`,
`settings.text_scale`). Both startup and animated theme changes use the same
scale.

## Focus and test policy

The app requests focus only when `YUHINA_FOCUS=1`. By default it opens without
activating the window. Agents and tests must never launch the GUI: no
`cargo run`, `Start-Process`, topmost or foreground capture. Manual visual checks
are performed by the user. Automated verification is `cargo test` only; a
focus-free headless renderer may be added later but is not part of the product.

## Data layout

Root: `%APPDATA%\Yuhina` on Windows, `~/Library/Application Support/Yuhina` on
macOS, `~/.config/Yuhina` on Linux.

```
settings.json                 app settings (incl. textScale)
accounts.json                 account metadata (no secrets)
servers.json                  saved servers
instances/<id>/instance.json  instance metadata
instances/<id>/minecraft/     isolated game directory
minecraft/versions/<id>/      version JSON, client jar, natives
minecraft/libraries/          Maven layout
minecraft/assets/             indexes + objects
minecraft/version_manifest.json
java/<major>/                 managed Temurin runtimes
backups/<instanceId>/         zip backups + manifests
exports/                      exported instance zips
logs/instances/<id>/*.log     launch logs
downloads/                    partial files and Java/installer archives
cache/skins/                  cropped skin heads
cache/mojang-news.json        news cache
```

All JSON writes are atomic: sibling temp file, `sync_all`, rename. Every path
derived from remote metadata is normalized and resolved inside a root.

## Security

- `Paths::resolve_inside` / `normalize_relative` reject absolute paths, NUL and
  `..` escapes.
- Remote URLs must be `http`/`https`; Modrinth and skin URLs are checked before
  use.
- Account secrets use `keyring` (Windows Credential Manager / Keychain / Secret
  Service). If the keychain is unavailable, the operation fails with
  `SECRET_STORAGE_UNAVAILABLE`; secrets are never written in plaintext.
- The UI receives `AccountSummary` only: no access or refresh tokens.
- External links use `App::open_url`; local folders use the platform opener.
- No telemetry.

## Minecraft

Version metadata is resolved from the manifest and merged through
`inheritsFrom` chains. Fabric/Quilt profiles are fetched from their meta APIs and
written under `versions/<loader-id>/`.

Forge/NeoForge use the official installer jars:
1. install the vanilla parent;
2. resolve a compatible Java runtime;
3. run `java -jar <installer> --installClient` with `minecraft/` as cwd;
4. detect and persist the produced version profile.

Classpath order: client jar, then libraries in metadata order. Natives are
extracted into the root version's `natives/`.

## Modpacks

`.mrpack` import reads `modrinth.index.json`, validates every path, downloads
files with SHA checks, installs the declared loader (Fabric/Quilt profile or
Forge/NeoForge installer), extracts `overrides/` then `client-overrides/`, and
creates a new instance. Installer output is forwarded to the install task.

## Accounts and skins

Microsoft uses the device-code flow and Xbox Live/XSTS/Minecraft Services chain.
Offline UUIDs are the vanilla MD5 scheme. LittleSkin/authlib uses the
Yggdrasil authenticate endpoint and downloads authlib-injector on demand.

Microsoft skin URLs and LittleSkin profile textures are fetched, the 8×8 head is
cropped and cached under `cache/skins/`. Monograms are the fallback.

## Backups, export and files

Backups zip the game directory with a sidecar manifest. Restore creates a safety
backup first, then replaces files; every entry is validated. Export/import uses a
zip with `yuhina.instance.json`; import assigns a fresh id and cleans up on
failure. The file browser is read-only with breadcrumbs and a 64 KB text preview.

## Updates

The update check reads the public GitHub release API and compares semver. The app
links to the release page; in-app installation is not implemented yet.

## Known gaps

- NeoForge 1.20.1 uses the legacy `net/neoforged/forge` artifact.
- LittleSkin servers with a disabled session endpoint fall back to monograms.
- Installer output is available from core but not yet shown as a full Activity
  log.
- In-app update installation.
- Custom title-bar appearance on Windows (DWM) is in progress.
- Drag-and-drop import, a command palette and tray behavior are not implemented.
- 200% text verification across every page is pending the HIG implementation
  pass.
