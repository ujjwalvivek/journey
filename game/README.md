# Journey Engine: WASM Compilation

WebAssembly build of **Journey**, a custom 2D Metroidvania game engine built from scratch in Rust. By hosting the compiled `.wasm` binaries on the global CDN, the engine's Git repository remains 100% clean of binary bloat while allowing frontend wrappers to time-travel between engine versions dynamically.

You can read the full technical breakdown of this engine's development at [ujjwalvivek.com](https://ujjwalvivek.com/blog).

## Tech Stack
* **Core Engine:** Rust
* **Compilation Target:** WebAssembly (`wasm32-unknown-unknown`)
* **Web Integration:** `wasm-bindgen`, `web-sys`
* **Audio:** Kira (Cross-platform audio synthesis)
* **Frontend Wrapper:** TypeScript, Vite

## How to Load This Package
If you want to run this specific historical version of the engine in a web project without installing local binaries, you can fetch it dynamically at runtime using an ES module import from the jsDelivr CDN.

```typescript
async function journeyEngine() {
    const version = "${version}"; //? This should match the version in package.json
    const cdnUrl = `https://cdn.jsdelivr.net/npm/@ujjwalvivek/journey-engine@${version}/game.js`;
    
    try {
        //? Fetch and initialize the compiled Rust engine directly from the CDN
        const module = await import(/* @vite-ignore */ cdnUrl);
        const init = module.default;
        
        await init();
        console.log(`Journey Engine v${version} booted successfully!`);
    } catch (error) {
        console.error("Failed to load Engine from CDN:", error);
    }
}

journeyEngine();
```