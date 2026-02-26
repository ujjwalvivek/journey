## Journey Engine v0.3.2 - Release Notes

### Features

**Engine & Physics**

* **Fixed-Timestep Physics:** Game logic now runs on a deterministic fixed-rate accumulator to ensure identical physics, combat, and FSM results.
* **Swept AABB Collision Detection:** Implemented Minkowski-expanded ray casting for ccd.
* **Game State Machine:** Implemented a clean enum-based state machine managing the game screens.

**Combat System**

* **Frame-Data Combat:** Full Startup/Active/Recovery combat FSM implemented with integer tick-based timing, supporting attack cancels during recovery windows.
* **Tick-Stamped Input Buffer:** Combat inputs are now queued with tick timestamps and consumed within a configurable frame window to allow for leniency and precise input timing.

**Level Editor**

* **Dual-Mode Level Editor:** Press `F12` to toggle a full-screen editor that operates on a single canonical ASCII string.
* **Live Minimap:** Added a color-coded minimap driven directly from the ASCII buffer that updates live on every keystroke.
* **Level Validation & Legend:** Added a validation pass that warns on missing elements before saving.
* **Universal Persistence:** Hot-reloading saves directly to `world.txt` on native builds, and seamlessly to `localStorage` on WASM web builds.

**Audio & Visuals**

* **Cross-Platform Audio Engine:** Integrated Kira audio supporting four independent sub-tracks with lazy WASM init, music ducking, and an `AudioResponse` trait for egui.
* **Screen Shake System:** Added a decaying sinusoidal screen shake with configurable variables.
* **Additive Blend Pipeline:** Added a secondary GPU render pipeline within the same render pass to support additive-blended sprites.

### Under the Hood

* **UV-Space Sprite Flipping:** Moved sprite horizontal flipping from scale-space to UV-space.
* **O(1) Animation Lookups:** `AnimationState` now caches `current_index` to avoid `O(n)` name-based lookups on every update.
* **Audio Init Fast-Path:** Added an `is_initialized` flag to the `AudioManager` so post-init SFX calls completely skip `Option::is_none()` branching.
* **Dynamic Animation Scaling:** Combat animation durations are now dynamically derived from FSM frame data instead of requiring manual specification.
* **Dimensional Consistency:** Normalized all configuration constants to a `PIXELS_PER_UNIT` base.

### Bug Fixes

* Fixed silent audio event loss from late code paths in `fixed_update` by consolidating all dispatches into a single drain point at the end of the tick.
* Fixed the sprite "ghost teleport" bug on horizontal flips by utilizing UV mirroring instead of negative scaling.
* Fixed the physics spiral-of-death on slow hardware frames by capping raw delta time at 100ms.
* Fixed the dash state stopping dead on wall contact by preventing dash states from restoring pre-collision X positions.
* Fixed camera Y clamping issues for maps containing geometry above the y=0 origin.
* Fixed grounded landings to correctly zero out the Y velocity for the dash state using grounding force.
* Fixed audio amplitude precision by keeping calculations in `f64` and only casting to `f32` at the final decibel construction.
