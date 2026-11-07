# Yuhina

**A Material 3 Expressive Minecraft: Java Edition launcher.**

Yuhina installs and launches Minecraft, keeps every world in its own instance,
manages accounts and Java runtimes, and browses Modrinth — with a Material 3
Expressive identity and Apple-HIG-grade hierarchy, spacing and interaction.

> **v0.2.x — Rust rewrite.** The previous Electron/React build, its git history,
> its releases and every AI/chat/agent feature have been removed. Yuhina is a
> focused launcher with no assistant, no model provider and no hidden
> networking.

[简体中文](README.zh-CN.md)

## What it does

- **Instances** — vanilla, Fabric, Quilt, Forge and NeoForge (NeoForge 1.20.2+).
  Isolated game directory, memory, JVM arguments, logs, playtime.
- **Launching** — version metadata merging, library/asset installation, SHA
  verification, native extraction, Java selection and quick-play joins.
- **Accounts** — Microsoft device-code sign-in, offline accounts and
  LittleSkin/authlib servers. Tokens stay in the OS keychain; Microsoft and
  LittleSkin skin heads are fetched and cropped.
- **Java** — detects existing runtimes and downloads Eclipse Temurin when a
  version needs one.
- **Servers** — saved servers, SRV-aware status ping (MOTD/players/latency) and
  one-click join.
- **Modrinth** — search, dependency-aware installs, resource packs, shaders,
  `.mrpack` modpacks, and file removal.
- **Backups and files** — zip backups with a safety copy on restore, instance
  export/import, and a read-only file browser with text preview.
- **Settings** — theme, language, motion, text scale (100–200%), download source,
  Java, memory, about and update check.
- **Material 3 Expressive + HIG** — adaptive rail/bar navigation, five
  destinations, one primary action per view, kit components for rows/cards/
  search/segmented choices, dark/light themes, zh-CN and en-US.
- **Brand** — the geometric Y mark from `design/brand/yuhina-mark.svg`; the
  executable icon is generated from `yuhina-icon.svg`.

## Build

Requirements:

- Rust **1.99.1** (pinned in `rust-toolchain.toml`)
- Windows: Visual Studio 2022 with the MSVC toolchain and Windows SDK
- Linux: GPUI build dependencies (X11/Wayland, Vulkan drivers, fontconfig)
- macOS: Xcode command line tools

```bash
cargo build
cargo test
cargo build --release
```

The first build fetches `gpui-kit`, `gpui-kit-material` and their pinned engine
patches; it takes several minutes.

**Focus policy:** the app requests focus only when `YUHINA_FOCUS=1`. Agents and
tests never launch the GUI; the user runs it manually.

## Data

| Platform | Directory |
| --- | --- |
| Windows | `%APPDATA%\Yuhina` |
| macOS | `~/Library/Application Support/Yuhina` |
| Linux | `~/.config/Yuhina` |

The layout is documented in [`docs/architecture.md`](docs/architecture.md).

## Documentation

- [`docs/product-spec.md`](docs/product-spec.md)
- [`docs/design-system.md`](docs/design-system.md)
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/git-workflow.md`](docs/git-workflow.md)
- [`design/brand/README.md`](design/brand/README.md)

## Status and known gaps

The rewrite is usable for vanilla, Fabric, Quilt, Forge and NeoForge instances,
Microsoft/offline/LittleSkin accounts, Modrinth mods and modpacks, backups,
servers and settings. Known gaps:

- NeoForge 1.20.1 uses the legacy Forge artifact and is not offered.
- LittleSkin servers with a disabled session endpoint fall back to monograms.
- Installer output is available from core but not yet shown as a full Activity
  log.
- In-app update installation (the release page opens in the browser).
- Custom Windows title-bar appearance is in progress.
- Drag-and-drop import, command palette and tray behavior are not implemented.
- The HIG text-scale/component pass is in progress; 200% verification is not yet
  complete across every page.

## License

MIT. Third-party licenses are documented in the dependency tree and in
`gpui-kit-material`'s notices.
