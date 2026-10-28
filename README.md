# Horizon Alive v0.8.0 — TNT pipeline

Sandboxed Minecraft Horizon mod (.hmod) in Rust: the code uses vanilla particle commands with world XYZ positions, not fake HUD overlays.

**Confirmed SDK limitation:** API 0.1 exposes no primed-TNT entity list, no real TNT fuse and no event for priming/detonation. Full coverage of redstone, minecarts, chained TNT, remote TNT or offscreen TNT is impossible using this client-only .hmod. The detector recognises only a **probable local ignition** when the player aims at TNT and uses flint and steel or a fire charge, then the block disappears under a stable camera.

## What v0.8 fixes

- For new installs, detection and its **estimated** automatic particle animation default to ON. Existing stored OFF settings remain OFF.
- One-line French chat messages and optional diagnostic logs distinguish actual mod startup, probable ignition, estimated detonation, queued particle commands and completion.
- New /alive diag and /alive diag last commands expose counters and the last recorded stage.
- Dimension changes and clock rewinds cancel temporary effects and reset pending detection.
- Existing WorldFX /alive preview, /alive tnt, quality, scale, smoke and flash commands remain available.
- Automatic detection **never spawns another gameplay explosion**.

## Useful in-game commands

| Command | Result |
| --- | --- |
| /alive status | Current state, observation and active-effect count |
| /alive diag | Diagnostic counters |
| /alive diag last | Last pipeline stage |
| /alive detect last | Last probable ignition position |
| /alive detect on / off | Switch local detector on/off |
| /alive detect fx on / off | Switch estimated animation on/off |
| /alive debug on / off | Additional log messages about submitted vanilla particle commands |
| /alive preview | Non-destructive XYZ particles at the aimed block |
| /alive tnt | Requests a single vanilla TNT; requires normal command permissions |
| /alive quality low / balanced / high | Quality profile |
| /alive scale 0.5..2.0 | Visual size |
| /alive smoke on / off | Smoke toggle |
| /alive flash on / off | Flash toggle |

## Build

On GitHub Actions, the .hmod is published only after Rust unit tests, cargo check, strict Clippy, cargo horizon-mod build/test/check and package verification pass. The SDK is pinned to Horizon tag v0.2.92; the renderer/engine audit refers to v0.2.93.

    rustup target add wasm32-unknown-unknown
    cargo install --git https://github.com/horizon-rust/horizon --tag v0.2.92 cargo-horizon-mod
    cargo fmt --all --check
    cargo check --all-targets
    cargo test --all-targets
    cargo clippy --all-targets -- -D warnings
    cargo horizon-mod build
    cargo horizon-mod test
    cargo horizon-mod check target/horizon-mod/horizon_alive.hmod

Install horizon_alive.hmod inside horizon-mods next to options.txt. Remove duplicate older copies and enable the mod in Options > Mods. Test in a disposable world with command permissions. Command submission is not proof of visual rendering because command::run_quiet returns no acknowledgement.

See [TNT pipeline audit and TNT-01–TNT-15 matrix](docs/TNT_AUDIT_V080.md) and [v0.2.93 engine crash diagnosis](docs/ENGINE_CRASH.md).
