## Journey Engine v1.4.0 - Release Notes

This update makes the engine lighter for web scenes that only draw, and steadier in browsers.

- egui, audio (Kira) and image loading are now optional features, all on by default, so existing games build unchanged.
- A web scene that needs none of them can set `default-features = false`. A hero scene measured 4.2 MB of wasm with them and 2.1 MB without (1.93 MB to 0.82 MB gzip); egui's bundled fonts alone were 1.4 MB.
- New `GameApp::scene` hook sets scene parameters every frame without egui. `ui` still works as before when the egui feature is on.
- New `engine::set_paused` for the web: a paused scene requests no animation frames, and resuming carries on where it stopped.
- When a browser exposes WebGPU but returns no adapter (blocklisted GPUs, headless browsers), the engine now falls back to WebGL2 instead of panicking.
- Cleared the lints and future-incompatibility warnings raised by newer Rust toolchains.
