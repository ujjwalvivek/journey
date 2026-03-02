## Journey Engine v1.0.0 - Release Notes

The first major release of Journey Engine. A custom 2D game engine built in Rust and wGPU, shipping a fast-momentum Metroidvania tech demo to both native desktop and WebAssembly.

---

### Engine Core

* **Trait-Based Game Architecture:** Games implement `GameApp` with `init`, `update`, `fixed_update`, `render`, and `ui` hooks. The engine owns the event loop, the game owns the logic.
* **wGPU Rendering Pipeline:** Orthographic camera, instanced sprite batching (up to 1024 per draw call), UV-based sprite sheet addressing, and nearest-neighbor upscaling from a configurable internal resolution (640×360).
* **Dual Blend Pipelines:** Alpha and additive blend modes within a single render pass for layered VFX.
* **Texture Manager:** `TextureHandle` abstraction over GPU texture loading, bind group creation, and atlas lookups.
* **eGUI Debug Overlay:** Immediate-mode GUI for runtime inspection of physics config, combat state, animation frames, and audio volumes.
* **Fixed-Timestep Simulation:** Deterministic 60Hz accumulator with max-step cap (5) preventing spiral-of-death, interpolation alpha for render smoothing, and frame-freeze for hitstop.

### Physics & Collision

* **AABB Collision Detection:** Center/half-size representation with overlap checks, minimum translation vectors, and platformer-biased Y-axis resolution.
* **Swept AABB (CCD):** Minkowski-expanded ray cast returning time-of-impact and contact normal for continuous collision along a displacement vector.
* **Multi-Layer Collision:** `Pushbox`, `Hurtbox`, `Hitbox`, and `Parrybox` layers with `BoxVolume` supporting local offsets and facing-direction flip.
* **One-Way Platforms:** Separate collision lists for solid, one-way, and wall geometry; downward-only resolution for one-way platforms.

### Combat System

* **Frame-Deterministic FSM:** `Idle → Startup → Active → Recovery` state machine driven by integer tick counts. Zero floating-point timing.
* **Data-Driven Move Database:** Six moves (`AttackHorizontal`, `AttackUp`, `AttackDown`, `Parry`, `Dash`, `Grapple`) defined with startup/active/recovery frames, damage, knockback, recoil, hitbox geometry, and cancel-window percentages. Tick-rate scaling preserves frame data across different update rates.
* **Tick-Stamped Input Buffer:** Combat inputs queued with tick stamps and consumed within a 20-frame window (~333ms at 60Hz), bridging the gap between variable-rate `update` and fixed-rate `fixed_update`.
* **Hitbox System:** Tick-windowed active ranges, directional knockback, posture damage, and `BoxVolume`-driven world-space AABB generation.
* **Parry Mechanic:** Active-phase parry window that deflects projectiles, staggers source enemies, and triggers hitstop (3 ticks freeze).
* **Invincibility Frames:** Dash Active phase grants full i-frame protection.
* **Health & Hitstun:** Clamped health with damage/posture tracking, directional knockback impulses, and friction-based hitstun deceleration.

### Player & Movement

* **15-State Player FSM:** Idle, Run, Jump, Fall, Dash, AirDash, Parry, AttackHorizontal, AttackUp, AttackDown, WallGrab, WallSlide, GrapplePull, GrappleSlingshot, Death.
* **Precision Platforming Assists:** Coyote time (6 ticks), jump buffering (8 ticks), and variable jump height via early-release gravity multiplier (6×).
* **Dash:** Ground and air dash with cooldown (10 ticks), 800 px/s speed, single air dash per airtime.
* **Wall Interactions:** Wall grab timeout, wall slide speed (120 px/s), wall jump with directional lock (30 ticks), detach grace period, and re-entry cooldown.
* **Grapple:** Pull toward grapple nodes (400 px/s), slingshot launch (600 px/s for 6 ticks), and bounce with separate X/Y velocity components.
* **Runtime-Tunable Physics:** All movement parameters adjustable live via the debug overlay.

### Enemy AI

* **Three Enemy Types:** Grunt (patrol + shoot), Sniper (stationary + long-range aim), Ronin (patrol + melee). Data-driven via `EnemyConfig`. Sniper and Ronin added in post-MVP.
* **Shared FSM:** Idle → Patrol → Aim → Shoot → MeleeWindup → Stagger, with per-type aggro ranges and combat parameters.
* **Core Combat Loop:** Enemy shoots → projectile flies → player parries → enemy staggers → player grapples → execute.
* **Ledge-Aware Patrol:** 4×4 pixel sensor ahead of feet prevents enemies from walking off platforms.
* **Line-of-Sight:** Raycast against wall geometry for aggro/shoot gating.

### Projectiles

* **Object Pool:** `ProjectilePool` manages lifecycle with alive/dead slot reuse.
* **Ricochet:** Wall bounce with a one-bounce limit before despawn.
* **Parry Interaction:** Parry-box contact destroys the projectile and staggers the source enemy via `EnemyHandle` callback.
* **Range Limit:** Auto-despawn at 400px max range; collision against 4×4 pixel AABB.

### Audio

* **Kira Backend:** Cross-platform audio with lazy WASM init (Web Audio API user-gesture requirement).
* **Four Sub-Tracks:** Music, Ambience, SFX, UI with independent per-track volume controls and a master volume.
* **17 Gameplay Events:** Jump, Land, Dash, Run, WallGrab, WallSlide, Swing, Hit, Parry, Stagger, Death, Respawn, GrappleStatic, GrappleEnemy, Projectile, ProjectileBounce, RunStop.
* **UI Audio:** `AudioResponse` trait auto-wires hover, click, checkbox, and tab-change sounds to egui widgets.
* **Embedded Assets:** All sounds loaded via `include_bytes!` for single-binary distribution on both platforms.

### Level Editor

* **Dual-Mode Editor:** Toggle with `F12`. Visual mode (pan with WASD/arrows/middle-click drag) and Text mode (raw ASCII editing with live minimap preview).
* **Level Validation & Legend:** Warns on missing spawn points and required elements before saving.
* **Platform Types:** Floor, Crate, OneWay, Wall with distinct collision behaviors. Grapple nodes and per-type enemy spawn points defined in-level.
* **Cross-Platform Persistence:** Filesystem on native, `localStorage` on WASM. Hot-reload saves directly to `world.txt`.

### Visual Effects

* **Screen Shake:** Decaying sinusoidal shake with Lissajous-like orbit (1.3× Y frequency ratio) for organic non-repeating motion. Configurable intensity, duration, frequency (40Hz), and exponential decay.
* **Perlin Fog:** Real-time animated fog via Perlin noise at 32×32 CPU resolution, uploaded as a GPU texture each frame. Vertical mask for top-half cloud effect with configurable density, opacity, and color.
* **VFX Bursts:** Timed particle bursts at world positions with configurable color for combat hit effects.
* **Splash Screen:** Fade-in/fade-out title screen with letterboxing.

### Cross-Platform

* **Native Desktop:** Blocking event loop via winit with fullscreen toggle, window icon, and 320×240 minimum dimensions.
* **WebAssembly:** Non-blocking event loop via `spawn_app()` with `thread_local!` pending state for async GPU init. Custom `wasm_ready_event` dispatch for JS interop.
* **Web Tooling:** Vite dev server with `wasm-pack` integration for rapid WASM iteration, TypeScript glue code, and npm package publishing.
* **Conditional Compilation:** Platform-gated gamepad support (gilrs), audio init, level persistence, and logging backend.

### Release Tooling (NEW)

* **Go Release TUI:** Full-featured terminal release tool built with Bubble Tea. Seven screens: Loading → Welcome → Version → Mode → Execution → CI → Summary.
* **Automated Pipeline:** Version bump → sync `web/package.json` → verify sync → stage → commit → checkout main → fast-forward merge → tag → push → push tag. Supports `staging → main` and `main`-only flows.
* **Approval Modes:** Zen (auto-approve all steps) and Prompt (manual step-by-step approval).
* **CI Monitoring:** Polls GitHub Actions `publish.yml` workflow with configurable timeout (default 45 minutes).
* **Release Statistics:** Tracks total/preflight/pipeline/CI durations, approval counts, success rate, and diff collection.
* **Headless Mode:** `--headless`, `--version`, `--mode`, `--dry-run`, `--skip-ci`, `--dirty` flags for CI/scripting use.
* **Version Sync Tool:** Standalone Go tool syncs `Cargo.toml` workspace version to `web/package.json` with `-check` validation and `-print` inspection modes.
* **Cross-Platform Builds:** Makefile targets for 6 platform/arch combos (darwin/linux/windows × amd64/arm64).

### Under the Hood

* **UV-Space Sprite Flipping:** Horizontal flip moved from scale-space to UV-space, eliminating the ghost-teleport rendering artifact.
* **O(1) Animation Lookups:** `AnimationState` caches `current_index` to avoid `O(n)` name-based lookups on every update.
* **Dynamic Animation Scaling:** Combat animations auto-derive duration from FSM frame data instead of requiring manual specification.
* **Dimensional Consistency:** All configuration constants normalized to a `PIXELS_PER_UNIT` base.
* **Audio Init Fast-Path:** `is_initialized` flag on `AudioManager` skips `Option::is_none()` branching for post-init SFX calls.

### Testing

* **72 Rust Tests:** 22 engine tests (physics, time, noise) + 50 game tests (combat FSM, input buffer, move database, health, entity physics, enemy AI, projectile pool, parry deflection, determinism).
* **Go Tool Tests:** Full test suites for both release (pipeline construction, orchestrator, headless mode, stats, semver) and versioning (TOML/JSON parsing, version sync) tools. All tests use `stubExecutor`, zero network, git, or filesystem access.
* **Zero Clippy Warnings:** `cargo clippy -D warnings` enforced across both crates.

### Bug Fixes (Cumulative)

* Fixed silent audio event loss by consolidating all dispatches into a single drain point at the end of each fixed tick.
* Fixed sprite ghost-teleport on horizontal flip via UV mirroring instead of negative scaling.
* Fixed physics spiral-of-death on slow frames by capping raw delta time at 100ms.
* Fixed dash stopping dead on wall contact by preventing dash states from restoring pre-collision X positions.
* Fixed camera Y clamping for maps with geometry above the y=0 origin.
* Fixed grounded landings zeroing out Y velocity for dash state using grounding force.
* Fixed audio amplitude precision by keeping calculations in `f64` until the final `f32` decibel cast.
