# Architecture

## Layers

```
┌──────────────────────────────────────────────────────────────┐
│ src/ui            GPUI views, Material 3 Expressive shell    │
│                   pages, dialogs, i18n, background tasks     │
└───────────────────────────▲──────────────────────────────────┘
                            │ typed Rust calls
┌───────────────────────────┴──────────────────────────────────┐
│ src/core          headless launcher engine                   │
│  paths · storage · settings · instances · version · install  │
│  launch · java · accounts · servers · modrinth · news        │
│  updates · download · http                                   │
└──────────────────────────────────────────────────────────────┘
```

`src/core` has no GPUI dependency and is directly unit-testable. `src/ui` owns
all rendering and scheduling; it never implements protocol details.

## Process model

One GPUI process, one window. There is no renderer/main split, no IPC, no
webview and no Node runtime. Blocking work runs on GPUI's background executor;
the render thread only builds elements.

## Data layout

Root: `%APPDATA%\Yuhina` on Windows, `~/Library/Application Support/Yuhina` on
macOS, `~/.config/Yuhina` on Linux.

```
settings.json                 app settings
accounts.json                 account metadata (no secrets)
servers.json                  saved servers
instances/<id>/instance.json  instance metadata
instances/<id>/minecraft/     isolated game directory
minecraft/versions/<id>/      version JSON, client jar, natives
minecraft/libraries/          Maven layout
minecraft/assets/             indexes + objects
minecraft/version_manifest.json
java/<major>/                 managed Temurin runtimes
logs/instances/<id>/*.log     launch logs
downloads/                    partial files and Java archives
cache/mojang-news.json        news cache
```

All JSON writes are atomic: write a sibling temp file, `sync_all`, rename.

## Security

- `Paths::resolve_inside` and `normalize_relative` reject absolute paths, NUL
  bytes and `..` escapes for every remote-controlled path.
- Remote URLs must be `http`/`https`. Modrinth download URLs are checked before
  a file is written.
- Account secrets use `keyring` (Windows Credential Manager / Keychain / Secret
  Service). If the keychain is unavailable, the operation fails with
  `SECRET_STORAGE_UNAVAILABLE`; secrets are never written in plaintext.
- The UI receives `AccountSummary` only: id, kind, name, uuid, timestamps and
  flags. Access tokens never cross into a page.
- No telemetry.

## Rendering and state

`ui::Yuhina` is the single root entity. It owns:

- the `Paths`, `Settings`, language and theme
- cached instance, account, server, news, version, Java and update state
- running game processes and their log rings
- install tasks and progress
- the current page, selected instance, tab and dialog

Commands are methods on `Yuhina`. Long-running commands use `spawn_task`, which
runs a closure on the background executor and applies the result on the UI
thread. Install progress is shared through `Arc<Mutex<InstallProgress>>` and a
120 ms poller; game logs are read from an `mpsc::Receiver` in a 120 ms pump.

## Theme

The app calls `gpui_kit::init`, `gpui_kit_material::init` and
`theme::init_component_theme` with `ThemeId::Expressive` before the first frame.
Theme changes go through `theme::advance_component_theme` and a next-frame
callback, so the scheme animates instead of snapping.

## Minecraft

Version metadata is resolved from the manifest and merged through
`inheritsFrom` chains. Loader profiles (Fabric, Quilt) are fetched from their
meta APIs and written under `versions/<loader-id>/`. The installer downloads
the parent vanilla version first, then the loader libraries.

Classpath order: client jar, then libraries in metadata order. Natives are
extracted into the root version's `natives/` directory and added through
`-Djava.library.path`.

## Known gaps

- Forge and NeoForge installer pipelines
- `.mrpack` modpack import
- SRV lookup for server addresses
- Skin head rendering (accounts use monogram avatars)
- In-app updater (the app links to the release page)
- Native title bar theming is left to the platform
