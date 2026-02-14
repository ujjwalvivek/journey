//! Native engine runtime — wGPU rendering loop with egui overlay.
//!
//! This module is only compiled on non-WASM targets. It owns the window,
//! GPU resources, and the egui integration, driving the two-pass render
//! pipeline (world quad + UI overlay) described in the TDD.

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui_wgpu::ScreenDescriptor;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::noise;
use crate::scene::SceneParams;

const NOISE_WIDTH: u32 = 512;
const NOISE_HEIGHT: u32 = 512;

/// Minimum interval between CPU noise regenerations for animated fog.
const NOISE_REGEN_INTERVAL: Duration = Duration::from_millis(33); // ~30 Hz

/// Launch the native event loop. Blocks until the window is closed.
pub fn start() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::default();
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop exited with error: {e}");
    }
}

// ---------------------------------------------------------------------------
// Application shell (winit ApplicationHandler)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct App {
    state: Option<EngineState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Journey Engine")
            .with_inner_size(LogicalSize::new(1280, 720));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        let state = pollster::block_on(EngineState::new(window));
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };

        let response = state.egui_state.on_window_event(&state.window, &event);
        if response.repaint {
            state.window.request_redraw();
        }
        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                match state.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("GPU out of memory, exiting");
                        event_loop.exit();
                    }
                    Err(e) => log::warn!("Surface error: {e:?}"),
                }
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Engine state — GPU resources, egui, scene params
// ---------------------------------------------------------------------------

struct EngineState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    pipeline: wgpu::RenderPipeline,
    noise_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    pixel_buffer: Vec<u8>,

    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    params: SceneParams,
    prev_params: SceneParams,
    last_frame: Instant,
    last_noise_regen: Instant,
    noise_dirty: bool,
}

impl EngineState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // --- wGPU bootstrap ---------------------------------------------------
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        log::info!("GPU adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Journey Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .expect("Failed to create GPU device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // --- Noise texture ----------------------------------------------------
        let noise_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Noise Texture"),
            size: wgpu::Extent3d {
                width: NOISE_WIDTH,
                height: NOISE_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = noise_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // --- Bind group -------------------------------------------------------
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Noise Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // --- Shader + pipeline ------------------------------------------------
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fullscreen Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fullscreen Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
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

        // --- egui integration -------------------------------------------------
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());

        // --- Initial noise bake -----------------------------------------------
        let params = SceneParams::default();
        let mut pixel_buffer = vec![0u8; (NOISE_WIDTH * NOISE_HEIGHT * 4) as usize];
        noise::render_scene_to_buffer(&mut pixel_buffer, NOISE_WIDTH, NOISE_HEIGHT, &params);
        upload_noise_texture(&queue, &noise_texture, &pixel_buffer);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            noise_texture,
            bind_group,
            pixel_buffer,
            egui_ctx,
            egui_state,
            egui_renderer,
            prev_params: params.clone(),
            params,
            last_frame: Instant::now(),
            last_noise_regen: Instant::now(),
            noise_dirty: false,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        // --- egui frame -------------------------------------------------------
        let mut params = self.params.clone();
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            build_ui(ctx, &mut params);
        });

        // Detect discrete UI changes (compare everything except time)
        let ui_changed = params.top_color != self.params.top_color
            || params.bottom_color != self.params.bottom_color
            || params.seed != self.params.seed
            || params.fog_enabled != self.params.fog_enabled
            || params.fog_density != self.params.fog_density
            || params.fog_opacity != self.params.fog_opacity
            || params.fog_color != self.params.fog_color
            || params.fog_anim_speed != self.params.fog_anim_speed;

        self.params = params;

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        // Advance fog animation time (separate from dirty-check)
        if self.params.fog_enabled && self.params.fog_anim_speed > 0.0 {
            self.params.time += dt;
        }

        if ui_changed {
            self.noise_dirty = true;
        }

        // --- Regenerate noise (throttled to ~30 Hz for animated fog) ----------
        let fog_animating = self.params.fog_enabled && self.params.fog_anim_speed > 0.0;
        let regen_due = now.duration_since(self.last_noise_regen) >= NOISE_REGEN_INTERVAL;

        if self.noise_dirty || (fog_animating && regen_due) {
            noise::render_scene_to_buffer(
                &mut self.pixel_buffer,
                NOISE_WIDTH,
                NOISE_HEIGHT,
                &self.params,
            );
            upload_noise_texture(&self.queue, &self.noise_texture, &self.pixel_buffer);
            self.prev_params = self.params.clone();
            self.noise_dirty = false;
            self.last_noise_regen = now;
        }

        // --- Tessellate egui --------------------------------------------------
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &clipped_primitives,
            &screen,
        );

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Pass 1: full-screen quad with noise texture
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("World Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Pass 2: egui overlay
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &clipped_primitives,
                &screen,
            );
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn upload_noise_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, data: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * NOISE_WIDTH),
            rows_per_image: Some(NOISE_HEIGHT),
        },
        wgpu::Extent3d {
            width: NOISE_WIDTH,
            height: NOISE_HEIGHT,
            depth_or_array_layers: 1,
        },
    );
}

fn build_ui(ctx: &egui::Context, params: &mut SceneParams) {
    egui::Window::new("Journey Controls").show(ctx, |ui| {
        ui.heading("Sky Gradient");
        ui.horizontal(|ui| {
            ui.label("Top Color");
            ui.color_edit_button_rgb(&mut params.top_color);
        });
        ui.horizontal(|ui| {
            ui.label("Bottom Color");
            ui.color_edit_button_rgb(&mut params.bottom_color);
        });

        ui.separator();
        ui.heading("Fog");
        ui.checkbox(&mut params.fog_enabled, "Enable Fog");
        if params.fog_enabled {
            ui.add(egui::Slider::new(&mut params.fog_density, 0.5..=10.0).text("Density"));
            ui.add(egui::Slider::new(&mut params.fog_opacity, 0.0..=1.0).text("Opacity"));
            ui.add(egui::Slider::new(&mut params.seed, 0..=999).text("Seed"));
            ui.horizontal(|ui| {
                ui.label("Fog Color");
                ui.color_edit_button_rgb(&mut params.fog_color);
            });
            ui.add(
                egui::Slider::new(&mut params.fog_anim_speed, 0.0..=2.0).text("Animation Speed"),
            );
        }
    });
}
