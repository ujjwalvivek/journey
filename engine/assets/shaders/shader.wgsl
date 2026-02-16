/**--------------------------------------------------------------------------------
*!  Full-screen triangle shader for displaying the CPU-generated noise texture.
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

@group(0) @binding(0) var t_noise: texture_2d<f32>;
@group(0) @binding(1) var s_noise: sampler;

//? Simple fragment shader that samples the noise texture using the UVs from the vertex shader.
@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_noise, s_noise, frag.uv);
}
