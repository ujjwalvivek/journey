/**--------------------------------------------------------------------------------
*!  2D sprite rendering system with sprite sheet support.
*?  Supports:
*?  - Instanced rendering for performance
*?  - Sprite sheets with UV coordinates
*?  - Color tinting
*?  - Multiple textures
*--------------------------------------------------------------------------------**/
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::texture::Texture;

const MAX_SPRITES: usize = 1024;

//? A rectangle defining a region (screen coordinates or sprite sheet).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    //? Create a rect from position and size vectors.
    pub fn from_pos_size(pos: Vec2, size: Vec2) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            w: size.x,
            h: size.y,
        }
    }
}

//? A sprite instance (position, size, color, UV).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteInstance {
    position: [f32; 2],
    scale: [f32; 2],
    color: [f32; 4],
    uv_offset: [f32; 2], //* Top-left UV
    uv_size: [f32; 2],   //* UV width/height
}

impl SpriteInstance {
    pub fn new(
        position: Vec2,
        scale: Vec2,
        color: [f32; 4],
        uv_offset: Vec2,
        uv_size: Vec2,
    ) -> Self {
        Self {
            position: position.to_array(),
            scale: scale.to_array(),
            color,
            uv_offset: uv_offset.to_array(),
            uv_size: uv_size.to_array(),
        }
    }
}

//? High-level sprite data (user-facing).
pub struct Sprite {
    pub position: Vec2,
    pub size: Vec2,
    pub color: [f32; 4],
    pub source_rect: Option<Rect>, //* Sprite sheet region (pixel coords)
    pub flip_x: bool,
    pub texture_id: usize, //* Which texture to use (0 = white pixel for rects)
}

//? User-facing sprite definition with various builder methods.
impl Sprite {
    pub fn new(position: Vec2, size: Vec2, color: [f32; 4]) -> Self {
        Self {
            position,
            size,
            color,
            source_rect: None,
            flip_x: false,
            texture_id: 0, //* Default to white pixel
        }
    }

    pub fn with_source(mut self, rect: Rect) -> Self {
        self.source_rect = Some(rect);
        self
    }

    pub fn with_flip(mut self, flip_x: bool) -> Self {
        self.flip_x = flip_x;
        self
    }

    pub fn with_texture_id(mut self, texture_id: usize) -> Self {
        self.texture_id = texture_id;
        self
    }

    //? Convert high-level Sprite to low-level SpriteInstance for rendering.
    //* Calculates UV coordinates based on source_rect and texture size,
    //* and applies horizontal flip by negating scale.
    fn to_instance(&self, texture_width: f32, texture_height: f32) -> SpriteInstance {
        let (uv_offset, uv_size) = if let Some(src) = self.source_rect {
            // Convert pixel coordinates to UV (0.0-1.0)
            let u = src.x / texture_width;
            let v = src.y / texture_height;
            let uw = src.w / texture_width;
            let vh = src.h / texture_height;
            (Vec2::new(u, v), Vec2::new(uw, vh))
        } else {
            //* Use full texture
            (Vec2::ZERO, Vec2::ONE)
        };

        //? Apply horizontal flip by negating scale
        let scale = if self.flip_x {
            Vec2::new(-self.size.x, self.size.y)
        } else {
            self.size
        };

        SpriteInstance::new(self.position, scale, self.color, uv_offset, uv_size)
    }
}

//? Sprite rendering system.
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    default_texture: Texture,
    default_bind_group: wgpu::BindGroup,
    rect_instance_buffer: wgpu::Buffer,
    rect_instance_data: Vec<SpriteInstance>,
    sprite_instance_buffer: wgpu::Buffer,
    sprite_instance_data: Vec<SpriteInstance>,
    texture_width: f32,
    texture_height: f32,

    //? Batching by texture ID
    texture_batches: std::collections::HashMap<usize, Vec<SpriteInstance>>,
}

impl SpriteRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        camera: &Camera,
    ) -> Self {
        //? Create default 1x1 white pixel texture
        let default_texture = Texture::white_pixel(device, queue);

        //? Camera uniform buffer
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*camera.uniform()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        //? Camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        //? Camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        //? Texture bind group layout (for both default and custom textures)
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sprite Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        //? Default bind group for the white pixel texture (used for rects)
        let default_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Default Sprite Texture Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&default_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&default_texture.sampler),
                },
            ],
        });

        //? Instance buffers
        let rect_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rect Instance Buffer"),
            size: (MAX_SPRITES * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Instance Buffer"),
            size: (MAX_SPRITES * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        //? Pipeline setup
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../assets/shaders/shader_sprite.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        //? Create render pipeline with instancing and texture support
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SpriteInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    //? The first 8 bytes of the SpriteInstance struct are interpreted as a vec2 for position,
                    //? the next 8 bytes as a vec2 for scale, the next 16 bytes as a vec4 for color, and so on.
                    //? The shader will read these attributes from the vertex buffer when rendering each instance.
                    attributes: &[
                        //* Position
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        //* Scale
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        //* Color
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 2,
                        },
                        //* UV Offset
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 32,
                            shader_location: 3,
                        },
                        //* UV Size
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 40,
                            shader_location: 4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            //? Fragment state with alpha blending for transparency
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            //? Primitive state for triangle list (two triangles per sprite)
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        //? Return the initialized SpriteRenderer
        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            default_texture,
            default_bind_group,
            rect_instance_buffer,
            rect_instance_data: Vec::with_capacity(MAX_SPRITES),
            sprite_instance_buffer,
            sprite_instance_data: Vec::with_capacity(MAX_SPRITES),
            texture_width: 1.0,
            texture_height: 1.0,
            texture_batches: std::collections::HashMap::new(),
        }
    }

    //? Create a bind group for a custom texture.
    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture: &Texture,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Custom Sprite Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            //? Bind the provided texture's view and sampler to the bind group for use in the shader.
            //? This allows the shader to sample from this texture when rendering sprites that use it.
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }

    //? Update camera uniform (call after camera.resize()).
    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[*camera.uniform()]),
        );
    }

    //? Set the active texture dimensions (for UV coordinate calculation).
    pub fn set_texture_size(&mut self, width: f32, height: f32) {
        self.texture_width = width;
        self.texture_height = height;
    }

    //? Prepare sprites for rendering, batching by texture ID.
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        sprites: &[Sprite],
        texture_sizes: &[(f32, f32)],
    ) {
        self.rect_instance_data.clear();
        self.sprite_instance_data.clear();
        self.texture_batches.clear();

        //? Convert high-level Sprite definitions to low-level SpriteInstance data and batch by texture ID.
        //* Sprites with a source_rect are considered textured and are batched by their texture_id,
        //* while sprites without a source_rect are treated as simple colored rectangles using the default white pixel texture.
        for sprite in sprites.iter().take(MAX_SPRITES) {
            if sprite.source_rect.is_some() {
                //? Textured sprite - batch by texture_id
                let (tex_width, tex_height) = texture_sizes
                    .get(sprite.texture_id)
                    .copied()
                    .unwrap_or((1.0, 1.0));

                let instance = sprite.to_instance(tex_width, tex_height);
                self.texture_batches
                    .entry(sprite.texture_id)
                    .or_default()
                    .push(instance);
            } else {
                //? Rect (no texture) - uses white pixel
                self.rect_instance_data.push(sprite.to_instance(1.0, 1.0));
            }
        }

        //? Upload rect instances
        if !self.rect_instance_data.is_empty() {
            queue.write_buffer(
                &self.rect_instance_buffer,
                0,
                bytemuck::cast_slice(&self.rect_instance_data),
            );
        }
    }

    //? Render all prepared sprites with multiple texture bind groups.
    //? bind_groups[0] should be white pixel (for rects), bind_groups[1..] are game textures.
    pub fn render_multi<'rpass>(
        &'rpass self,
        render_pass: &mut wgpu::RenderPass<'rpass>,
        queue: &wgpu::Queue,
        bind_groups: &'rpass [wgpu::BindGroup],
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        //? Draw rects with the 1x1 white pixel texture
        if !self.rect_instance_data.is_empty() {
            render_pass.set_bind_group(1, &self.default_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.rect_instance_buffer.slice(..));
            render_pass.draw(0..6, 0..self.rect_instance_data.len() as u32);
        }

        //? Draw each texture batch with its corresponding bind group
        for (&texture_id, instances) in &self.texture_batches {
            if instances.is_empty() {
                continue;
            }

            //? Upload instances for this texture
            queue.write_buffer(
                &self.sprite_instance_buffer,
                0,
                bytemuck::cast_slice(instances),
            );

            //? Get bind group for this texture (with bounds check)
            let bind_group = bind_groups
                .get(texture_id)
                .unwrap_or(&self.default_bind_group);

            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.sprite_instance_buffer.slice(..));
            render_pass.draw(0..6, 0..instances.len() as u32);
        }
    }

    //? Render all prepared sprites with the default texture (legacy).
    pub fn render<'rpass>(&'rpass self, render_pass: &mut wgpu::RenderPass<'rpass>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        if !self.rect_instance_data.is_empty() {
            render_pass.set_bind_group(1, &self.default_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.rect_instance_buffer.slice(..));
            render_pass.draw(0..6, 0..self.rect_instance_data.len() as u32);
        }
    }

    //? Render all prepared sprites with a custom texture bind group for textured sprites (legacy).
    pub fn render_split<'rpass>(
        &'rpass self,
        render_pass: &mut wgpu::RenderPass<'rpass>,
        texture_bind_group: &'rpass wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        if !self.rect_instance_data.is_empty() {
            render_pass.set_bind_group(1, &self.default_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.rect_instance_buffer.slice(..));
            render_pass.draw(0..6, 0..self.rect_instance_data.len() as u32);
        }

        if !self.sprite_instance_data.is_empty() {
            render_pass.set_bind_group(1, texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.sprite_instance_buffer.slice(..));
            render_pass.draw(0..6, 0..self.sprite_instance_data.len() as u32);
        }
    }
}
