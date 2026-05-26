/**--------------------------------------------------------------------------------
*!  Cross-platform engine runtime and wGPU rendering loop with egui overlay.
*?  Handles both native (desktop) and WASM (web) targets. The core rendering
*?  pipeline (CPU atmosphere → GPU textures → full-screen quad + egui overlay) is
*?  shared; only event-loop bootstrap and async GPU initialization differ.
*--------------------------------------------------------------------------------**/
use crate::GameApp;
use crate::atmosphere;
use crate::camera::Camera;
use crate::context::{Context, FrameStats};
use crate::sprite::SpriteRenderer;
use crate::time::FixedTime;
use crate::{BloomSettings, SceneParams};
use bytemuck::{Pod, Zeroable};
use egui_wgpu::ScreenDescriptor;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(not(target_arch = "wasm32"))]
use winit::window::Fullscreen;
use winit::window::{Window, WindowAttributes, WindowId};

//? Internal simulation resolution decoupled from the actual window/canvas size, CPU atmosphere pass stays cheap.
//* Upscaled with nearest-neighbor filtering for a retro pixelated look.
const SIM_WIDTH: u32 = 32;
const SIM_HEIGHT: u32 = 32;

//? Minimum window dimensions to prevent invalid rendering
const MIN_WIDTH: u32 = 320;
const MIN_HEIGHT: u32 = 240;

const ATMOSPHERE_REGEN_INTERVAL: Duration = Duration::from_millis(16); //* ~60 Hz

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BloomUniform {
    enabled_threshold_intensity_radius: [f32; 4],
}

impl From<BloomSettings> for BloomUniform {
    fn from(settings: BloomSettings) -> Self {
        Self {
            enabled_threshold_intensity_radius: [
                if settings.enabled { 1.0 } else { 0.0 },
                settings.threshold.clamp(0.0, 0.99),
                settings.intensity.max(0.0),
                settings.radius.clamp(1.0, 8.0),
            ],
        }
    }
}

#[derive(Clone, Copy)]
enum RenderPacing {
    SleepToCap,
    #[cfg(not(target_arch = "wasm32"))]
    Immediate,
}

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
            .with_title(G::window_title())
            .with_resizable(true)
            .with_visible(false);

        #[cfg(target_arch = "wasm32")]
        let attrs = WindowAttributes::default().with_title(G::window_title());

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(icon_bytes) = G::window_icon()
                && let Ok(img) = image::load_from_memory(icon_bytes)
            {
                let img = img.to_rgba8();
                let (w, h) = img.dimensions();
                let rgba = img.into_raw();
                if let Ok(icon) = winit::window::Icon::from_rgba(rgba, w, h) {
                    window.set_window_icon(Some(icon));
                }
            }
        }

        //? Native GPU init via pollster block_on, then show the window once ready
        //? to avoid title bar flash and ensure the first frame renders immediately.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut state = pollster::block_on(EngineState::new(window));
            state.apply_display_mode();
            let _ = state.render(RenderPacing::SleepToCap);
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

        #[cfg(not(target_arch = "wasm32"))]
        if matches!(event, WindowEvent::CursorMoved { .. }) {
            state.pending_cursor_moved = Some(event);
            if state.native_frame_due() {
                match state.render(RenderPacing::Immediate) {
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
            }
            if state.context.request_exit {
                event_loop.exit();
            }
            return;
        }

        //? Always forward keyboard/mouse to game input BEFORE egui consumed check.
        match &event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state == winit::event::ElementState::Pressed {
                    state.context.audio.notify_user_gesture();
                }
                state.context.input.handle_key_event(key_event);
            }
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                let pressed = *button_state == winit::event::ElementState::Pressed;
                if pressed {
                    state.context.audio.notify_user_gesture();
                }
                state.context.input.handle_mouse_button(*button, pressed);
            }
            WindowEvent::Touch(_) => {
                state.context.audio.notify_user_gesture();
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
                //? On native, rendering is driven from about_to_wait() to avoid
                //? WM_PAINT priority starvation during aggressive cursor movement.
                //? On WASM, RedrawRequested is driven by requestAnimationFrame
                //? which is already decoupled from the DOM event queue.
                #[cfg(target_arch = "wasm32")]
                {
                    match state.render(RenderPacing::SleepToCap) {
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
                }
            }
            _ => {}
        }

        if state.context.request_exit {
            event_loop.exit();
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

            //? On native, render directly here after all pending events have been
            //? drained. This bypasses WM_PAINT priority starvation that causes FPS
            //? drops during aggressive cursor movement on Windows. WM_PAINT is the
            //? lowest priority Win32 message and gets indefinitely delayed when
            //? WM_MOUSEMOVE floods the queue.
            #[cfg(not(target_arch = "wasm32"))]
            {
                if state.native_frame_due() {
                    match state.render(RenderPacing::SleepToCap) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            let size = state.window.inner_size();
                            state.resize(size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("GPU out of memory, exiting");
                            _event_loop.exit();
                        }
                        Err(e) => log::warn!("Surface error: {e:?}"),
                    }
                }
                if state.context.request_exit {
                    _event_loop.exit();
                }
            }

            //? Schedule next frame. On WASM this requests a requestAnimationFrame
            //? callback. On native this posts WM_PAINT to keep the event loop awake
            //? (the actual render already happened above, so RedrawRequested is a no-op).
            state.window.request_redraw();
            #[cfg(not(target_arch = "wasm32"))]
            {
                if state.context.target_fps > 0 {
                    let target_frame_time =
                        std::time::Duration::from_secs_f64(1.0 / state.context.target_fps as f64);
                    _event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                        std::time::Instant::now() + target_frame_time,
                    ));
                } else {
                    _event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                }
            }
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
    bloom_pipeline: wgpu::RenderPipeline,
    bloom_uniform_buffer: wgpu::Buffer,
    bloom_bind_group: wgpu::BindGroup,
    sky_texture: wgpu::Texture,
    fog_texture: wgpu::Texture,
    atmosphere_bind_group: wgpu::BindGroup,
    sky_buffer: Vec<u8>,
    fog_buffer: Vec<u8>,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    camera: Camera,
    sprite_renderer: SpriteRenderer,
    internal_width: u32,
    internal_height: u32,
    texture_bind_groups: Vec<wgpu::BindGroup>, //* Index 0 = white pixel, N = loaded textures
    texture_sizes: Vec<(f32, f32)>,            //* Texture dimensions for UV calculation
    game: G,
    context: Context<G::Action>,
    params: SceneParams,
    prev_params: SceneParams,
    last_frame: Instant,
    last_atmosphere_regen: Instant,
    atmosphere_dirty: bool,
    scale_factor: f64,
    render_format: wgpu::TextureFormat,
    fixed_time: FixedTime,
    atmosphere_noise_cache: Option<(u32, ::noise::Perlin)>,
    fps_samples: std::collections::VecDeque<f32>,
    pending_cursor_moved: Option<WindowEvent>,
    #[allow(dead_code)] //* Kept alive - GPU bind group references the underlying TextureView
    offscreen_texture: wgpu::Texture,
    offscreen_view: wgpu::TextureView,
    blit_bind_group: wgpu::BindGroup,
    #[cfg(target_arch = "wasm32")]
    first_frame_event_sent: bool,
}

//? Core engine state initialization:
//? Async GPU setup, resource creation, game init, initial atmosphere bake.
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
                label: Some("Engine Device"),
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

        //? Create separate sky and fog textures at the fixed atmosphere resolution.
        //* Sky is sampled linearly for blended gradients. Fog is sampled nearest for chunky retro fog.
        let sky_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sky Texture"),
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
        let fog_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Fog Texture"),
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

        let sky_texture_view = sky_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let fog_texture_view = fog_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let fog_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        //? Create a bind group layout for sky/fog atmosphere composition.
        let atmosphere_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Atmosphere Bind Group Layout"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let atmosphere_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atmosphere Bind Group"),
            layout: &atmosphere_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sky_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sky_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&fog_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&fog_sampler),
                },
            ],
        });

        //? Create a single-texture bind group layout for final blits.
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

        //? Create offscreen render target at internal resolution.
        //? All game passes (atmosphere + sprites) render here blitted to surface with nearest-neighbor.
        let (internal_w, internal_h) = G::internal_resolution();
        let offscreen_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Render Target"),
            size: wgpu::Extent3d {
                width: internal_w,
                height: internal_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: render_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let offscreen_view = offscreen_texture.create_view(&wgpu::TextureViewDescriptor::default());

        //? Blit bind group: samples offscreen texture with nearest-neighbor filtering.
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&offscreen_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        });

        //? Create the render pipeline for drawing the full-screen quad with atmosphere textures.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fullscreen Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../assets/shaders/shader.wgsl").into()),
        });
        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../assets/shaders/shader_bloom.wgsl").into(),
            ),
        });

        let bloom_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bloom Uniform Buffer"),
            contents: bytemuck::bytes_of(&BloomUniform::from(BloomSettings::default())),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bloom_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bloom Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let bloom_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bloom Bind Group"),
            layout: &bloom_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bloom_uniform_buffer.as_entire_binding(),
            }],
        });

        //? Create a pipeline layout that includes the sky/fog atmosphere bind group.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&atmosphere_bind_group_layout],
            push_constant_ranges: &[],
        });
        let bloom_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Bloom Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout, &bloom_bind_group_layout],
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
        let bloom_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bloom Blit Pipeline"),
            layout: Some(&bloom_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bloom_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bloom_shader,
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

        let ui_theme = crate::ui::theme();
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = ui_theme.panel;
        visuals.window_corner_radius = egui::CornerRadius::same(0);
        visuals.window_shadow = egui::Shadow::NONE;
        visuals.panel_fill = ui_theme.panel;
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

        //? Camera and context use the fixed internal resolution.
        let scale_factor = window.scale_factor();

        let camera = Camera::new(internal_w as f32, internal_h as f32);
        let sprite_renderer = SpriteRenderer::new(&device, &queue, render_format, &camera);

        //? Create the game instance, passing in a mutable reference to the context.
        //? The game queues texture loads via ctx.load_texture() during init.
        let mut context = Context::new(internal_w as f32, internal_h as f32);
        let game = G::init(&mut context);

        //? Process textures queued by the game during init
        let mut texture_bind_groups = vec![];
        let mut texture_sizes = vec![];

        //? Index 0: placeholder (white pixel fallback for bind group lookups)
        {
            let white_tex = crate::texture::Texture::white_pixel(&device, &queue);
            texture_bind_groups
                .push(sprite_renderer.create_texture_bind_group(&device, &white_tex));
            texture_sizes.push((1.0, 1.0));
        }

        //? Load each texture the game requested and create bind groups
        for pending in &context.pending_textures {
            let texture = crate::texture::Texture::from_bytes(
                &device,
                &queue,
                pending.bytes,
                Some(&pending.label),
            )
            .unwrap_or_else(|e| panic!("Failed to load texture '{}': {e}", pending.label));

            let bind_group = sprite_renderer.create_texture_bind_group(&device, &texture);
            texture_sizes.push((texture.width as f32, texture.height as f32));
            texture_bind_groups.push(bind_group);
        }
        context.pending_textures.clear();

        //? Initial atmosphere bake to populate textures before the first frame renders.
        let params = SceneParams::default();
        let mut sky_buffer = vec![0u8; (SIM_WIDTH * SIM_HEIGHT * 4) as usize];
        let mut fog_buffer = vec![0u8; (SIM_WIDTH * SIM_HEIGHT * 4) as usize];
        let mut atmosphere_noise_cache = None;
        atmosphere::render_sky_to_buffer(&mut sky_buffer, SIM_WIDTH, SIM_HEIGHT, &params);
        atmosphere::render_fog_to_buffer(
            &mut fog_buffer,
            SIM_WIDTH,
            SIM_HEIGHT,
            &params,
            &mut atmosphere_noise_cache,
        );
        upload_atmosphere_texture(&queue, &sky_texture, &sky_buffer);
        upload_atmosphere_texture(&queue, &fog_texture, &fog_buffer);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            bloom_pipeline,
            bloom_uniform_buffer,
            bloom_bind_group,
            sky_texture,
            fog_texture,
            atmosphere_bind_group,
            sky_buffer,
            fog_buffer,
            egui_ctx,
            egui_state,
            egui_renderer,
            camera,
            sprite_renderer,
            texture_bind_groups,
            texture_sizes,
            game,
            context,
            internal_width: internal_w,
            internal_height: internal_h,
            prev_params: params.clone(),
            params,
            last_frame: Instant::now(),
            last_atmosphere_regen: Instant::now(),
            atmosphere_dirty: false,
            scale_factor,
            render_format,
            fixed_time: FixedTime::default(),
            atmosphere_noise_cache,
            fps_samples: std::collections::VecDeque::with_capacity(120),
            pending_cursor_moved: None,
            offscreen_texture,
            offscreen_view,
            blit_bind_group,
            #[cfg(target_arch = "wasm32")]
            first_frame_event_sent: false,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_display_mode(&mut self) {
        if self.context.fullscreen_enabled {
            self.window.set_decorations(false);
            self.window.set_maximized(true);
            if self.context.hdr_enabled {
                self.window
                    .set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                self.window.set_fullscreen(None);
            }
        } else {
            self.window.set_fullscreen(None);
            self.window.set_decorations(true);
            self.window.set_maximized(false);
            self.context.hdr_enabled = false;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_requested_display_changes(&mut self) {
        let mut changed = false;

        if let Some(fullscreen) = self.context.request_fullscreen.take() {
            self.context.fullscreen_enabled = fullscreen;
            if !fullscreen {
                self.context.hdr_enabled = false;
            }
            changed = true;
        }

        if let Some(hdr) = self.context.request_hdr.take() {
            self.context.hdr_enabled = hdr;
            if hdr {
                self.context.fullscreen_enabled = true;
            }
            changed = true;
        }

        if changed {
            self.apply_display_mode();
        }
    }

    //? The blit pass handles scaling + letterboxing to the actual window size.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width < MIN_WIDTH || new_size.height < MIN_HEIGHT {
            return;
        }

        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.scale_factor = self.window.scale_factor();
        }
    }

    fn flush_pending_cursor_moved(&mut self) {
        if let Some(event) = self.pending_cursor_moved.take() {
            let _ = self.egui_state.on_window_event(&self.window, &event);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_frame_due(&self) -> bool {
        if self.context.target_fps == 0 {
            return true;
        }

        let target_frame_time = Duration::from_secs_f64(1.0 / self.context.target_fps as f64);
        Instant::now().duration_since(self.last_frame) >= target_frame_time
    }

    //? Main render loop: handle input, update game, regenerate atmosphere if needed, and draw the frame.
    fn render(&mut self, pacing: RenderPacing) -> Result<(), wgpu::SurfaceError> {
        #[cfg(target_arch = "wasm32")]
        let _ = pacing;

        self.flush_pending_cursor_moved();

        let now = Instant::now();
        let raw_dt = (now - self.last_frame).as_secs_f32().min(0.1); //* cap at 100ms to prevent spiral of death
        self.last_frame = now;

        //? Rebuild action state from raw inputs at frame start
        self.context.input.begin_frame(raw_dt);

        //? Compute letterbox viewport (shared by egui constraint + blit pass)
        let sw = self.config.width as f32;
        let sh = self.config.height as f32;
        let target_aspect = self.internal_width as f32 / self.internal_height as f32;
        let window_aspect = sw / sh;
        let (vp_w, vp_h) = if window_aspect > target_aspect {
            (sh * target_aspect, sh)
        } else {
            (sw, sw / target_aspect)
        };
        let vp_x = (sw - vp_w) / 2.0;
        let vp_y = (sh - vp_h) / 2.0;

        //? Build the egui UI and detect discrete changes to scene parameters (excluding time).
        let mut params = self.params.clone();
        self.context.scene_params_override = None;
        self.context.bloom_override = None;
        let mut raw_input = self.egui_state.take_egui_input(&self.window);
        let ppp = self.egui_ctx.pixels_per_point();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::pos2(vp_x / ppp, vp_y / ppp),
            egui::vec2(vp_w / ppp, vp_h / ppp),
        ));
        let ctx = &mut self.context;
        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            crate::ui::apply_theme(egui_ctx);
            self.game.ui(egui_ctx, ctx, &mut params);
            if ctx.show_perf_hud {
                crate::ui::show_perf_hud(egui_ctx, ctx.perf());
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        self.apply_requested_display_changes();

        let ui_changed = params.background_color != self.params.background_color
            || params.sky != self.params.sky
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
            self.params.time += raw_dt;
        }

        let render_params = self
            .context
            .scene_params_override
            .clone()
            .unwrap_or_else(|| self.params.clone());

        let render_params_changed = render_params.background_color
            != self.prev_params.background_color
            || render_params.sky != self.prev_params.sky
            || render_params.seed != self.prev_params.seed
            || render_params.fog_enabled != self.prev_params.fog_enabled
            || render_params.fog_density != self.prev_params.fog_density
            || render_params.fog_opacity != self.prev_params.fog_opacity
            || render_params.fog_color != self.prev_params.fog_color
            || render_params.fog_anim_speed != self.prev_params.fog_anim_speed;

        if ui_changed || render_params_changed {
            self.atmosphere_dirty = true;
        }

        let fog_animating = render_params.fog_enabled && render_params.fog_anim_speed > 0.0;
        let regen_due = now.duration_since(self.last_atmosphere_regen) >= ATMOSPHERE_REGEN_INTERVAL;

        //? Dirty check: animations dont get interrupted by UI tweaks.
        if self.atmosphere_dirty || (fog_animating && regen_due) {
            atmosphere::render_sky_to_buffer(
                &mut self.sky_buffer,
                SIM_WIDTH,
                SIM_HEIGHT,
                &render_params,
            );
            atmosphere::render_fog_to_buffer(
                &mut self.fog_buffer,
                SIM_WIDTH,
                SIM_HEIGHT,
                &render_params,
                &mut self.atmosphere_noise_cache,
            );
            upload_atmosphere_texture(&self.queue, &self.sky_texture, &self.sky_buffer);
            upload_atmosphere_texture(&self.queue, &self.fog_texture, &self.fog_buffer);
            self.prev_params = render_params;
            self.atmosphere_dirty = false;
            self.last_atmosphere_regen = now;
        }

        //? Sync tick rate if the game changed it via debug UI.
        if self.context.fixed_tick_rate != self.fixed_time.tick_rate() {
            self.fixed_time.set_tick_rate(self.context.fixed_tick_rate);
        }

        //? Apply pending freeze frames from game (hit-stop)
        if self.context.freeze_frames > 0 {
            self.fixed_time.freeze(self.context.freeze_frames);
            self.context.freeze_frames = 0;
        }

        //? Fixed-timestep accumulator: run deterministic updates at FixedTime intervals
        let steps = self.fixed_time.accumulate(raw_dt);
        let hit_fixed_step_cap = self.fixed_time.pending_steps() > self.fixed_time.max_steps();
        for _ in 0..steps {
            self.context.delta_time = self.fixed_time.fixed_dt;
            self.game.fixed_update(&mut self.context, &self.fixed_time);
            self.fixed_time.advance();
        }

        self.context.interpolation_alpha = self.fixed_time.interpolation_alpha();

        self.fps_samples.push_back(raw_dt);
        if self.fps_samples.len() > 120 {
            self.fps_samples.pop_front();
        }
        let avg_dt = self.fps_samples.iter().sum::<f32>() / self.fps_samples.len() as f32;
        let fps = if raw_dt > 0.0 { 1.0 / raw_dt } else { 0.0 };
        let avg_fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
        let frame_time_ms = raw_dt * 1000.0;
        let avg_frame_time_ms = avg_dt * 1000.0;
        self.context.fps = avg_fps;
        self.context.frame_time_ms = avg_frame_time_ms;
        self.context.set_perf(FrameStats {
            fps,
            avg_fps,
            frame_time_ms,
            avg_frame_time_ms,
            fixed_steps: steps,
            max_fixed_steps: self.fixed_time.max_steps(),
            hit_fixed_step_cap,
            fixed_debt_ms: self.fixed_time.accumulator_seconds().max(0.0) * 1000.0,
        });

        //? Per-frame update with visual dt (camera smoothing, interpolation, etc.)
        self.context.delta_time = raw_dt;
        self.context.clear_sprites();
        self.game.update(&mut self.context);
        self.game.render(&mut self.context);

        //? Apply pending screen shakes from game to camera
        for &(intensity, duration) in &self.context.pending_shakes {
            self.camera.add_shake(intensity, duration);
        }
        self.context.pending_shakes.clear();
        self.camera.update_shakes(raw_dt);

        //? Drain deduplicated UI audio events queued during fixed_update/update/ui
        self.context.drain_ui_audio_events();

        //? Update camera position based on context before rendering.
        self.camera
            .set_offset(self.context.camera_offset_x, self.context.camera_offset_y);
        self.sprite_renderer
            .update_camera(&self.queue, &self.camera);

        self.sprite_renderer
            .prepare(&self.queue, &self.context.sprite_batch, &self.texture_sizes);

        let bloom_settings = self.context.bloom_override.unwrap_or(self.context.bloom);
        self.queue.write_buffer(
            &self.bloom_uniform_buffer,
            0,
            bytemuck::bytes_of(&BloomUniform::from(bloom_settings)),
        );

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
        //* Multiple render passes: one for the full-screen atmosphere quad, one for the sprites, and one for the egui overlay.
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

        //? Pass 1: full-screen quad with atmosphere textures → offscreen buffer
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("World Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.offscreen_view,
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
            pass.set_bind_group(0, &self.atmosphere_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        //? Pass 2: sprite rendering → offscreen buffer
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sprite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.offscreen_view,
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
                .render_multi(&mut pass, &self.texture_bind_groups);
        }

        //? Pass 3: blit offscreen buffer → surface with nearest-neighbor + letterboxing
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blit Pass"),
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
            pass.set_viewport(vp_x, vp_y, vp_w, vp_h, 0.0, 1.0);
            pass.set_pipeline(&self.bloom_pipeline);
            pass.set_bind_group(0, &self.blit_bind_group, &[]);
            pass.set_bind_group(1, &self.bloom_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        //? Pass 4: egui overlay → surface (native resolution for crisp debug text)
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

        #[cfg(target_arch = "wasm32")]
        if !self.first_frame_event_sent {
            self.first_frame_event_sent = true;
            if let Some(event_name) = G::wasm_ready_event() {
                if let Some(web_window) = web_sys::window()
                    && let Ok(event) = web_sys::Event::new(event_name)
                {
                    let _ = web_window.dispatch_event(&event);
                }
            }
        }

        //? Save the current state of tracked keys for the next frame's edge detection calculations.
        self.context.input.end_frame();

        //? Visual FPS limiter: sleep to cap frame rate if target_fps is set (native only)
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(pacing, RenderPacing::SleepToCap) && self.context.target_fps > 0 {
            let target_frame_time =
                std::time::Duration::from_secs_f64(1.0 / self.context.target_fps as f64);
            let elapsed = Instant::now() - now;
            if elapsed < target_frame_time {
                std::thread::sleep(target_frame_time - elapsed);
            }
        }

        Ok(())
    }
}

//? Additional Helpers

//? Convert physical pixel size to game-logic dimensions (unused with fixed internal resolution).
#[allow(dead_code)]
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

fn upload_atmosphere_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, data: &[u8]) {
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
