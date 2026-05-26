# Journey Engine, with a Fast Momentum Metroidvania Tech Demo

[![NPM Version](https://img.shields.io/npm/v/@ujjwalvivek/journey-engine?style=for-the-badge&logo=npm&logoColor=white&label=npm)](https://www.npmjs.com/package/@ujjwalvivek/journey-engine)
[![Crates.io](https://img.shields.io/crates/v/journey-engine.svg?style=for-the-badge&logo=rust&logoColor=white&label=crate)](https://crates.io/crates/journey-engine)
[![Docs](https://img.shields.io/badge/docs-journey.ujjwalvivek.com-blue?style=for-the-badge&logo=rust&logoColor=white&label=docs)](https://docs.journey.ujjwalvivek.com)
[![Publish Status](https://img.shields.io/github/actions/workflow/status/ujjwalvivek/journey/publish.yml?style=for-the-badge&logo=githubactions&logoColor=white&label=Publish)](https://github.com/ujjwalvivek/journey/actions)
[![License](https://img.shields.io/github/license/ujjwalvivek/journey?style=for-the-badge&logo=github&logoColor=white&label=License)](LICENSE)

[![Yayy](./docs/assets/latest.gif)](https://ujjwalvivek.com/blog/)

## Architectural Summary

<table>
  <tr>
    <td align="right"><img src="https://img.shields.io/badge/CORE_STACK-000000?style=for-the-badge" alt="Core Stack"/></td>
    <td>
      <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-FFFFFF?style=for-the-badge&logo=rust&logoColor=000000" alt="Rust"/></a>
      <a href="https://webassembly.org/"><img src="https://img.shields.io/badge/WebAssembly-FFFFFF?style=for-the-badge&logo=webassembly&logoColor=000000" alt="WebAssembly"/></a>
      <a href="https://wgpu.rs/"><img src="https://img.shields.io/badge/wgpu-FFFFFF?style=for-the-badge&logo=wgpu&logoColor=000000" alt="wgpu"/></a>
      <a href="https://www.typescriptlang.org/"><img src="https://img.shields.io/badge/TypeScript-FFFFFF?style=for-the-badge&logo=typescript&logoColor=000000" alt="TypeScript"/></a>
    </td>
  </tr>
  <tr>
    <td align="right"><img src="https://img.shields.io/badge/LIBRARIES-000000?style=for-the-badge" alt="Libraries"/></td>
    <td>
      <a href="https://docs.rs/kira/latest/kira/"><img src="https://img.shields.io/badge/Kira-FFFFFF?style=for-the-badge&logo=rust&logoColor=000000" alt="Kira"/></a>
      <a href="https://github.com/bitshifter/glam-rs"><img src="https://img.shields.io/badge/Glam-FFFFFF?style=for-the-badge&logo=rust&logoColor=000000" alt="Glam"/></a>
      <a href="https://docs.rs/pollster/latest/pollster/"><img src="https://img.shields.io/badge/Pollster-FFFFFF?style=for-the-badge&logo=rust&logoColor=000000" alt="Pollster"/></a>
      <a href="https://docs.rs/egui/latest/egui/"><img src="https://img.shields.io/badge/eGUI-FFFFFF?style=for-the-badge&logo=rust&logoColor=000000" alt="eGUI"/></a>
      <a href="https://docs.rs/winit/latest/winit/"><img src="https://img.shields.io/badge/Winit-FFFFFF?style=for-the-badge&logo=rust&logoColor=000000" alt="Winit"/></a>
      <a href="https://docs.rs/gilrs/latest/gilrs/"><img src="https://img.shields.io/badge/Gilrs-FFFFFF?style=for-the-badge&logo=rust&logoColor=000000" alt="Gilrs"/></a>
    </td>
  </tr>
  <tr>
    <td align="right"><img src="https://img.shields.io/badge/ARCHITECTURE-000000?style=for-the-badge" alt="Architecture"/></td>
    <td><img src="https://img.shields.io/badge/Data--Oriented_Trait--Driven_Fixed--Step_2D_Engine-FFFFFF?style=for-the-badge&logoColor=000000" alt="Data-Oriented"/></td>
  </tr>
  <tr>
    <td align="right"><img src="https://img.shields.io/badge/TARGET-000000?style=for-the-badge" alt="Target"/></td>
    <td>
      <img src="https://img.shields.io/badge/Native-FFFFFF?style=for-the-badge&logoColor=000000" alt="Native"/>
      <img src="https://img.shields.io/badge/WebAssembly-FFFFFF?style=for-the-badge&logoColor=000000" alt="WebAssembly"/>
    </td>
  </tr>
  <tr>
    <td align="right"><img src="https://img.shields.io/badge/STATUS-000000?style=for-the-badge" alt="Status"/></td>
    <td><img src="https://img.shields.io/badge/Phase_5:_MVP1_Complete-FFFFFF?style=for-the-badge&logoColor=000000" alt="Status"/></td>
  </tr>
  <tr>
    <td align="right"><img src="https://img.shields.io/badge/INSPIRATION-000000?style=for-the-badge" alt="Inspiration"/></td>
    <td>
      <img src="https://img.shields.io/badge/Nine_Sols-FFFFFF?style=for-the-badge&logoColor=000000" alt="Nine Sols"/>
      <img src="https://img.shields.io/badge/Sekiro-FFFFFF?style=for-the-badge&logoColor=000000" alt="Sekiro"/>
      <img src="https://img.shields.io/badge/Ghostrunner-FFFFFF?style=for-the-badge&logoColor=000000" alt="Ghostrunner"/>
      <img src="https://img.shields.io/badge/Katana_Zero-FFFFFF?style=for-the-badge&logoColor=000000" alt="Katana Zero"/>
      <img src="https://img.shields.io/badge/Hollow_Knight-FFFFFF?style=for-the-badge&logoColor=000000" alt="Hollow Knight"/>
    </td>
  </tr>
</table>

## A different take on Souls-like Metroidvania

A custom high-performance 2D game engine written in Rust + WGPU. Features AABB physics, focuses on precision platforming (*Hollow Knight*) and parry-based combat (*Sekiro*/*Nine Sols*) with a touch of a fast momentum based platformer (*Ghostrunner*). For a Metroidvania running at 60FPS (`Important Metric`) in a web browser, I want tight, deterministic, arcade physics, not realistic simulations. This project also serves as a "Living Proof of Work" for a **TPM** role.

It's all in the details. It’s not just about `Can I jump?` but `How does it feel to jump?` The secret sauce is in the mechanics that make the player feel powerful and responsive. Like coyote time, jump buffering, variable jump height, and a parry mechanic that rewards precise timing. The goal is to create a tech demo of the engine's capabilities by building a tight, fun, and responsive player controller that embodies the essence of a Fast Momentum Metroidvania gameplay.

## Cloning to Windows

1. Clone the repo with `git clone https://github.com/ujjwalvivek/journey.git`.
2. Install Rust, Node.js, and wasm-pack.
3. Press `Ctrl+Shift+B` → "Run Native (Release)".
4. Enjoy!

## Folder Architecture

The pipeline needs to be `Cross-Platform First`.

- The **"Renderer"** uses `wgpu` (which targets `Vulkan/Metal/DX12` on Desktop, and `WebGL/WebGPU` on Browser).
- The **"Input"** uses `winit` (which captures `Windows events` on Desktop, and `JS events` on Browser).

```bash
Journey/
├── Cargo.toml          //* Workspace definition
├── engine/             //* The reusable library (journey-engine on crates.io)
│   ├── src/lib.rs      //* Public API surface, GameApp trait, re-exports
│   └── Cargo.toml      //* Dependencies: wgpu, winit, kira, egui, glam, bytemuck
├── resonance/          //* The custom no_std DSP primitive (journey-sound)
├── cadence/            //* The procedural audio sequencer (journey-sequencer)
├── game/               //* The executable (Content)
│    ├── pkg/           //* WASM Artifacts (Generated here)
│    ├── src/main.rs    //* Native entry point
│    ├── src/lib.rs     //* JourneyGame: GameApp implementation
│    ├── src/input.rs   //* JourneyAction enum and key/gamepad bindings
│    └── Cargo.toml     //* Dependencies: journey-engine (aliased as engine)
└── web/                //* Vite Project for WASM distribution
    ├── src/            //* JavaScript Glue Code 
    ├── package.json    //* NPM Dependencies
    └── vite.config.ts  //* WASM Configuration

```

This forces a clean API from `engine` to `game`. It also allows the engine to be published as an independent crate (`journey-engine`) for reuse, testing, and documentation, and prevents spaghetti coupling. All crates are to be `WASM-compatible`. `Pollster` for async main, and `Kira` for audio. More have been mentioned in the badges above.

### Native: Build and Run

1. Make Rust changes in `engine/` or `game/`
2. Press `Ctrl+Shift+B` to build + run.

### Web: Build and Run

**Full rebuild (Rust + TS changes):**

1. Make changes in `engine/` or `web/src/`
2. Run task "Run Web Dev Server"
3. Vite will rebuild WASM + serve at localhost:5173

**Quick iteration (TS/HTML only):**

1. Make changes in `web/src/` (no Rust changes)
2. Run task "Quick Web Dev (skip WASM rebuild)"
3. Vite hot-reloads instantly

### Tips

- Rust Analyzer will run clippy automatically on save.
- Format on save is enabled for both Rust and TypeScript.
- `target/`, `node_modules/`, `dist/`, and `Cargo.lock` are hidden from file explorer.
- The default task (`Ctrl+Shift+B`) is "Run Native (Debug)" which is fastest for iteration.

> You might want to run "Run Native (Release)" for more accurate performance testing since debug builds can be very slow.

## Success Criteria (MVP)

- **Performance:** 60 FPS stable on both Desktop and Chrome/Firefox/Safari.
- **Size:** WASM binary under 5MB (gzip). Keep it lean. Hopeful.
- **Gameplay:** Fast Momentum Metroidvania feel.
- **Code Quality:** No `unwrap()` in the main loop and clean separation between Engine and Game.

## The Roadmap

### v1.0.0 Released (Check the [Release Notes](RELEASE_NOTES.md) for details)

### Interim

- [ ] Plan for future features and create a roadmap for MVP2.

## Media and Resources

![Crate Architecture](docs/assets/architecture.png)
![Render Pipeline](docs/assets/renderpipeline.png)

## Docs

- [Rust + WASM Book](https://rustwasm.github.io/docs/book/)
- [The Rust Book](https://doc.rust-lang.org/)
- [wgpu Docs](https://docs.rs/wgpu/latest/wgpu/)
- [Game Programming Patterns](http://gameprogrammingpatterns.com/)
- [Engine API Guide](https://github.com/ujjwalvivek/journey/blob/main/docs/src/ENGINE_API.md)
- [Technical Documentation](https://github.com/ujjwalvivek/journey/blob/main/docs/src/TECHNICAL_DOCUMENTATION.md)
- [Procedural Audio Documentation](https://github.com/ujjwalvivek/journey/blob/main/docs/src/AUDIO.md)
- [Post-Mortem Blog Post](https://ujjwalvivek.com/blog/)
