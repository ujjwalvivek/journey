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

//? GPU blend mode for sprite rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Alpha,
    Additive,
}

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
    pub blend_mode: BlendMode,
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
            blend_mode: BlendMode::Alpha,
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

    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    //? Convert high-level Sprite to low-level SpriteInstance for rendering.
    //* Calculates UV coordinates based on source_rect and texture size,
    //* and applies horizontal flip by negating scale.
    fn to_instance(&self, texture_width: f32, texture_height: f32) -> SpriteInstance {
        let (uv_offset, uv_size) = if let Some(src) = self.source_rect {
            //? Convert pixel coordinates to UV (0.0-1.0)
            let u = src.x / texture_width;
            let v = src.y / texture_height;
            let uw = src.w / texture_width;
            let vh = src.h / texture_height;
            (Vec2::new(u, v), Vec2::new(uw, vh))
        } else {
            //* Use full texture
            (Vec2::ZERO, Vec2::ONE)
        };

        //? Flip horizontally by mirroring the UV: start sampling from the RIGHT edge
        //? of the frame and walk left (negative uv_size.x). This keeps scale always
        //? positive and position always top-left, so callers never need to pre-shift
        //? the anchor and thus eliminating the ghost/teleport double-offset problem.
        let (uv_offset, uv_size) = if self.flip_x {
            (
                Vec2::new(uv_offset.x + uv_size.x, uv_offset.y),
                Vec2::new(-uv_size.x, uv_size.y),
            )
        } else {
            (uv_offset, uv_size)
        };

        SpriteInstance::new(self.position, self.size, self.color, uv_offset, uv_size)
    }
}

//? Sprite rendering system.
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    default_texture: Texture,
    default_bind_group: wgpu::BindGroup,
    rect_instance_buffer: wgpu::Buffer,
    rect_instance_data: Vec<SpriteInstance>,
    sprite_instance_buffer: wgpu::Buffer,

    //? Pre-uploaded batch ranges: (texture_id, start_instance..end_instance)
    batch_ranges: Vec<(usize, std::ops::Range<u32>)>,
    additive_batch_ranges: Vec<(usize, std::ops::Range<u32>)>,

    //? Reusable per-frame texture batch map (avoids heap alloc each frame)
    texture_batches: Vec<Vec<SpriteInstance>>,
    additive_texture_batches: Vec<Vec<SpriteInstance>>,
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

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 8,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 32,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 40,
                    shader_location: 4,
                },
            ],
        };

        //? Additive blend pipeline: src color adds to dest (glow effects)
        let additive_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let additive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Additive Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(additive_blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
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
            additive_pipeline,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            default_texture,
            default_bind_group,
            rect_instance_buffer,
            rect_instance_data: Vec::with_capacity(MAX_SPRITES),
            sprite_instance_buffer,
            batch_ranges: Vec::new(),
            additive_batch_ranges: Vec::new(),
            texture_batches: Vec::new(),
            additive_texture_batches: Vec::new(),
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

    //? Prepare sprites for rendering: batch by texture and blend mode, upload once.
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        sprites: &[Sprite],
        texture_sizes: &[(f32, f32)],
    ) {
        self.rect_instance_data.clear();
        self.batch_ranges.clear();
        self.additive_batch_ranges.clear();

        //? Clear and reuse per-texture batch vectors (avoids HashMap alloc each frame)
        for batch in &mut self.texture_batches {
            batch.clear();
        }
        for batch in &mut self.additive_texture_batches {
            batch.clear();
        }

        //? Convert high-level Sprite definitions to low-level SpriteInstance data,
        //? partitioned by blend mode and then by texture ID.
        for sprite in sprites.iter().take(MAX_SPRITES) {
            if sprite.source_rect.is_some() {
                let (tex_width, tex_height) = texture_sizes
                    .get(sprite.texture_id)
                    .copied()
                    .unwrap_or((1.0, 1.0));

                let instance = sprite.to_instance(tex_width, tex_height);

                let batches = match sprite.blend_mode {
                    BlendMode::Alpha => &mut self.texture_batches,
                    BlendMode::Additive => &mut self.additive_texture_batches,
                };

                if sprite.texture_id >= batches.len() {
                    batches.resize_with(sprite.texture_id + 1, Vec::new);
                }
                batches[sprite.texture_id].push(instance);
            } else {
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

        //? Concatenate alpha batches, then additive batches, into one contiguous buffer
        let mut all_instances: Vec<SpriteInstance> = Vec::new();

        for (tex_id, instances) in self.texture_batches.iter().enumerate() {
            if instances.is_empty() {
                continue;
            }
            let start = all_instances.len() as u32;
            all_instances.extend_from_slice(instances);
            self.batch_ranges
                .push((tex_id, start..all_instances.len() as u32));
        }

        for (tex_id, instances) in self.additive_texture_batches.iter().enumerate() {
            if instances.is_empty() {
                continue;
            }
            let start = all_instances.len() as u32;
            all_instances.extend_from_slice(instances);
            self.additive_batch_ranges
                .push((tex_id, start..all_instances.len() as u32));
        }

        if !all_instances.is_empty() {
            queue.write_buffer(
                &self.sprite_instance_buffer,
                0,
                bytemuck::cast_slice(&all_instances),
            );
        }
    }

    //? Render all prepared sprites with multiple texture bind groups.
    //? Alpha-blended sprites first, then additive-blended sprites.
    //? All buffer uploads happen in prepare(); this only binds and draws.
    pub fn render_multi<'rpass>(
        &'rpass self,
        render_pass: &mut wgpu::RenderPass<'rpass>,
        bind_groups: &'rpass [wgpu::BindGroup],
    ) {
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        //? Draw rects with the 1x1 white pixel texture (always alpha blend)
        if !self.rect_instance_data.is_empty() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(1, &self.default_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.rect_instance_buffer.slice(..));
            render_pass.draw(0..6, 0..self.rect_instance_data.len() as u32);
        }

        //? Draw alpha-blended texture batches
        let has_alpha = !self.batch_ranges.is_empty();
        let has_additive = !self.additive_batch_ranges.is_empty();

        if has_alpha || has_additive {
            render_pass.set_vertex_buffer(0, self.sprite_instance_buffer.slice(..));
        }

        if has_alpha {
            render_pass.set_pipeline(&self.pipeline);

            for &(texture_id, ref range) in &self.batch_ranges {
                let bind_group = bind_groups
                    .get(texture_id)
                    .unwrap_or(&self.default_bind_group);

                render_pass.set_bind_group(1, bind_group, &[]);
                render_pass.draw(0..6, range.clone());
            }
        }

        //? Draw additive-blended texture batches (glow effects)
        if has_additive {
            render_pass.set_pipeline(&self.additive_pipeline);

            for &(texture_id, ref range) in &self.additive_batch_ranges {
                let bind_group = bind_groups
                    .get(texture_id)
                    .unwrap_or(&self.default_bind_group);

                render_pass.set_bind_group(1, bind_group, &[]);
                render_pass.draw(0..6, range.clone());
            }
        }
    }
}
