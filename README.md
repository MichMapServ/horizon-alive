# Horizon Alive v0.5.0

This update RETIRES the unwanted 2D HUD particle effects. It does not render 3D explosions in a normal Horizon .hmod. The SDK API 0.1 cannot render world geometry or observe TNT detonations.

The code now includes a testable Rust module, src/explosion3d.rs, that computes *world-space* TNT explosion geometry. It models a pixelated XZ-ground shockwave, a layered fireball dome and white/gray smoke ribbons. This is a native-renderer-ready mathematical core, NOT a working native renderer.

## What is delivered

- A .hmod that clears legacy HUD shapes and has no tick/frame particle hooks.
- Existing enabled, quality and motion preferences are retained in the isolated mod storage.
- A pure Rust 3D explosion core with time-based overlapping phases, deterministic seed, independent gameplay and visual radius, terrain sampling callback, and limits of 350 / 900 / 2000 primitives per event and four simultaneous events.
- Cargo and Horizon SDK tests and sandbox checks in GitHub Actions.
- Exact source audit and native integration requirements in docs/EXPLOSION_3D.md.

## Commands

/ alive status (remove space after slash when entering it in-game)
- /alive on or off: saves a future-renderer preference.
- /alive quality low, balanced or high: saves target fidelity.
- /alive motion reduced or normal: saves camera-motion preference.
- /alive graphics off: explicitly clears any retained legacy HUD.
- /alive demo boom: returns an unsupported message, rather than a fake HUD explosion.

## Build

Run these commands in the repository directory:

    rustup target add wasm32-unknown-unknown
    cargo install --git https://github.com/horizon-rust/horizon --tag v0.2.92 cargo-horizon-mod
    cargo fmt --all --check
    cargo test --all-targets
    cargo clippy --all-targets -- -D warnings
    cargo horizon-mod build
    cargo horizon-mod test
    cargo horizon-mod check target/horizon-mod/horizon_alive.hmod

After a successful GitHub Actions run, obtain horizon-alive-0.5.0-hmod and extract horizon_alive.hmod. Replace the old file in Horizon's horizon-mods directory and enable it in Options > Mods.

## Native 3D integration is NOT complete

The existing native source-mixin option builds a custom Horizon executable, NOT a .hmod. The official horizon-native-mod-template pins a different engine revision and cannot be applied unmodified to v0.2.93. It must be revision-pinned, compiled and tested separately. See docs/EXPLOSION_3D.md.

Passing .hmod sandbox checks does NOT mean that this new explosion appears in-world. It cannot until Horizon offers a world-space SDK hook or a separately validated native renderer is supplied.
