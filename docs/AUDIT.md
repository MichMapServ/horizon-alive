# Horizon Alive 0.4.0 audit

Scope: source-level review of `MichMapServ/horizon-alive`, official Horizon SDK v0.2.92 and game renderer tag v0.2.93, reported crash on Intel UHD OpenGL.

## Findings and status

| Severity | Finding | Fix / evidence |
| --- | --- | --- |
| Critical, engine | Surface texture without COPY_SRC copied in frame encoder | Out of .hmod scope; concrete source audit and upstream fix recommended in ENGINE_CRASH.md |
| High, mod | v0.3.3's particle motion was barely perceptible | New per-tick movement physics with distinct ambient, projectile, and ring styles |
| Medium, mod | No configurable density | Three quality levels with bounded spawn rates |
| Medium, mod | Camera jolts could annoy sensitive players | Reduced-motion mode persisted |
| Medium, mod | Poor graphics isolation | `/alive graphics off` turns off retained HUD shapes and shake attempts |
| Medium, mod | World transition and teleport noise | Reset on dimension changes and non-finite positions; ignore large teleport jumps |
| Medium, mod | Minimal tests | Pure Rust simulation tests, reload/persistence and demo command sandbox tests |
| Medium, CI | Clean compile did not ensure GPU safety | README and workflow distinguish sandbox from actual renderer tests |
| Low, mod | Legacy on/off preferences could be lost | Keep `alive.enabled` storage key |

## Rendering constraints

- The official `render::set_hud` limit is 1024 shapes; the new mod uses at most 96 logical particles x 4 shapes = **384**.
- A single retained `render::set_hud` update per game tick (20 Hz). No frame callback is registered.
- The mod never creates a GPU encoder, swapchain, shader, texture or CPU readback.
- No network, file system, native code, shader injection or world modification.
- Quality low/balanced/high adjusts spawn interval and burst size, not GPU surface permissions.

## Validation levels

1. `cargo test`: simulation and config unit tests.
2. `cargo clippy -- -D warnings`: quality checks.
3. `cargo horizon-mod build`: package compilation.
4. `cargo horizon-mod test`: command and persistence regression tests with a virtual game.
5. `cargo horizon-mod check`: sandbox hook stress and import permissions.
6. **Manual**: real client with Windows 26200, Intel UHD OpenGL adapter; screenshots, menu blur, world transitions, F1, scaling, large DPI, max FPS and low memory.

Steps 1-5 cannot establish step 6. No one should claim that a green CI fixes the engine's COPY_SRC bug.
