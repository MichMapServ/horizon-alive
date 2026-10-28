# Horizon Alive

A lightweight **Horizon** client mod with HUD-space ambient particles, attack sparkles and damage feedback.

The effects are **2D HUD particles**, not free-moving 3D world particles. Explosions and water effects from other mods are not automatically detected by the Horizon v0.2.92 mod API.

## Compile on GitHub Actions

Open **Actions > Build Horizon Alive hmod** and select a successful workflow run. Under **Artifacts**, download `horizon-alive-0.3.3-hmod` and extract `horizon_alive.hmod`.

The workflow installs the official Horizon packaging tool, tests and lints the Rust code, packages the component and runs `cargo horizon-mod check` before uploading an artifact.

## Local build

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon --tag v0.2.92 cargo-horizon-mod
cargo test
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/horizon_alive.hmod
```

Place `horizon_alive.hmod` in the **horizon-mods** folder beside the game's `options.txt` file, then enable it in **Options > Mods**.

Commands: `/alive on`, `/alive off`, `/alive status`, `/alive demo`.

## Status

Build success and in-game stability are separate checks. A green CI indicates build and sandbox validation, not visual testing in a running game.
