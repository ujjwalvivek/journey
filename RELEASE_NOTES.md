## Journey Engine v1.4.1 - Release Notes

A small fix for web scenes that run all the time, such as a page header.

- `target_fps` now caps the frame rate on the web too. Before, wasm drew on every animation frame whatever it was set to, so 120 frames a second on a 120 Hz screen. The default of 60 now applies on the web; set 0 for uncapped.
- The canvas size is checked every 30th frame instead of every frame. Reading it forced the browser to lay out the page each frame.
