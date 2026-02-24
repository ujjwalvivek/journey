# Journey - Technical Design Document (TDD)

## Architectural Summary

| **Owner**        | Ujjwal Vivek (Technical Product Manager)                   |
| ---------------- | ---------------------------------------------------------- |
| **Core Stack**   | Rust, wGPU (WebGPU), Winit, WASM                           |
| **Physics**      | Glam, Nalgebra                                             |
| **Architecture** | Custom ECS (Entity Component System), Data-Oriented Design |
| **Target**       | Native (Dev) + WebAssembly (Distribution)                  |
| **Status**       | **Phase 0: Initialization**                                |

## A different take on Souls-like Metroidvania

A custom high-performance 2D ECS game engine written in Rust + WGPU. Features AABB physics, focuses on precision platforming (*Hollow Knight*) and parry-based combat (*Sekiro*/*Nine Sols*) with a touch of a fast momentum based platformer (*Ghostrunner*). For a Metroidvania running at 60FPS (`Important Metric`) in a web browser, I want tight, deterministic, arcade physics, not realistic simulations. This project also serves as a "Living Proof of Work" for a **TPM** role.

It's all in the details. It’s not just about `Can I jump?` but `How does it feel to jump?` The secret sauce is in the mechanics that make the player feel powerful and responsive.

- **Coyote Time**: Allow jumping for 0.1s after walking off a ledge.
- **Jump Buffering**: If I press 'Jump' 0.1s before hitting the ground, execute it on landing.
- **Variable Jump Height**: Tap 'A' for a hop, hold 'A' for a leap.
- **Parry Mechanic**: If I press 'Parry' within 0.2s of an enemy attack, I negate damage and stagger the enemy.
- **Hurtbox**: The area where the player takes damage.
- **Hitbox**: The area where the sword deals damage.
- **Parrybox**: A special box that, if it overlaps an enemy Hitbox within 0.2s, triggers the "Clang" effect and negates damage.

## Technical Architecture

The pipeline needs to be `Cross-Platform First`.

- The **"Renderer"** uses `wgpu` (which targets `Vulkan/Metal/DX12` on Desktop, and `WebGL/WebGPU` on Browser).
- The **"Input"** uses `winit` (which captures `Windows events` on Desktop, and `JS events` on Browser).

### Folder Architecture

```bash
Journey/
├── Cargo.toml          //* Workspace definition
├── engine/             //* The reusable library (Product)
│   ├── src/lib.rs      //* ECS, Renderer, Input, Physics
│   └── Cargo.toml      //* Dependencies: wgpu, winit, bytemuck
├── game/               //* The executable (Content)
│    ├── pkg/           //* WASM Artifacts (Generated here)
│    ├── src/main.rs    //* Level design, Player stats, Assets
│    └── Cargo.toml     //* Dependencies: engine
└── web/                //* Vite Project for WASM distribution
    ├── src/            //* JavaScript Glue Code 
    ├── package.json    //* NPM Dependencies
    └── vite.config.js  //* WASM Configuration

```

This forces a clean API from `engine` to `game`. It also allows `engine` to be compiled as a separate crate for testing and documentation and prevents spaghetti coupling.

All crates should be `WASM-compatible`.

- **`pollster`**: This will be used for the `main()` function. Native Rust supports async main, but WASM is complicated. This will help block the thread safely to make async code feel synchronous where needed.
- **`console_error_panic_hook`**: Crucial for Web. If the game crashes in the browser, this pipes the Rust panic message to the Chrome DevTools console.

### The Custom ECS (Entity Component System)

Build a native ECS using Contiguous Memory. Alternatively, existing ECS library like `hecs` or `specs` could be used, but building one will give me more control over performance optimizations.

- **Entities:** Simple `u32` IDs.
- **Components:** Struct-of-Arrays (`position`, `velocity`, etc.) or `HashMap<EntityID, Component>` for initial simplicity.
- **Systems:** Functions that iterate over component queries (e.g., `fn physics_system(pos: &mut Position, vel: &Velocity)`).

```bash
[ World Struct ]
   |
   +-- [ Entities (Vec<u32>) ]  : [ 0, 1, 2, 3 ... ]
   |
   +-- [ Component Stores ]
         |
         +-- Positions (Vec<Pos>): [ {x,y}, {x,y}, {x,y} ... ]
         |                          ^ tightly packed for CPU cache
         |
         +-- Velocities (Vec<Vel>): [ {dx,dy}, None, {dx,dy} ... ]
                                    ^ 'None' if Entity 1 has no velocity
```

Implement **Delta Time (`dt`)**.

- On Native: `dt` is the time since the last frame.
- On Web: `requestAnimationFrame` controls the loop.
- physics should be consistent regardless of frame rate.

## Success Criteria (MVP)

- **Performance:** 60 FPS stable on both Desktop and Chrome/Firefox/Safari.
- **Size:** WASM binary under 5MB (gzip). Keep it lean.
- **Gameplay:** Souls-like Metroidvania feel.
- **Code Quality:** No `unwrap()` in the main loop and clean separation between Engine and Game.

## The Roadmap (Phases)

### Phase 1,2,3: Completed

### Phase 4: The Combat

- [x] **State Machine:** Implement `PlayerState` enum (`Idle`, `Attack`, `ParryWindow`, `Stun`).
- [x] **Hitbox Architecture:** Separate `Hurtbox` (Body) from `Hitbox` (Weapon).
- [X] **Parry Logic:** Create a window where overlapping an enemy `Hitbox` triggers a "ParrySuccess" event instead of "Damage".
- [X] **Enemies:** Implement enemy entities with their own `Hitbox` and `Hurtbox`.
- [ ] **Visual and Audio Feedback:** Add a "Clang" sound effect and a visual cue when a parry is successful.
- [ ] **Testing:** Create test cases for parry timing and hitbox interactions.

### Phase 5: Wrapping up v1.0.0

- [ ] Prepare the project for open-source release along with a technical documentation.
- [ ] Document the `engine` API with examples for how to use it in `game`.
- [ ] Write a post-mortem blog post detailing the development process, challenges faced, and lessons learned.- [ ] Plan for future features (e.g., particle system, audio engine, networking) and create a roadmap for version 2.0.

## Media and Resources

![Crate Architecture](./media/crate_architecture.png)
![Gameloop](./media/game_loop.png)
![InputBuffer Logic](./media/inputBuffer_logic.png)
![Render Pipeline](./media/render_pipeline.png)

## Docs

- [Rust + WASM Book](https://rustwasm.github.io/docs/book/)
- [The Rust Book](https://doc.rust-lang.org/)
- [wgpu Docs](https://docs.rs/wgpu/latest/wgpu/)
- [Game Programming Patterns](http://gameprogrammingpatterns.com/)
- **Inspiration:** Hollow knight Series, Nine Sols, Sekiro, Ghostrunner, Katana Zero.
