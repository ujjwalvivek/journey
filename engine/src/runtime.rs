/**--------------------------------------------------------------------------------
*!  Cross-platform engine runtime — wGPU rendering loop with egui overlay.
*?  Handles both native (desktop) and WASM (web) targets. The core rendering
*?  pipeline (CPU noise → GPU texture → full-screen quad + egui overlay) is
*?  shared; only event-loop bootstrap and async GPU initialization differ.
*--------------------------------------------------------------------------------**/
use crate::GameApp;
use crate::camera::Camera;
use crate::context::Context;
use crate::noise;
use crate::scene::SceneParams;
use crate::sprite::SpriteRenderer;
use egui_wgpu::ScreenDescriptor;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(not(target_arch = "wasm32"))]
use winit::window::Fullscreen;
use winit::window::{Window, WindowAttributes, WindowId};

//? Internal simulation resolution decoupled from the actual window/canvas size, CPU noise pass stays cheap.
//* Upscaled with nearest-neighbor filtering for a retro pixelated look.
const SIM_WIDTH: u32 = 32;
const SIM_HEIGHT: u32 = 32;

const NOISE_REGEN_INTERVAL: Duration = Duration::from_millis(16); //* ~60 Hz

//? On WASM, async GPU init completes after `resumed` returns. The spawned
//? future writes into this thread-local; the event handler picks it up on
//? the next frame. We use Box<dyn Any> to store the type-erased EngineState<G>.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_STATE: std::cell::RefCell<Option<Box<dyn std::any::Any>>> =
        const { std::cell::RefCell::new(None) };
}

//? Launch the engine event loop. Blocks on native; non-blocking on WASM.
pub fn start<G: GameApp>() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::<G>::default();
        if let Err(e) = event_loop.run_app(&mut app) {
            log::error!("Event loop exited with error: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let app = App::<G>::default();
        event_loop.spawn_app(app);
    }
}

//? Application shell: winit ApplicationHandler
//? Phantom data means the App struct is generic over G without actually storing a G value.
struct App<G: GameApp> {
    state: Option<EngineState<G>>,
    init_started: bool,
    _phantom: PhantomData<G>,
}

impl<G: GameApp> Default for App<G> {
    fn default() -> Self {
        Self {
            state: None,
            init_started: false,
            _phantom: PhantomData,
        }
    }
}

impl<G: GameApp> ApplicationHandler for App<G> {
    //? On both native and WASM, `resumed` is called once after the event loop starts.
    //* On native, synchronously create the EngineState and store it directly.
    //* On WASM, start the async GPU init and return immediately.
    //* EngineState value is written into a thread-local when done, which is picked up in the next `window_event` call.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.init_started {
            return;
        }
        self.init_started = true;

        #[cfg(not(target_arch = "wasm32"))]
        let attrs = WindowAttributes::default()
            .with_title("Journey Engine")
            .with_resizable(false)
            .with_visible(false)
            .with_fullscreen(Some(Fullscreen::Borderless(None)));

        #[cfg(target_arch = "wasm32")]
        let attrs = WindowAttributes::default().with_title("Journey Engine");

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(img) = image::load_from_memory(include_bytes!("../../web/public/favicon.png"))
            {
                let img = img.to_rgba8();
                let (w, h) = img.dimensions();
                let rgba = img.into_raw();
                if let Ok(icon) = winit::window::Icon::from_rgba(rgba, w, h) {
                    window.set_window_icon(Some(icon));
                }
            }
            window.set_decorations(false);
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        //? Native GPU init via pollster block_on, then show the window once ready
        //? to avoid title bar flash and ensure the first frame renders immediately.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut state = pollster::block_on(EngineState::new(window));
            let _ = state.render();
            state.window.set_maximized(true);
            state.window.set_visible(true);
            self.state = Some(state);
        }

        //? Attach canvas to DOM via WASM, then async GPU init in a spawned future.
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;

            if let Some(canvas) = window.canvas() {
                let doc = web_sys::window()
                    .and_then(|w| w.document())
                    .expect("No document");
                let body = doc.body().expect("No body element");
                body.append_child(&canvas).expect("Failed to append canvas");
                let style = canvas.style();
                let _ = style.set_property("width", "100vw");
                let _ = style.set_property("height", "100vh");
                let _ = style.set_property("display", "block");
                let _ = canvas.set_attribute("tabindex", "0");
                let _ = canvas.focus();

                //? Explicitly size the canvas backing buffer to physical pixels.
                sync_canvas_backing_buffer(&canvas);
            }

            let win = window.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let state = EngineState::<G>::new(win.clone()).await;
                PENDING_STATE.with(|cell| {
                    *cell.borrow_mut() = Some(Box::new(state));
                });
                win.request_redraw();
            });
        }
    }

    //? Handle window events: input, resize, redraw requests, close
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        //? On WASM, check whether async init has delivered the state yet.
        #[cfg(target_arch = "wasm32")]
        if self.state.is_none() {
            PENDING_STATE.with(|cell| {
                if let Some(boxed) = cell.borrow_mut().take() {
                    if let Ok(state) = boxed.downcast::<EngineState<G>>() {
                        self.state = Some(*state);
                    }
                }
            });
            //? The canvas may have been laid out / resized while the async GPU
            //? init was in flight. Immediately sync the surface configuration
            //? so the first frame renders at the correct resolution.
            if let Some(state) = &mut self.state {
                let size = state.window.inner_size();
                state.resize(size);
                state.window.request_redraw();
            }
        }

        let Some(state) = &mut self.state else {
            return;
        };

        //? Always forward keyboard/mouse to game input BEFORE egui consumed check.
        match &event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                state.context.input.handle_key_event(key_event);
            }
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                let pressed = *button_state == winit::event::ElementState::Pressed;
                state.context.input.handle_mouse_button(*button, pressed);
            }
            _ => {}
        }

        //? Forward to egui and check if it wants to consume the event (eg. for UI interaction).
        //? If so, request a redraw to update the UI.
        let response = state.egui_state.on_window_event(&state.window, &event);
        if response.repaint {
            state.window.request_redraw();
        }

        //? Handle other window events (resize, redraw, close)
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                //? Ensure egui state is synced with new dimensions/DPI
                let scale_factor = state.window.scale_factor() as f32;
                state.egui_ctx.set_pixels_per_point(scale_factor);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                //? DPI changed (e.g., dragged to another monitor or DevTools device switch)
                state.egui_ctx.set_pixels_per_point(scale_factor as f32);
                let size = state.window.inner_size();
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

    //? On web, the event loop may sleep if no events occur. Continuously
    //? request redraws to keep the animation loop running (especially for
    //? animated fog). On native, VSync naturally paces the loop.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            //? On WASM, poll the canvas for size/DPR changes every frame.
            //? Catches iframe resizes and monitor DPR changes that winit may miss.
            #[cfg(target_arch = "wasm32")]
            {
                use winit::platform::web::WindowExtWebSys;
                if let Some(canvas) = state.window.canvas() {
                    if let Some(web_window) = web_sys::window() {
                        let dpr = web_window.device_pixel_ratio();
                        let css_w = canvas.client_width() as f64;
                        let css_h = canvas.client_height() as f64;
                        let phys_w = (css_w * dpr).round() as u32;
                        let phys_h = (css_h * dpr).round() as u32;

                        //? Skip resize if dimensions are too small (portrait warning showing)
                        const MIN_WIDTH: u32 = 320;
                        const MIN_HEIGHT: u32 = 240;

                        let dpr_changed = (dpr - state.scale_factor).abs() > 0.01;
                        let size_changed =
                            phys_w != state.config.width || phys_h != state.config.height;

                        if phys_w >= MIN_WIDTH
                            && phys_h >= MIN_HEIGHT
                            && (size_changed || dpr_changed)
                        {
                            canvas.set_width(phys_w);
                            canvas.set_height(phys_h);
                            state.resize(winit::dpi::PhysicalSize::new(phys_w, phys_h));
                            state.egui_ctx.set_pixels_per_point(dpr as f32);
                            state.window.request_redraw();
                        }
                    }
                }
            }

            state.window.request_redraw();
        }
    }
}

//? Engine state, GPU resources, egui, scene params
struct EngineState<G: GameApp> {
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
    camera: Camera,
    sprite_renderer: SpriteRenderer,
    texture_bind_groups: Vec<wgpu::BindGroup>, //* Index 0 = white pixel, 1-7 = game textures
    texture_sizes: Vec<(f32, f32)>,            //* Texture dimensions for UV calculation
    game: G,
    context: Context,
    params: SceneParams,
    prev_params: SceneParams,
    last_frame: Instant,
    last_noise_regen: Instant,
    noise_dirty: bool,
    scale_factor: f64,
    render_format: wgpu::TextureFormat,
}

//? Core engine state initialization:
//? Async GPU setup, resource creation, game init, initial noise bake.
impl<G: GameApp> EngineState<G> {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        //? Create wGPU instance, surface, adapter, device, and configure the surface for rendering.
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

        //? Configure surface with the adapter's preferred format and initial size.
        //? This also implicitly creates the swap chain.
        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0];
        let render_format = match surface_format {
            wgpu::TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8UnormSrgb,
            other => other,
        };
        let view_formats = if surface_format == render_format {
            vec![]
        } else {
            vec![render_format]
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats,
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        //? Create the noise texture and sampler.
        //* Nearest neighbor filtering preserves hard pixel edges for retro aesthetic.
        let noise_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Noise Texture"),
            size: wgpu::Extent3d {
                width: SIM_WIDTH,
                height: SIM_HEIGHT,
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
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        //? Create a bind group layout for the noise texture and sampler.
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

        //? Create a bind group for the noise texture and sampler.
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

        //? Create the render pipeline for drawing the full-screen quad with the noise texture.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fullscreen Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../assets/shaders/shader.wgsl").into()),
        });

        //? Create a pipeline layout that includes the bind group layout for the noise texture.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        //? Finally, create the render pipeline with the shader, pipeline layout, and surface format.
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
                    format: render_format,
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

        //? Initialize egui context, state, and renderer.
        let egui_ctx = egui::Context::default();

        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_gray(40);
        visuals.window_shadow = egui::Shadow::NONE;
        egui_ctx.set_visuals(visuals);

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            render_format,
            egui_wgpu::RendererOptions::default(),
        );

        //? Create the camera and sprite renderer for the game.
        let scale_factor = window.scale_factor();
        let (game_w, game_h) = game_dimensions(size, scale_factor);

        let camera = Camera::new(game_w, game_h);
        let sprite_renderer = SpriteRenderer::new(&device, &queue, render_format, &camera);

        //? Load game textures from embedded bytes and create bind groups for them.
        let idle_bytes = include_bytes!("../../game/assets/player/Knight(Idle).png");
        let run_bytes = include_bytes!("../../game/assets/player/Knight(Run).png");
        let jump_bytes = include_bytes!("../../game/assets/player/Knight(Jump).png");
        let fall_bytes = include_bytes!("../../game/assets/player/Knight(Fall).png");
        let attack_bytes = include_bytes!("../../game/assets/player/Knight(Attack).png");
        let block_bytes = include_bytes!("../../game/assets/player/Knight(Block).png");
        let roll_bytes = include_bytes!("../../game/assets/player/Knight(Roll).png");

        let textures = vec![
            crate::texture::Texture::from_bytes(&device, &queue, idle_bytes, Some("Knight Idle"))
                .expect("Failed to load idle"),
            crate::texture::Texture::from_bytes(&device, &queue, run_bytes, Some("Knight Run"))
                .expect("Failed to load run"),
            crate::texture::Texture::from_bytes(&device, &queue, jump_bytes, Some("Knight Jump"))
                .expect("Failed to load jump"),
            crate::texture::Texture::from_bytes(&device, &queue, fall_bytes, Some("Knight Fall"))
                .expect("Failed to load fall"),
            crate::texture::Texture::from_bytes(
                &device,
                &queue,
                attack_bytes,
                Some("Knight Attack"),
            )
            .expect("Failed to load attack"),
            crate::texture::Texture::from_bytes(&device, &queue, block_bytes, Some("Knight Block"))
                .expect("Failed to load block"),
            crate::texture::Texture::from_bytes(&device, &queue, roll_bytes, Some("Knight Roll"))
                .expect("Failed to load roll"),
        ];

        //? Create bind groups for each texture
        //* Index 0: placeholder and reserved for white pixel in renderer.
        //* Indices 1-7: game textures (idle, run, jump, fall, attack, block, roll)
        let mut texture_bind_groups = vec![];
        let mut texture_sizes = vec![];

        texture_bind_groups.push(sprite_renderer.create_texture_bind_group(&device, &textures[0]));
        texture_sizes.push((1.0, 1.0));
        for texture in &textures {
            let bind_group = sprite_renderer.create_texture_bind_group(&device, texture);
            texture_bind_groups.push(bind_group);
            texture_sizes.push((texture.width as f32, texture.height as f32));
        }

        //? Create the game instance, passing in a mutable reference to the context.
        let mut context = Context::new(game_w, game_h);
        let game = G::init(&mut context);

        //? Initial noise bake to populate the texture before the first frame renders.
        let params = SceneParams::default();
        let mut pixel_buffer = vec![0u8; (SIM_WIDTH * SIM_HEIGHT * 4) as usize];
        noise::render_scene_to_buffer(&mut pixel_buffer, SIM_WIDTH, SIM_HEIGHT, &params);
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
            camera,
            sprite_renderer,
            texture_bind_groups,
            texture_sizes,
            game,
            context,
            prev_params: params.clone(),
            params,
            last_frame: Instant::now(),
            last_noise_regen: Instant::now(),
            noise_dirty: false,
            scale_factor,
            render_format,
        }
    }

    //? Resize handler: reconfigure the surface and update camera and sprite renderer with new dimensions.
    //? Surface config always uses physical pixels. Camera and context use game_dimensions
    //? (physical on native, logical/CSS on WASM) for a consistent coordinate system.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        //? Prevent rendering at invalid dimensions (portrait/too narrow)
        //? This avoids corruption when the CSS warning overlay is showing
        const MIN_WIDTH: u32 = 320;
        const MIN_HEIGHT: u32 = 240;

        if new_size.width < MIN_WIDTH || new_size.height < MIN_HEIGHT {
            return;
        }

        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.scale_factor = self.window.scale_factor();
            let (game_w, game_h) = game_dimensions(new_size, self.scale_factor);

            self.camera.resize(game_w, game_h);
            self.sprite_renderer
                .update_camera(&self.queue, &self.camera);
            self.context.resize(game_w, game_h);
        }
    }

    //? Main render loop: handle input, update game, regenerate noise if needed, and draw the frame.
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let now = Instant::now();
        let raw_dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        //? Hitstop: Freeze game time during impact
        let dt = if self.context.hitstop_timer > 0.0 {
            self.context.hitstop_timer -= raw_dt;
            if self.context.hitstop_timer <= 0.0 {
                self.context.hitstop_timer = 0.0;
            }
            //? Reduce delta_time to near-zero during hitstop (5% for subtle drift)
            raw_dt * 0.05
        } else {
            raw_dt
        };

        //? Rebuild action state from raw inputs at frame start
        self.context.input.begin_frame(dt);

        //? Build the egui UI and detect discrete changes to scene parameters (excluding time).
        let mut params = self.params.clone();
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            crate::scene::show_ui(ctx, &mut params);
            self.game.ui(ctx, &mut params);
        });

        let ui_changed = params.background_color != self.params.background_color
            || params.seed != self.params.seed
            || params.fog_enabled != self.params.fog_enabled
            || params.fog_density != self.params.fog_density
            || params.fog_opacity != self.params.fog_opacity
            || params.fog_color != self.params.fog_color
            || params.fog_anim_speed != self.params.fog_anim_speed;

        self.params = params;
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        //? Advance fog animation time (separate from dirty-check)
        if self.params.fog_enabled && self.params.fog_anim_speed > 0.0 {
            self.params.time += dt;
        }

        if ui_changed {
            self.noise_dirty = true;
        }

        let fog_animating = self.params.fog_enabled && self.params.fog_anim_speed > 0.0;
        let regen_due = now.duration_since(self.last_noise_regen) >= NOISE_REGEN_INTERVAL;

        //? Dirty check: animations dont get interrupted by UI tweaks.
        if self.noise_dirty || (fog_animating && regen_due) {
            noise::render_scene_to_buffer(
                &mut self.pixel_buffer,
                SIM_WIDTH,
                SIM_HEIGHT,
                &self.params,
            );
            upload_noise_texture(&self.queue, &self.noise_texture, &self.pixel_buffer);
            self.prev_params = self.params.clone();
            self.noise_dirty = false;
            self.last_noise_regen = now;
        }

        //? Update game logic and prepare sprites based on the current context and parameters.
        self.context.delta_time = dt;
        self.context.clear_sprites();
        self.game.update(&mut self.context);
        self.game.render(&mut self.context);

        //? Update camera position based on context before rendering.
        self.camera.set_offset(self.context.camera_offset_x);
        self.sprite_renderer
            .update_camera(&self.queue, &self.camera);

        self.sprite_renderer
            .prepare(&self.queue, &self.context.sprite_batch, &self.texture_sizes);

        //? Tessellate egui shapes into GPU primitives,
        //? using the same pixels_per_point for correct scaling on high-DPI displays.
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        //? Create a screen descriptor for egui rendering, matching the surface size and pixels_per_point.
        let screen = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }

        //? Begin encoding commands for the frame.
        //* Multiple render passes: one for the full-screen noise quad, one for the sprites, and one for the egui overlay.
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

        //? Get the current frame's swap chain texture and create a view for rendering.
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });

        //? Pass 1: full-screen quad with noise texture
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

        //? Pass 2: sprite rendering
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sprite Pass"),
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

            self.sprite_renderer
                .render_multi(&mut pass, &self.queue, &self.texture_bind_groups);
        }

        //? Pass 3: egui overlay
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

        //? Free any egui textures that were marked for deletion in this frame.
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        //? Submit the command buffer to the GPU queue and present the frame.
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

//? Additional Helpers

//? Convert physical pixel size to game-logic dimensions.
fn game_dimensions(physical: winit::dpi::PhysicalSize<u32>, scale_factor: f64) -> (f32, f32) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = scale_factor;
        (physical.width as f32, physical.height as f32)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let s = scale_factor as f32;
        (physical.width as f32 / s, physical.height as f32 / s)
    }
}

//? On WASM, set the canvas backing buffer (width/height attributes) to match
//? the CSS layout size × devicePixelRatio.
#[cfg(target_arch = "wasm32")]
fn sync_canvas_backing_buffer(canvas: &web_sys::HtmlCanvasElement) {
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or(1.0);

    let css_w = canvas.client_width() as f64;
    let css_h = canvas.client_height() as f64;
    let phys_w = (css_w * dpr).round() as u32;
    let phys_h = (css_h * dpr).round() as u32;

    if phys_w > 0 && phys_h > 0 {
        canvas.set_width(phys_w);
        canvas.set_height(phys_h);
    }
}

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
            bytes_per_row: Some(4 * SIM_WIDTH),
            rows_per_image: Some(SIM_HEIGHT),
        },
        wgpu::Extent3d {
            width: SIM_WIDTH,
            height: SIM_HEIGHT,
            depth_or_array_layers: 1,
        },
    );
}
