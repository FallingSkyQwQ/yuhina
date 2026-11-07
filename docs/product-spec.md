# Product Spec

## Vision

Yuhina is a Minecraft: Java Edition launcher with a calm, expressive Material 3
interface and Apple-HIG-grade interaction quality. It installs and launches the
game, keeps every world in an isolated instance, manages accounts and Java
runtimes, and browses Modrinth without leaving the launcher.

There is no AI assistant, chat, agent, skill or model provider in this product.

## Principles

- **One primary action per view.** Secondary actions are tonal/text and
  destructive actions are separated and confirmed.
- **Content first.** Rows and lists are for scanning; cards are for media and
  identity, not every surface.
- **8pt rhythm.** Spacing and radii come from tokens; text-coupled geometry is
  rem-based so 200% text scaling grows the layout.
- **Clarity over decoration.** M3E tonal surfaces and expressive type provide
  identity; HIG restraint keeps the interface quiet.
- **Keyboard and pointer parity.** Every action is reachable, labelled and
  focus-visible.

## Navigation

Adaptive navigation, cross-platform, Windows-first:

- **≥840dp**: expanded navigation rail (220dp) with icon+label rows.
- **600–839dp**: collapsed rail (96dp) with labels.
- **<600dp**: bottom navigation bar with five labelled destinations.

Primary destinations: Home, Instances, Library, Accounts, Servers.
Activity and Settings are top-bar actions; at compact widths they collapse into
one overflow menu.

Top bar: brand mark, page title, then Activity/Settings/theme/language at wide
widths. The page content keeps a single large title; the bar does not repeat it.

## Home

- Account-aware greeting; add-account CTA when no account exists.
- A 3/2/1 responsive grid of recent instances; news as a feed, not a stretched
  banner.
- One primary "New instance" entry point and one Play action for the latest
  instance.
- Loading skeleton, empty state and error/retry state.

## Instances

An instance is a self-contained game directory:

- name, icon, Minecraft version, loader (vanilla/fabric/quilt/forge/neoforge)
- isolated `minecraft/` directory
- memory mode (auto/manual), JVM and game arguments, Java override
- mods, resource packs, shader packs, worlds and screenshots
- playtime and last-played time

Create from a vanilla version with an optional loader. Forge and NeoForge use
the official installer jars. NeoForge 1.20.1 uses the legacy Forge artifact and
is not offered; NeoForge 1.20.2+ is supported.

Instance detail uses one primary action (Play or Install/Repair) plus a menu for
secondary actions. Tabs: Overview, Logs, Content, Backups, Files, Settings.
Tabs overflow into a menu at narrow widths.

## Accounts

- Microsoft OAuth device code.
- Offline account: 3–16 letters, numbers or underscores, deterministic UUID.
- LittleSkin and other authlib servers: URL + email + password, authlib-injector
  on demand.
- Multiple accounts, one active account.
- Refresh tokens and Minecraft access tokens encrypted in the OS keychain; the
  UI only sees summaries.
- Microsoft and LittleSkin skin heads are fetched, cropped and cached; monograms
  are the fallback.

## Java

- Detect `JAVA_HOME`, `PATH`, vendor folders and launcher-managed runtimes.
- Parse version, vendor and architecture with `-XshowSettings:properties`.
- Prefer exact major, then the lowest compatible newer major.
- Download Eclipse Temurin when a compatible runtime is missing and the setting
  allows it.
- Per-instance override and a detected-runtime list in Settings.

## Downloads

- Manifest, client, libraries and assets use BMCLAPI first in `auto`, then
  official Mojang endpoints.
- SHA-1/SHA-512 verification, bounded concurrency, retries and atomic writes.
- Asset index and objects follow the vanilla layout; natives are extracted into
  `versions/<id>/natives` with a marker.

## Launch

- Merge `inheritsFrom` metadata, build the classpath and natives.
- Support modern `arguments` and legacy `minecraftArguments`.
- Add `-javaagent` and prefetched metadata for authlib accounts.
- `--quickPlayMultiplayer` for 1.20+, legacy `--server`/`--port` otherwise.
- Stream stdout/stderr into a per-instance log and a bounded ring buffer.
- Emit starting/running/exited/crashed states and classify common crashes.
- Stop the process when the app exits.

## Servers

- Saved server list with name, address and port.
- SRV-aware Java status ping (MOTD, players, latency, version), fallback to the
  literal address.
- One-click join: launch the selected/first installed instance with the server
  target. Join passes the original hostname so Minecraft can re-resolve SRV.
- Add/remove with confirmation.

## Modrinth

- Search by type, loader and game version using `SearchField`.
- Install a project and its required dependencies into the selected instance.
- Resource packs and shaders install into their own folders; remove files.
- `.mrpack` import creates a new instance, installs Fabric/Quilt/Forge/NeoForge
  as declared, verifies hashes and extracts overrides.
- Modpack import reports progress and installer output in Activity.

## Backups and files

- Zip backups of the game directory with a manifest; restore creates a safety
  backup first.
- Export/import an instance as a zip with a manifest; validate every entry.
- Read-only file browser with breadcrumbs and a 64 KB text preview.

## Settings

- Appearance: theme (dark/light/system), language (zh-CN/en-US/system), motion
  (system/on/off), text scale (100/125/150/200%).
- Downloads: auto/BMCL/official.
- Java: auto-download, detected runtimes, rescan.
- Defaults: memory, JVM/game args.
- About: version, data directory, update check, release link.
- Settings changes apply immediately and persist atomically.

## Internationalization

- zh-CN default, en-US complete.
- Every user-facing string is a key; placeholders for composed values.
- Layout tolerates +30% English string length.
- Dates/numbers use locale-aware formatting.

## Accessibility

- Text scaling to 200%, with layout reflow rather than clipping.
- Full keyboard navigation, visible focus rings, Enter/Space activation,
  Escape to dismiss.
- Accessible names for icon-only controls.
- WCAG AA contrast in both themes; System theme and reduce-motion honor the OS.
- Snackbars are announced and errors persist with Retry/Close.
- No mouse-only action; destructive actions name their target and confirm.

## Non-goals

- Bedrock Edition
- In-launcher social chat, friends or messaging
- Cloud sync of instances or settings
- CurseForge (Modrinth only)
- Any AI assistant, agent or model provider
