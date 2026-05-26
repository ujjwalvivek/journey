/**--------------------------------------------------------------------------------
*!  Full-screen blit shader with a small bright-pass bloom composite.
*?  Keeps the source crisp via textureLoad, then adds a low-cost neighborhood glow.
*--------------------------------------------------------------------------------**/
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct BloomUniform {
    enabled_threshold_intensity_radius: vec4<f32>,
};

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;
@group(1) @binding(0) var<uniform> bloom: BloomUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );

    var out: VertexOutput;
    out.position = vec4<f32>(pos[vi], 0.0, 1.0);
    out.uv = vec2<f32>(
        (pos[vi].x + 1.0) * 0.5,
        (1.0 - pos[vi].y) * 0.5,
    );
    return out;
}

fn load_scene(coord: vec2<i32>, dims: vec2<i32>) -> vec3<f32> {
    let clamped = clamp(coord, vec2<i32>(0, 0), dims - vec2<i32>(1, 1));
    return textureLoad(t_scene, clamped, 0).rgb;
}

fn bright_part(color: vec3<f32>, threshold: f32) -> vec3<f32> {
    let brightness = max(max(color.r, color.g), color.b);
    let factor = max(brightness - threshold, 0.0) / max(1.0 - threshold, 0.001);
    return color * factor;
}

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4<f32> {
    let dims_u = textureDimensions(t_scene, 0);
    let dims = vec2<i32>(i32(dims_u.x), i32(dims_u.y));
    let coord = vec2<i32>(floor(frag.uv * vec2<f32>(dims)));
    let source = load_scene(coord, dims);

    let settings = bloom.enabled_threshold_intensity_radius;
    if settings.x < 0.5 {
        return vec4<f32>(source, 1.0);
    }

    let threshold = settings.y;
    let intensity = settings.z;
    let radius = max(i32(round(settings.w)), 1);

    var glow = bright_part(source, threshold) * 0.22;
    glow += bright_part(load_scene(coord + vec2<i32>( radius,  0), dims), threshold) * 0.10;
    glow += bright_part(load_scene(coord + vec2<i32>(-radius,  0), dims), threshold) * 0.10;
    glow += bright_part(load_scene(coord + vec2<i32>( 0,  radius), dims), threshold) * 0.10;
    glow += bright_part(load_scene(coord + vec2<i32>( 0, -radius), dims), threshold) * 0.10;
    glow += bright_part(load_scene(coord + vec2<i32>( radius,  radius), dims), threshold) * 0.07;
    glow += bright_part(load_scene(coord + vec2<i32>(-radius,  radius), dims), threshold) * 0.07;
    glow += bright_part(load_scene(coord + vec2<i32>( radius, -radius), dims), threshold) * 0.07;
    glow += bright_part(load_scene(coord + vec2<i32>(-radius, -radius), dims), threshold) * 0.07;
    glow += bright_part(load_scene(coord + vec2<i32>( radius * 2,  0), dims), threshold) * 0.05;
    glow += bright_part(load_scene(coord + vec2<i32>(-radius * 2,  0), dims), threshold) * 0.05;
    glow += bright_part(load_scene(coord + vec2<i32>( 0,  radius * 2), dims), threshold) * 0.05;
    glow += bright_part(load_scene(coord + vec2<i32>( 0, -radius * 2), dims), threshold) * 0.05;

    return vec4<f32>(source + glow * intensity, 1.0);
}
