# Product Spec

## Vision

Yuhina is a Minecraft: Java Edition launcher with a calm, expressive Material 3
interface. It installs and launches the game, keeps every world in an isolated
instance, manages accounts and Java runtimes, and lets the player browse
Modrinth without leaving the launcher.

There is no AI assistant, chat, agent, skill or model provider in this product.
The launcher is a focused tool.

## Non-goals

- Bedrock Edition
- In-launcher social chat, friends or messaging
- Cloud sync of instances or settings
- CurseForge (Modrinth only)
- A bundled or remote AI assistant of any kind

## Navigation

`NavigationSuite` adapts to the window:

- compact: bottom navigation bar
- medium: collapsed navigation rail
- expanded and wider: navigation drawer with labels

Destinations:

- **Home**: greeting, recent instances, Mojang news, quick launch.
- **Instances**: list, create, detail, install, repair, launch, logs, content,
  per-instance settings.
- **Library**: Modrinth search and installation into the selected instance.
- **Accounts**: Microsoft device-code, offline and LittleSkin/authlib accounts.
- **Servers**: saved servers, status ping, one-click join.
- **Activity**: running games and install progress.
- **Settings**: appearance, downloads, Java, defaults, about and updates.

A primary action (“New instance”) is always one click away from the navigation.

## Instances

An instance is a self-contained game directory:

- name, icon, Minecraft version, loader (vanilla/fabric/quilt/forge/neoforge)
- isolated `minecraft/` directory
- memory mode (auto/manual), JVM and game arguments
- Java override
- mods, resource packs, shader packs, worlds and screenshots
- playtime and last-played time

Create from a vanilla version with an optional loader. Forge and NeoForge are
recognized but disabled until their installer pipeline lands.

## Accounts

- Microsoft OAuth device code, client ID
  `5cde052b-0132-4f10-a523-186d372365ff`
- Offline account: 3–16 letters, numbers or underscores, deterministic UUID
- LittleSkin and other authlib servers: URL + email + password
- Multiple accounts, one active account
- Refresh tokens and Minecraft access tokens are encrypted in the OS keychain;
  the UI only sees `{ has token metadata }` summaries
- authlib-injector is downloaded on demand for authlib launches

## Java

- Detect `JAVA_HOME`, `PATH`, common vendor folders and launcher-managed
  runtimes.
- Parse `java.version`, vendor and architecture with
  `-XshowSettings:properties`.
- Pick an exact major version first, then the lowest compatible newer major.
- Download Eclipse Temurin JRE when a compatible runtime is missing and the
  setting allows it.
- Per-instance override and a detected-runtime list in Settings.

## Downloads

- Version manifest, client, libraries and assets use BMCLAPI first in `auto`,
  then official Mojang endpoints.
- SHA-1 verification, bounded concurrency, retries, atomic `.part` writes.
- Asset index and objects follow the vanilla layout.
- Native jars are extracted into `versions/<id>/natives` with a marker file.

## Launch

- Build the classpath from the resolved version (parents merged).
- Support both modern `arguments` and legacy `minecraftArguments`.
- Add `-javaagent` and prefetched metadata for authlib accounts.
- `--quickPlayMultiplayer` for 1.20+, legacy `--server`/`--port` otherwise.
- Pipe stdout/stderr into a per-instance log file and a bounded in-memory ring.
- Emit starting/running/exited/crashed states; classify common crash reasons.

## Servers

- Saved server list with name, address, port and optional icon.
- Java status protocol ping with latency, player counts and MOTD.
- One-click join: launch the selected (or first installed) instance with the
  server target.
- Add and remove servers.

## Modrinth

- Search projects by type, loader and game version.
- Install a project and its required dependencies into the selected instance.
- Resource packs and shaders install into their own folders.
- Remove installed files.
- Modpack installation is a known gap in this phase.

## Internationalization

- zh-CN default, en-US complete.
- Every user-facing string is a key in `src/ui/i18n.rs`.
- Layout tolerates +30% English string length.

## Accessibility

- Full keyboard navigation and visible focus rings from the kit.
- `prefers-reduced-motion` / the reduced-motion setting disables transforms.
- Accessible names for icon-only controls.
- WCAG AA contrast in both themes.
