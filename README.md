# Horizon Alive v0.4.0

A configurable, **screen-space** visual effects mod for Minecraft Horizon, built as a single native WebAssembly component inside a `.hmod` archive. It is **not** a 3D world particle renderer.

## Effects

- A restrained daytime dust/leaf ambience, nighttime fireflies, Nether embers and End motes.
- Swing sparkles with colors inferred from the block in the crosshair.
- Health-loss sparkles and an optional subtle camera jolt.
- Motion dust trails when moving, with teleport detection.
- Manual 2D previews of water rings, explosions, fireflies and swings.

The `render` capability draws only HUD geometry. No world-space particles, vanilla explosion hook, water-surface wake detection, or shader replacement is exposed by the Horizon 0.1 client mod API.

## Commands

| Command | Description |
| --- | --- |
| `/alive status` | Current settings and live particle count |
| `/alive on`, `/alive off` | Enable/disable effects |
| `/alive quality low` | Minimal particle density |
| `/alive quality balanced` | Default |
| `/alive quality high` | More particles |
| `/alive motion reduced` | Suppress camera jolts |
| `/alive motion normal` | Enable camera jolts |
| `/alive graphics off` | Disable the mod's HUD renderer for troubleshooting |
| `/alive graphics on` | Restore HUD rendering |
| `/alive demo boom` | 2D shock ring and sparks |
| `/alive demo water` | 2D ripples and droplets |
| `/alive demo fireflies` | 2D luminous motes |
| `/alive demo punch` | 2D swing sparks |

Settings are persisted in Horizon's isolated mod storage. The previous `alive.enabled` setting from v0.3.x is preserved. The same settings are also available in Options > Mods.

## Build

Source and CI target the Horizon v0.2.92 SDK API 0.1, compatible with Horizon v0.2.93. `cargo-horizon-mod` builds a WebAssembly component, checks imports versus the manifest, then runs its test sandbox.

A **green GitHub Actions build** uploads `horizon-alive-0.4.0-hmod`. The archive holds `horizon_alive.hmod`. Do not rename the source ZIP to `.hmod`.

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon --tag v0.2.92 cargo-horizon-mod
cargo fmt --all --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo horizon-mod build
cargo horizon-mod test
cargo horizon-mod check target/horizon-mod/horizon_alive.hmod
```

Copy the `.hmod` to your Horizon game folder's `horizon-mods/` directory next to `options.txt`, then enable the mod in **Options > Mods**.

## IMPORTANT: Horizon v0.2.93 Intel OpenGL crash

If you see `Surface Texture ... required COPY_SRC`, see [docs/ENGINE_CRASH.md](docs/ENGINE_CRASH.md). This is a validation failure in the game's GPU command encoder; the `.hmod` cannot fix the swapchain permissions. A passing sandbox check is **not** a test of the Windows OpenGL renderer. For comparative diagnosis, test the same world **without the mod** and try `/alive graphics off` to disable HUD shapes, if the game can start.

## Validation

- Native Rust unit tests exercise simulation limits, lifecycle and configuration.
- Horizon sandbox tests exercise commands, migration and reload.
- GitHub CI builds and checks the `.hmod`, then uploads only after validation succeeds.
- A real Windows/Intel UHD graphics test is still required to certify visual stability.

See [docs/AUDIT.md](docs/AUDIT.md) for details.
