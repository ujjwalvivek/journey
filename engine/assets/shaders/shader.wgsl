/**--------------------------------------------------------------------------------
*!  Full-screen triangle shader for displaying CPU-generated atmosphere textures.
*?  Uses 3 vertices (no vertex buffer) to cover clip space with a single triangle.
*--------------------------------------------------------------------------------**/
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

//? Simple vertex shader that generates a full-screen triangle and passes UVs to the fragment shader.
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

@group(0) @binding(0) var t_sky: texture_2d<f32>;
@group(0) @binding(1) var s_sky: sampler;
@group(0) @binding(2) var t_fog: texture_2d<f32>;
@group(0) @binding(3) var s_fog: sampler;

//? Sky samples linearly while fog samples nearest in its own texture.
@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4<f32> {
    let sky = textureSample(t_sky, s_sky, frag.uv);
    let fog = textureSample(t_fog, s_fog, frag.uv);
    return vec4<f32>(mix(sky.rgb, fog.rgb, fog.a), 1.0);
}
