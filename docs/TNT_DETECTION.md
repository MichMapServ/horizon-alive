# TNT ignition detection in .hmod (Horizon v0.2.93)

## Verified engine and SDK facts

- Engine: `crates/horizon-client/src/game/tnt.rs::spawn_primed_tnt` emits internal `minecraft:prime_fuse`, plays `minecraft:entity.tnt.primed` and initializes the TNT fuse at **80 ticks**. It creates a separate `PrimedTnt` object managed in `Game::tnts`; this event is **not** exported by WIT.
- Client WIT SDK (API 0.1): `game::raycast` sees **blocks only**, `input::held` exposes the main-hand item, and `input::mapping("key.use")` observes the vanilla use control. No exposed TNT entity list, world block query, sound listener or fuse event. `command::run_quiet` can issue commands but cannot inspect return values. TNT entities are **not** included in the game's `Target` selector candidates in `game/commands.rs` (lines around 682–706).
- Consequently **automatic exhaustive detection** of arbitrary primed TNT is impossible from this WIT API; event-driven detection would require a new upstream host function or native change.

## Implemented detector v0.7

An **inference for local manual ignition** without changing vanilla blocks, commands or gameplay:

1. Record the block under the crosshair when it is `minecraft:tnt`, and an ignition item (`minecraft:flint_and_steel` or `minecraft:fire_charge`) is held.
2. On a fresh press of `key.use`, create a pending ignition candidate. A recently observed TNT/item may be reused for two or three ticks, because a consumed fire charge or an immediately removed TNT block may be absent by the next snapshot.
3. Within four ticks, require that the same TNT block is no longer the looked-at block, **while the camera and player remain nearly stationary** (<= 6 degrees, <= 0.8 blocks).
4. Report once per dimension and block location within 120 ticks, avoiding repeated notifications. Chat calls explicitly say **probable / unconfirmed**.
5. The last observation can be inspected using `/alive detect last`; `/alive status` includes the detector count.

No new TNT is spawned, destroyed, removed or extended by this detector. Ordinary redstone TNT, explosion chains, remote server explosions, and TNT primed while outside the crosshair **will not be detected**.

## Visual integration (off by default)

`/alive detect fx on` schedules the existing v0.6 vanilla world-space particle animation about 80 ticks after the observed manual ignition. It approximates the default fuse but **cannot follow the real TNT position**, distinguish shortened fuses or verify that an ignition was actually accepted. Default is OFF because the current task prioritizes detection fidelity over texture work.

`/alive detect fx off` disables the scheduling but leaves detection active. Quality, smoke and scale settings persist unchanged.

## Risks and testing

- Using a charge that is rejected by game rules, breaking TNT by another action during the pending window, or quickly shifting the camera may respectively cause a false positive or false negative.
- Very fast presses shorter than a tick might be missed by `input::mapping` at 20 Hz.
- Streaming/teleporting away clears pending information. The detector never modifies gameplay state.
- Unit tests cover ignition, consumed items, stationary viewing, missing ignition items, camera motion, dimension changes, dedup and backward time.
- CI checks Cargo, Clippy, sandbox command/persistence and the `.hmod` package; **real manual flint/charge gameplay** remains to be tested in the Windows client.
- For guaranteed TNT events, propose a future API such as `tnt-primed(id, dimension, position, fuse)` and `explosion-started(id, ...)` to Horizon upstream.

## Manual test

1. Install the v0.7 `.hmod`, run `/alive detect on`.
2. Aim at placed TNT from under 8 blocks while holding flint and steel, click once, and **do not move the camera** until the TNT block disappears.
3. Expect a chat line **Probable TNT ignition at x y z ... unconfirmed**, then `/alive detect last` shows the same position.
4. Test fire charge (may be consumed), then a TNT destroyed without any ignition item (should not report).
5. Attempt TNT ignited by redstone (not supported); observe no automatic event.
6. Optionally enable `/alive detect fx on` to see the existing WorldFX after the approximate fuse.
