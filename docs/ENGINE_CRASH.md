# Horizon v0.2.93: wgpu COPY_SRC validation failure

## Observed in the player's crash report (28 Oct 2026)

- Horizon **v0.2.93**, Minecraft 26.3, Windows 10.0.26200, Intel UHD Graphics via **OpenGL** driver 32.0.101.7088.
- `Texture '<Surface Texture>'` declared `RENDER_ATTACHMENT`, but a `CommandEncoder` attempted to use it as `COPY_SRC`.
- The reported mod was initialized, and `/alive demo` returned feedback 17 seconds earlier. This temporal sequence is **not evidence** of a causal connection.
- No Horizon mod sandbox operation can set a `wgpu::SurfaceConfiguration` usage flag.

## Engine audit: independently verified against tag v0.2.93

1. `crates/horizon-client/src/gfx.rs` around line 1108: swapchain `COPY_SRC` is requested only if the adapter reports support; Intel/OpenGL can expose a presentable surface without that usage.
2. Around line 5178: when a screenshot is pending, the game checks the **pixel format**, but calls `self.copy_frame(...)` without first verifying `target_texture.usage().contains(wgpu::TextureUsages::COPY_SRC)`.
3. Around line 5277: `copy_frame` records a `copy_texture_to_buffer` operation; this requires `COPY_SRC` on the source texture.
4. The screenshot writer around line 5298 checks `self.config.usage`, but that happens **after** the invalid GPU command has already been encoded. This is too late.
5. The menu blur gate near line 5135 checks `self.config.usage`, not the actual source `target_texture.usage()`. While often equivalent, the latter is the reliable permission source, especially with alternate rendering configurations.

The screenshot path is a concrete reproducer candidate; an actual capture or another GPU-copy feature must be confirmed to distinguish it from other paths.

## Suggested upstream engine correction (not part of a .hmod)

Inside `gfx.rs`, before ever calling `self.copy_frame`, require:

```rust
if !target_texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
    self.screenshot_result = Some(Err(
        "the current surface cannot be copied for screenshots".to_owned()
    ));
    None
} else if self.config.format.block_copy_size(None) != Some(4) {
    self.screenshot_result = Some(Err(
        format!("screenshots need a 32-bit surface format, not {:?}", self.config.format)
    ));
    None
} else {
    Some((path, self.copy_frame(&mut encoder, &target_texture)))
}
```

Likewise gate menu blur using `target_texture.usage()` rather than only `self.config.usage`. The engine should also test behavior on an adapter whose surface capabilities omit `COPY_SRC`.

**Do not unconditionally add `COPY_SRC` to the surface configuration:** wgpu can reject it if the adapter does not support that usage.

## Reproduction / immediate diagnostic steps

1. Back up your world before reproducing crashes.
2. Relaunch v0.2.93 with **no .hmod loaded**, and try entering a test world.
3. If the engine remains stable, enable Horizon Alive in `/alive graphics off` diagnostic mode and try again. This suppresses the mod's HUD shapes; it does not change engine GPU usage.
4. Note whether pressing **F2** or opening menus with background blur triggers the same crash.
5. Compare another supported graphics backend (for example Vulkan, if your Horizon build offers it) and an updated Intel driver; record the exact selections.
6. Attach the **full new crash log** and exact preceding actions to the upstream Horizon issue.

## Important distinction

The mod is bounded to 96 logical particles and 384 retained HUD shapes. That reduces its overhead but **cannot** make an invalid wgpu surface copy legal. Only a Horizon renderer fix can prevent that class of engine validation error.
