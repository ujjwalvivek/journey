/**--------------------------------------------------------------------------------
*!  Sprite shader with sprite sheet support.
*?  Supports UV coordinates for rendering sub-regions of textures.
*--------------------------------------------------------------------------------**/
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(1) @binding(1)
var sprite_sampler: sampler;

//? Instance data (per sprite)
struct InstanceInput {
    @location(0) position: vec2<f32>,    //* World position (pixels)
    @location(1) scale: vec2<f32>,       //* Width/height in pixels
    @location(2) color: vec4<f32>,       //* Tint color
    @location(3) uv_offset: vec2<f32>,   //* UV top-left (0.0-1.0)
    @location(4) uv_size: vec2<f32>,     //* UV width/height (0.0-1.0)
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

//? Vertex shader: procedurally generate a quad (6 vertices per instance)
@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    //? Generate quad vertices (two triangles: 0,1,2 and 2,1,3)
    //* Local UV coordinates for each corner (0.0-1.0)
    var local_uv: vec2<f32>;
    switch vertex_idx {
        case 0u: { local_uv = vec2<f32>(0.0, 0.0); }  //* Top-left
        case 1u: { local_uv = vec2<f32>(1.0, 0.0); }  //* Top-right
        case 2u: { local_uv = vec2<f32>(0.0, 1.0); }  //* Bottom-left
        case 3u: { local_uv = vec2<f32>(0.0, 1.0); }  //* Bottom-left
        case 4u: { local_uv = vec2<f32>(1.0, 0.0); }  //* Top-right
        default: { local_uv = vec2<f32>(1.0, 1.0); }  //* Bottom-right
    }

    //? Convert local UV to world position
    let vertex_pos = instance.position + local_uv * instance.scale;

    //? Map local UV to sprite sheet region
    let tex_coords = instance.uv_offset + local_uv * instance.uv_size;

    //? Transform to clip space
    out.clip_position = camera.view_proj * vec4<f32>(vertex_pos, 0.0, 1.0);
    out.tex_coords = tex_coords;
    out.color = instance.color;

    return out;
}

//? Fragment shader: sample texture and multiply by tint color
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(sprite_texture, sprite_sampler, in.tex_coords);
    return tex_color * in.color;
}
