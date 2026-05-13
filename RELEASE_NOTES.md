## Journey Engine v1.2.1 - Release Notes

Focus on audio generation and WASM size.

- Game crate now depends on `resonance` and `cadence` for sound generation.
- Replaced large embedded tracks with procedural synth audio made using `resonance` and `cadence`. 
- Added two new procedural soundtracks
- WASM Size dropped from **32.8MB down to 4.2MB**
- Added engine support for building StaticSoundData from generated mono samples.