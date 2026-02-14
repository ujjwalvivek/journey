//! Procedural noise generation and scene rendering.
//!
//! Contains the legacy gradient + Perlin fog pipeline, preserved from the
//! original WASM prototype. [`render_scene`] remains exported via `wasm-bindgen`
//! for the web frontend. [`render_scene_to_buffer`] provides a zero-allocation
//! variant for the native render loop.

use noise::{NoiseFn, Perlin};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::scene::SceneParams;

pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return (0, 0, 0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn draw_gradient(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    top_rgb: (u8, u8, u8),
    bot_rgb: (u8, u8, u8),
) {
    for y in 0..height {
        let t = y as f32 / height as f32;
        let r = top_rgb.0 as f32 * (1.0 - t) + bot_rgb.0 as f32 * t;
        let g = top_rgb.1 as f32 * (1.0 - t) + bot_rgb.1 as f32 * t;
        let b = top_rgb.2 as f32 * (1.0 - t) + bot_rgb.2 as f32 * t;
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            buffer[idx] = r as u8;
            buffer[idx + 1] = g as u8;
            buffer[idx + 2] = b as u8;
            buffer[idx + 3] = 255;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_fog(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    time: f32,
    density: f32,
    opacity: f32,
    seed: u32,
    fog_rgb: (u8, u8, u8),
    fog_anim_speed: f32,
) {
    let perlin = Perlin::new(seed);
    for y in 0..height {
        for x in 0..width {
            let nx = x as f64 / width as f64;
            let ny = y as f64 / height as f64;
            let base = perlin.get([
                nx * density as f64 * 0.8 + time as f64 * fog_anim_speed as f64,
                ny * density as f64 * 0.4,
                seed as f64,
            ]);
            let detail = perlin.get([
                nx * density as f64 * 1.5 + 100.0,
                ny * density as f64 * 0.7 + 50.0,
                seed as f64 + 42.0,
            ]);
            let fog = (base * 0.6 + detail * 0.4 + 1.0) * 0.5;
            let fog_mask = smoothstep(0.2, 0.8, fog as f32);

            let idx = ((y * width + x) * 4) as usize;
            for c in 0..3 {
                let orig = buffer[idx + c] as f32;
                let fogc = [fog_rgb.0, fog_rgb.1, fog_rgb.2][c] as f32;
                buffer[idx + c] = (orig * (1.0 - fog_mask * opacity) + fogc * fog_mask * opacity)
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Legacy WASM-exported scene renderer. Allocates a new buffer each call.
#[allow(clippy::too_many_arguments)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn render_scene(
    width: u32,
    height: u32,
    time: f32,
    quality: u32,
    top_hex: &str,
    bot_hex: &str,
    fog_on: bool,
    fog_density: f32,
    fog_opacity: f32,
    fog_seed: u32,
    fog_hex: &str,
    fog_anim_speed: f32,
) -> Vec<u8> {
    let q = quality.max(1);
    let w = width / q;
    let h = height / q;
    let mut buffer = vec![0u8; (w * h * 4) as usize];

    let top_rgb = hex_to_rgb(top_hex);
    let bot_rgb = hex_to_rgb(bot_hex);
    draw_gradient(&mut buffer, w, h, top_rgb, bot_rgb);

    if fog_on {
        apply_fog(
            &mut buffer,
            w,
            h,
            time,
            fog_density,
            fog_opacity,
            fog_seed,
            hex_to_rgb(fog_hex),
            fog_anim_speed,
        );
    }
    buffer
}

/// WASM greeting for connectivity testing.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn greet() {
    web_sys::console::log_1(&"Hello from Rust WASM!".into());
}

/// Render the scene into a pre-allocated buffer (zero-allocation hot path).
pub fn render_scene_to_buffer(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    params: &SceneParams,
) {
    let top_rgb = color_f32_to_u8(params.top_color);
    let bot_rgb = color_f32_to_u8(params.bottom_color);
    draw_gradient(buffer, width, height, top_rgb, bot_rgb);

    if params.fog_enabled {
        let fog_rgb = color_f32_to_u8(params.fog_color);
        apply_fog(
            buffer,
            width,
            height,
            params.time,
            params.fog_density,
            params.fog_opacity,
            params.seed,
            fog_rgb,
            params.fog_anim_speed,
        );
    }
}

fn color_f32_to_u8(c: [f32; 3]) -> (u8, u8, u8) {
    (
        (c[0] * 255.0).clamp(0.0, 255.0) as u8,
        (c[1] * 255.0).clamp(0.0, 255.0) as u8,
        (c[2] * 255.0).clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgb() {
        let result = hex_to_rgb("#87ceeb");
        assert_eq!(result, (135, 206, 235));
    }

    #[test]
    fn test_smoothstep_edges() {
        assert!((smoothstep(0.0, 1.0, 0.0) - 0.0).abs() < f32::EPSILON);
        assert!((smoothstep(0.0, 1.0, 1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_color_f32_to_u8() {
        assert_eq!(color_f32_to_u8([1.0, 0.5, 0.0]), (255, 127, 0));
    }

    #[test]
    fn test_render_scene_to_buffer_dimensions() {
        let params = SceneParams::default();
        let (w, h) = (4, 4);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        render_scene_to_buffer(&mut buf, w, h, &params);
        assert!(buf.iter().any(|&b| b != 0), "Buffer should contain non-zero pixels");
    }
}
