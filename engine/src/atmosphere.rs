/**--------------------------------------------------------------------------------
*!  Procedural atmosphere generation.
*?  Contains the sky gradient and Perlin fog overlay pipeline used by the engine
*?  render loop. Sky and fog are uploaded separately so the shader can sample
*?  sky linearly and fog with nearest filtering.
*--------------------------------------------------------------------------------**/
use crate::SceneParams;
use noise::{NoiseFn, Perlin};

//? Convert a hex color string input to an RGB tuple.
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

//? Smoothstep function for smooth transitions in fog density.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

//? Draw a vertical gradient from `top_rgb` to `bot_rgb` into the RGBA buffer.
//* Currently used to fill the background with a solid color by passing the same
//* color for both endpoints. Flexibility preserved for potential future gradient
//* backgrounds without changing the API.
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

fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] * (1.0 - t) + b[0] * t,
        a[1] * (1.0 - t) + b[1] * t,
        a[2] * (1.0 - t) + b[2] * t,
    ]
}

fn color_f32_to_byte(c: f32) -> u8 {
    (c * 255.0).clamp(0.0, 255.0).round() as u8
}

fn clear_rgba(buffer: &mut [u8], rgba: [u8; 4]) {
    for px in buffer.chunks_exact_mut(4) {
        px.copy_from_slice(&rgba);
    }
}

pub fn render_sky_to_buffer(buffer: &mut [u8], width: u32, height: u32, params: &SceneParams) {
    let sky = &params.sky;
    if !sky.enabled {
        let bg_rgb = color_f32_to_u8(params.background_color);
        draw_gradient(buffer, width, height, bg_rgb, bg_rgb);
        return;
    }

    let horizon_y = sky.horizon_y.clamp(0.05, 0.95);
    let horizon_width = sky.horizon_width.max(0.001);
    let horizon_glow = sky.horizon_glow.clamp(0.0, 1.0);

    for y in 0..height {
        let fy = if height <= 1 {
            0.0
        } else {
            y as f32 / (height - 1) as f32
        };
        let vertical = if fy <= horizon_y {
            let t = smoothstep(0.0, 1.0, fy / horizon_y);
            lerp_color(sky.top_color, sky.horizon_color, t)
        } else {
            let t = smoothstep(0.0, 1.0, (fy - horizon_y) / (1.0 - horizon_y));
            lerp_color(sky.horizon_color, sky.bottom_color, t)
        };

        let distance = (fy - horizon_y).abs();
        let horizon_mask = 1.0 - smoothstep(0.0, horizon_width, distance);
        let glow = [
            (sky.horizon_color[0] * 1.18).min(1.0),
            (sky.horizon_color[1] * 1.18).min(1.0),
            (sky.horizon_color[2] * 1.18).min(1.0),
        ];

        for x in 0..width {
            let color = lerp_color(vertical, glow, horizon_mask * horizon_glow);
            let idx = ((y * width + x) * 4) as usize;
            buffer[idx] = color_f32_to_byte(color[0]);
            buffer[idx + 1] = color_f32_to_byte(color[1]);
            buffer[idx + 2] = color_f32_to_byte(color[2]);
            buffer[idx + 3] = 255;
        }
    }
}

//? Apply Perlin noise-based fog to the buffer as an overlay.
//* RGB stores fog color, alpha stores fog coverage. The shader samples this with nearest filtering.
#[allow(clippy::too_many_arguments)]
pub fn render_fog_overlay(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    time: f32,
    density: f32,
    opacity: f32,
    perlin: &Perlin,
    seed: u32,
    fog_rgb: (u8, u8, u8),
    fog_anim_speed: f32,
) {
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
            let half_h = (height as f32) * 0.5;
            let vertical_t = ((half_h - y as f32) / half_h).clamp(0.0, 1.0);
            let vertical_mask = smoothstep(0.0, 1.0, vertical_t);
            let final_mask = fog_mask * vertical_mask;
            let idx = ((y * width + x) * 4) as usize;

            buffer[idx] = fog_rgb.0;
            buffer[idx + 1] = fog_rgb.1;
            buffer[idx + 2] = fog_rgb.2;
            buffer[idx + 3] = (final_mask * opacity * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
}

//? Render the fog overlay into a pre-allocated buffer.
//? Accepts an optional cached Perlin instance and rebuilds only when seed changes.
pub fn render_fog_to_buffer(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    params: &SceneParams,
    fog_noise_cache: &mut Option<(u32, Perlin)>,
) {
    clear_rgba(buffer, [0, 0, 0, 0]);
    if !params.fog_enabled {
        return;
    }

    if !matches!(fog_noise_cache, Some((s, _)) if *s == params.seed) {
        *fog_noise_cache = Some((params.seed, Perlin::new(params.seed)));
    }
    let Some((_, perlin)) = fog_noise_cache.as_ref() else {
        return;
    };

    let fog_rgb = color_f32_to_u8(params.fog_color);
    render_fog_overlay(
        buffer,
        width,
        height,
        params.time,
        params.fog_density,
        params.fog_opacity,
        perlin,
        params.seed,
        fog_rgb,
        params.fog_anim_speed,
    );
}

//? Render a CPU-composited atmosphere buffer for tests and non-GPU callers.
pub fn render_atmosphere_to_buffer(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    params: &SceneParams,
    fog_noise_cache: &mut Option<(u32, Perlin)>,
) {
    render_sky_to_buffer(buffer, width, height, params);
    let mut fog = vec![0u8; buffer.len()];
    render_fog_to_buffer(&mut fog, width, height, params, fog_noise_cache);
    for (dst, src) in buffer.chunks_exact_mut(4).zip(fog.chunks_exact(4)) {
        let alpha = src[3] as f32 / 255.0;
        for c in 0..3 {
            dst[c] =
                (dst[c] as f32 * (1.0 - alpha) + src[c] as f32 * alpha).clamp(0.0, 255.0) as u8;
        }
        dst[3] = 255;
    }
}

//? Helper to convert [f32; 3] color in 0.0..=1.0 range to (u8, u8, u8) in 0..=255 range.
fn color_f32_to_u8(c: [f32; 3]) -> (u8, u8, u8) {
    (
        (c[0] * 255.0).clamp(0.0, 255.0).round() as u8,
        (c[1] * 255.0).clamp(0.0, 255.0).round() as u8,
        (c[2] * 255.0).clamp(0.0, 255.0).round() as u8,
    )
}

//? Unit tests for atmosphere generation logic.
#[cfg(test)]
mod tests {
    use super::*;

    //* Tests for hex color parsing
    #[test]
    fn test_hex_to_rgb() {
        let result = hex_to_rgb("#FFD300");
        assert_eq!(result, (255, 211, 0));
    }

    //* Tests for smoothstep function edge cases
    #[test]
    fn test_smoothstep_edges() {
        assert!((smoothstep(0.0, 1.0, 0.0) - 0.0).abs() < f32::EPSILON);
        assert!((smoothstep(0.0, 1.0, 1.0) - 1.0).abs() < f32::EPSILON);
    }

    //* Tests for gradient drawing (solid color case)
    #[test]
    fn test_color_f32_to_u8() {
        assert_eq!(color_f32_to_u8([1.0, 0.827, 0.0]), (255, 211, 0));
    }

    //* Tests for rendering a scene to a buffer and checking dimensions and basic content
    #[test]
    fn test_render_atmosphere_to_buffer_dimensions() {
        let params = SceneParams::default();
        let (w, h) = (4, 4);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let mut cache = None;
        render_atmosphere_to_buffer(&mut buf, w, h, &params, &mut cache);
        assert!(
            buf.iter().any(|&b| b != 0),
            "Buffer should contain non-zero pixels"
        );
    }

    //* Tests for fog application logic by checking that the top half of the buffer is modified
    //* while the bottom half remains unchanged. Uses a high fog density and opacity to ensure visible changes.
    #[test]
    fn test_fog_top_half_only() {
        let params = SceneParams {
            background_color: [1.0, 1.0, 1.0],
            fog_color: [0.0, 0.0, 0.0],
            fog_enabled: true,
            fog_density: 20.0,
            fog_opacity: 1.0,
            fog_anim_speed: 0.0,
            sky: crate::SkyParams {
                enabled: false,
                ..Default::default()
            },
            seed: 0,
            time: 0.0,
        };

        let (w, h) = (8u32, 8u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let mut cache = None;
        render_atmosphere_to_buffer(&mut buf, w, h, &params, &mut cache);

        let (bg_r, bg_g, bg_b) = color_f32_to_u8(params.background_color);
        let half = (h / 2) as usize;

        for y in half..(h as usize) {
            for x in 0..(w as usize) {
                let idx = (((y as u32) * w + x as u32) * 4) as usize;
                assert_eq!(buf[idx], bg_r);
                assert_eq!(buf[idx + 1], bg_g);
                assert_eq!(buf[idx + 2], bg_b);
            }
        }

        let mut top_changed = false;
        for y in 0..half {
            for x in 0..(w as usize) {
                let idx = (((y as u32) * w + x as u32) * 4) as usize;
                if buf[idx] != bg_r || buf[idx + 1] != bg_g || buf[idx + 2] != bg_b {
                    top_changed = true;
                    break;
                }
            }
            if top_changed {
                break;
            }
        }
        assert!(top_changed, "Top half should contain fog pixels");
    }

    #[test]
    fn test_sky_replaces_background_color() {
        let params = SceneParams {
            background_color: [1.0, 0.0, 0.0],
            fog_enabled: false,
            sky: crate::SkyParams {
                enabled: true,
                top_color: [0.0, 0.0, 0.0],
                horizon_color: [0.0, 0.0, 0.0],
                bottom_color: [0.0, 0.0, 0.0],
                ..Default::default()
            },
            ..Default::default()
        };

        let (w, h) = (8u32, 8u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let mut cache = None;
        render_atmosphere_to_buffer(&mut buf, w, h, &params, &mut cache);

        assert_ne!(buf[0], 255, "Sky should not blend over background red");
    }

    #[test]
    fn test_fog_applies_over_sky() {
        let params = SceneParams {
            background_color: [1.0, 1.0, 1.0],
            fog_enabled: true,
            fog_color: [1.0, 1.0, 1.0],
            fog_density: 20.0,
            fog_opacity: 1.0,
            sky: crate::SkyParams {
                enabled: true,
                top_color: [0.0, 0.0, 0.0],
                horizon_color: [0.0, 0.0, 0.0],
                bottom_color: [0.0, 0.0, 0.0],
                ..Default::default()
            },
            ..Default::default()
        };

        let (w, h) = (8u32, 8u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let mut cache = None;
        render_atmosphere_to_buffer(&mut buf, w, h, &params, &mut cache);

        assert!(
            buf.chunks_exact(4)
                .any(|px| px[0] > 2 || px[1] > 2 || px[2] > 2),
            "Fog should be able to apply over sky mode"
        );
    }
}
