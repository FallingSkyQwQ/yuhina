# Yuhina

**A Material 3 Expressive Minecraft: Java Edition launcher.**

Yuhina installs and launches Minecraft, keeps every world in its own instance,
manages accounts and Java runtimes, and browses Modrinth — in a calm desktop
shell built with Rust, [GPUI](https://github.com/zed-industries/zed) and
[`gpui-kit-material`](https://github.com/mizorewww/gpui-kit-material).

> **v0.2.0 — full rewrite.** The previous Electron/React build, its git history,
> its releases and every AI/chat/agent feature have been removed. Yuhina is now
> a focused launcher with no assistant, no model provider and no hidden
> networking.

[简体中文](README.zh-CN.md)

## What it does

- **Instances** — vanilla, Fabric and Quilt today; Forge and NeoForge are
  recognized but disabled until their installers land. Per-instance game
  directory, memory, JVM arguments and logs.
- **Launching** — version metadata merging, library and asset installation,
  SHA-1 verification, native extraction, Java selection, quick-play joins.
- **Accounts** — Microsoft device-code sign-in, offline accounts and
  LittleSkin/authlib servers. Tokens stay in the OS keychain.
- **Java** — detects existing runtimes and can download Eclipse Temurin when a
  version needs one.
- **Servers** — saved servers, status ping with MOTD/players/latency, one-click
  join.
- **Modrinth** — search by type/loader/version, dependency-aware installs,
  resource packs and shaders, and file removal.
- **Material 3 Expressive UI** — adaptive navigation bar/rail/drawer, tonal
  surfaces, expressive type, dark and light themes, zh-CN and en-US.

## Build

Requirements:

- Rust **1.99.1** (pinned in `rust-toolchain.toml`)
- Windows: Visual Studio 2022 with the MSVC toolchain and Windows SDK
- Linux: the usual GPUI build dependencies (X11/Wayland, Vulkan drivers)
- macOS: Xcode command line tools

```bash
cargo run            # development build
cargo test           # unit tests
cargo build --release
```

The first build fetches `gpui-kit`, `gpui-kit-material` and their pinned engine
patches; it takes several minutes.

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

## Status and known gaps

This is an early rewrite. The launcher is usable for vanilla, Fabric and Quilt
instances, Microsoft/offline/LittleSkin accounts, Modrinth mods, servers and
settings. Known gaps:

- Forge / NeoForge installer pipelines
- `.mrpack` modpack import
- SRV lookup for server addresses
- Per-instance file browser, backups and export/import
- Skin-head rendering (accounts use monogram avatars)
- In-app updater (checking links to the release page)

## License

MIT. Third-party licenses are documented in the dependency tree and in
`gpui-kit-material`'s notices.
