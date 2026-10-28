# Horizon Alive WorldFX v0.7.0

**This is a real WebAssembly .hmod**, with code in Rust. The new world-space effects are generated using Horizon's verified vanilla `/particle` command implementation instead of the old HUD rectangle engine.

The mod's visual commands appear at real XYZ coordinates. They are still **vanilla particle sprites**, not custom volumetric meshes, so their appearance will not exactly match the large pixel-art explosion references.

## In-game use

Enable Horizon Alive in Options > Mods. In a creative test world with commands enabled, look at a block no more than 12 blocks away and type:

- `/alive preview` - show the explosion visual in the world, **no damage and no TNT**.
- `/alive tnt` - request **one vanilla TNT with fuse 1 tick** at that position, followed by the same world-space visual.
- `/alive status`, `/alive on`, `/alive off`.
- `/alive quality low|balanced|high`.
- `/alive scale 0.5..2.0`.
- `/alive smoke on|off`, `/alive flash on|off`.

Command permissions are those of the player. If cheats are unavailable, `/alive tnt` can fail to summon TNT; the mod cannot discover that outcome and may still show the preview. Use `/alive preview` for a harmless test.

**Detection:** Horizon v0.2.93's `.hmod` API has no primed-TNT entity list or `prime_fuse` event. `/alive detect on` enables a **best-effort local inference**: aim at TNT with flint and steel or a fire charge, use the item, and keep aiming while the TNT block disappears. It reports a *probable* ignition in chat. This does not detect remote, redstone, explosion-chain, or offscreen priming and cannot confirm whether the server accepted the action. `/alive detect fx on` optionally schedules the existing WorldFX animation approximately 80 ticks later; it is OFF by default pending in-game tests. `/alive detect last` reports last observation. Normal gameplay remains unchanged.

## Safety

- No HUD shapes. `render::set_hud(&[])` clears any legacy retained overlay.
- Never changes the original TNT blast physics or damage radius.
- At most 3 active effects and 44 visual commands per tick, shared fairly.
- No native code, shaders, world writes or copied GPU textures.
- Resets effects after changing dimensions or re-entering a world.
- Persists quality, size, smoke, flash, and enabled settings.

## Compile (or download Actions artifact)

The repository workflow compiles and uploads `horizon_alive.hmod` after Rust tests, strict Clippy, sandbox tests, and the official Horizon checker succeed. Install that file (not the source ZIP) inside the `horizon-mods` folder alongside Horizon's `options.txt`.

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/horizon-rust/horizon --tag v0.2.92 cargo-horizon-mod
cargo test --all-targets
cargo horizon-mod build
cargo horizon-mod test
cargo horizon-mod check target/horizon-mod/horizon_alive.hmod
```

Horizon engine tag v0.2.93 has the verified command-side `/particle` implementation. CI does not replace an in-game Intel UHD / OpenGL visual test, and it does not fix the unrelated engine's `COPY_SRC` crash.

## Ignition detector diagnostics

- `/alive detect on` / `/alive detect off`: enable or disable the passive inference.
- `/alive detect last`: show last inferred TNT position and estimated detonation tick.
- `/alive detect fx on`: opt in to the **existing** 3D WorldFX at the estimated end of the 80-tick fuse. Approximation only; explosions can move or be shortened.
- `/alive detect fx off`: continue detecting without spawning automatic visual effects (default).

Reliable event-driven detection of arbitrary vanilla TNT (redstone, chain TNT, server spawned, offscreen) remains blocked by the SDK interface. See `docs/TNT_DETECTION.md`.
