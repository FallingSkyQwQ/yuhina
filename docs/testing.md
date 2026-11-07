# Testing and focus policy

## The rule

Agents and automated tests **never launch the GUI**. No `cargo run`, no
`Start-Process`, no `SetForegroundWindow`, no topmost or foreground capture.
The user performs manual visual checks. The app requests focus only when
`YUHINA_FOCUS=1`; by default it opens without activating the window.

This is a hard rule after repeated focus-theft reports. A change that adds a
GUI launch to CI or an agent workflow is rejected.

## Automated checks

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

CI runs the same on Windows, Linux and macOS. Linux installs the GPUI system
dependencies (fontconfig, X11/Wayland, Vulkan) before building.

Unit tests cover the headless engine:

- relative-path guards and id validation
- offline UUIDs and name rules
- version rules, Maven paths, asset paths
- quick-play and argument splitting
- server address parsing and SRV naming
- update semver ordering
- backup/export/import round trips and traversal rejection
- modpack index parsing and cleanup
- LittleSkin texture decoding
- i18n key parity and text-scale clamping

UI behavior that can be expressed as pure helpers should be tested there. The
GPUI element tree itself is verified manually, because a native window is
required for pixel capture.

## Manual visual matrix

Run the app yourself (agents must not):

```bash
cargo run
```

Check:

| Dimension | Values |
| --- | --- |
| Window | 1440×900, 1000×800, 800×600, 500×700 |
| Theme | dark, light |
| Language | zh-CN, en-US |
| Text scale | 100%, 200% |
| Pages | Home, Instances (detail + all tabs), Library, Accounts, Servers, Activity, Settings |
| Dialogs | Create instance, Add account, Add server, Delete instance, Restore/delete backup |
| Input | keyboard-only, focus rings, Enter/Space/Escape, reduced motion |
| States | loading, empty, error/retry, offline, busy |

Acceptance: no text or action is clipped; no primary control is covered by the
FAB or snackbar; every destination and dialog action is reachable; dark/light
contrast is AA; 200% text remains operable.

## Future headless capture

GPUI exposes `render_to_image` only with `test-support`, and it requires a
shown window in the current engine. A future harness may render the element tree
inside a test context and write PNGs under `target/ui-captures/`, but only if it
is focus-free by construction and never creates a native window. Until then,
screenshots are user-provided.

## Known gaps

- No automated pixel/visual regression.
- 200% text and the 500×700 matrix are pending the HIG UI pass.
- Installer stdout is available from core but not fully surfaced in Activity.
