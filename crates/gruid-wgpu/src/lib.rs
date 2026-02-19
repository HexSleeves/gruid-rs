//! GPU-accelerated graphical backend for gruid using wgpu.
//!
//! Renders the grid using instanced quads on the GPU. Each grid cell is
//! one quad instance; glyph bitmaps are packed into a texture atlas and
//! sampled in the fragment shader for fg/bg color blending.
//!
//! Uses:
//! - [`wgpu`] for GPU rendering
//! - [`winit`] for window creation and input events
//! - [`fontdue`] for glyph rasterization into the atlas
//!
//! Supports custom tile rendering via the [`TileManager`] trait (same
//! interface as `gruid-winit`).

mod input;
mod renderer;

use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use gruid_core::{
    app::{AppRunner, EventLoopDriver},
    messages::Msg,
};

use renderer::{CellInstance, GridRenderer};

pub use gruid_core::TileManager;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the wgpu driver.
pub struct WgpuConfig {
    /// Window title.
    pub title: String,
    /// Embedded font bytes (TTF/OTF).
    pub font_data: Option<Vec<u8>>,
    /// Font size in logical points.
    pub font_size: f32,
    /// Number of grid columns.
    pub grid_width: i32,
    /// Number of grid rows.
    pub grid_height: i32,
    /// Optional tile manager for custom tile-based rendering.
    pub tile_manager: Option<Box<dyn TileManager>>,
    /// Integer scale factor for tiles (0 = auto-detect from DPI).
    pub tile_scale: u32,
}

impl Default for WgpuConfig {
    fn default() -> Self {
        Self {
            title: "gruid".into(),
            font_data: None,
            font_size: 18.0,
            grid_width: 80,
            grid_height: 24,
            tile_manager: None,
            tile_scale: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// WgpuDriver
// ---------------------------------------------------------------------------

/// GPU-accelerated graphical driver for gruid.
///
/// Implements [`EventLoopDriver`] — it owns the main-thread event loop
/// and drives an [`AppRunner`].
pub struct WgpuDriver {
    config: WgpuConfig,
}

impl WgpuDriver {
    pub fn new(config: WgpuConfig) -> Self {
        Self { config }
    }
}

impl EventLoopDriver for WgpuDriver {
    fn run(self, runner: AppRunner) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        let mut app = WgpuApp::new(self.config, runner);
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GPU State
// ---------------------------------------------------------------------------

struct GpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    _bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    atlas_texture: wgpu::Texture,
    atlas_extent: wgpu::Extent3d,
    _sampler: wgpu::Sampler,
    instance_count: u32,
}

// ---------------------------------------------------------------------------
// WgpuApp — ApplicationHandler
// ---------------------------------------------------------------------------

struct WgpuApp {
    config: WgpuConfig,
    runner: AppRunner,
    renderer: Option<GridRenderer>,
    gpu: Option<GpuState>,
    window: Option<Arc<Window>>,
    scale_factor: f64,
}

impl WgpuApp {
    fn new(config: WgpuConfig, runner: AppRunner) -> Self {
        Self {
            config,
            runner,
            renderer: None,
            gpu: None,
            window: None,
            scale_factor: 1.0,
        }
    }

    fn render(&mut self) {
        if self.runner.should_quit() {
            return;
        }

        self.runner.process_pending_msgs();

        let frame = self.runner.draw_frame();

        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => return,
        };

        if let Some(frame) = frame {
            renderer.apply_frame(&frame);
        }

        let gpu = match self.gpu.as_ref() {
            Some(g) => g,
            None => return,
        };

        // Upload instance buffer if dirty
        if renderer.dirty {
            let data = bytemuck::cast_slice(&renderer.instances);
            if data.len() as u64 <= gpu.instance_buffer.size() {
                gpu.queue.write_buffer(&gpu.instance_buffer, 0, data);
            }
            renderer.dirty = false;
        }

        // Upload atlas if dirty
        if renderer.atlas_dirty {
            let aw = renderer.atlas.width;
            let ah = renderer.atlas.height;
            if aw == gpu.atlas_extent.width && ah == gpu.atlas_extent.height {
                gpu.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &gpu.atlas_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &renderer.atlas.data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(aw),
                        rows_per_image: Some(ah),
                    },
                    gpu.atlas_extent,
                );
            } else {
                log::warn!("Atlas grew to {}x{} — rebuild needed", aw, ah);
            }
            renderer.atlas_dirty = false;
        }

        // Update uniforms
        let uniforms = renderer.uniforms();
        gpu.queue
            .write_buffer(&gpu.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Render
        let surface_texture = match gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gruid-wgpu encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gruid-wgpu pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.bind_group, &[]);
            pass.set_vertex_buffer(0, gpu.instance_buffer.slice(..));
            pass.draw(0..4, 0..gpu.instance_count);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    fn init_gpu(&mut self, window: Arc<Window>) {
        let scale_factor = window.scale_factor();
        self.scale_factor = scale_factor;

        let physical_font_size = self.config.font_size * scale_factor as f32;
        let tile_scale = if self.config.tile_scale > 0 {
            self.config.tile_scale
        } else {
            (scale_factor.round() as u32).max(1)
        };

        let renderer = GridRenderer::new(
            self.config.font_data.as_deref(),
            physical_font_size,
            self.config.grid_width as usize,
            self.config.grid_height as usize,
            self.config.tile_manager.take(),
            tile_scale,
        );

        let phys_w = renderer.pixel_width() as u32;
        let phys_h = renderer.pixel_height() as u32;

        // wgpu setup
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster_block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter found");

        let (device, queue) =
            pollster_block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("failed to create GPU device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: phys_w.max(1),
            height: phys_h.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Shader
        let shader_src = include_str!("grid.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grid shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        // Uniform buffer
        let uniforms = renderer.uniforms();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Instance buffer
        let instance_data = bytemuck::cast_slice(&renderer.instances);
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instances"),
            contents: instance_data,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Atlas texture
        let atlas_extent = wgpu::Extent3d {
            width: renderer.atlas.width,
            height: renderer.atlas.height,
            depth_or_array_layers: 1,
        };
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph atlas"),
            size: atlas_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &renderer.atlas.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(renderer.atlas.width),
                rows_per_image: Some(renderer.atlas.height),
            },
            atlas_extent,
        );

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("grid bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<CellInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        // grid_pos
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        // fg_color
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 8,
                            shader_location: 1,
                        },
                        // bg_color
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 12,
                            shader_location: 2,
                        },
                        // atlas_rect
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 3,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        let instance_count = renderer.instances.len() as u32;

        self.renderer = Some(renderer);
        self.gpu = Some(GpuState {
            device,
            queue,
            surface,
            surface_config,
            pipeline,
            _bind_group_layout: bind_group_layout,
            bind_group,
            uniform_buffer,
            instance_buffer,
            atlas_texture,
            atlas_extent,
            _sampler: sampler,
            instance_count,
        });
        self.window = Some(window);
    }
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let scale_factor = event_loop
            .available_monitors()
            .next()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0);

        let physical_font_size = self.config.font_size * scale_factor as f32;
        let tile_scale = if self.config.tile_scale > 0 {
            self.config.tile_scale
        } else {
            (scale_factor.round() as u32).max(1)
        };

        // Temporarily create renderer to get window size
        let temp_renderer = GridRenderer::new(
            self.config.font_data.as_deref(),
            physical_font_size,
            self.config.grid_width as usize,
            self.config.grid_height as usize,
            None, // don't consume tile_manager yet
            tile_scale,
        );
        let phys_w = temp_renderer.pixel_width() as u32;
        let phys_h = temp_renderer.pixel_height() as u32;
        drop(temp_renderer);

        let window_attrs = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(PhysicalSize::new(phys_w, phys_h))
            .with_resizable(true);

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("failed to create window"),
        );

        self.init_gpu(window);
        self.runner.init();
        self.render();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let cell_w = self.renderer.as_ref().map(|r| r.cell_width).unwrap_or(8);
        let cell_h = self.renderer.as_ref().map(|r| r.cell_height).unwrap_or(16);

        match event {
            WindowEvent::CloseRequested => {
                self.runner.handle_msg(Msg::Quit);
                event_loop.exit();
            }

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.surface_config.width = width.max(1);
                    gpu.surface_config.height = height.max(1);
                    gpu.surface.configure(&gpu.device, &gpu.surface_config);

                    if let Some(renderer) = self.renderer.as_mut() {
                        let (cw, ch) = (renderer.cell_width, renderer.cell_height);
                        if cw > 0 && ch > 0 {
                            let new_cols = (width as i32) / (cw as i32);
                            let new_rows = (height as i32) / (ch as i32);
                            if new_cols > 0 && new_rows > 0 {
                                renderer.resize_grid(new_cols as usize, new_rows as usize);
                                gpu.instance_count = (new_cols as u32) * (new_rows as u32);

                                // Reallocate instance buffer if needed
                                let needed = (renderer.instances.len()
                                    * std::mem::size_of::<CellInstance>())
                                    as u64;
                                if needed > gpu.instance_buffer.size() {
                                    gpu.instance_buffer =
                                        gpu.device.create_buffer(&wgpu::BufferDescriptor {
                                            label: Some("instances"),
                                            size: needed,
                                            usage: wgpu::BufferUsages::VERTEX
                                                | wgpu::BufferUsages::COPY_DST,
                                            mapped_at_creation: false,
                                        });
                                }

                                self.runner.resize(new_cols, new_rows);
                                self.runner.handle_msg(Msg::Screen {
                                    width: new_cols,
                                    height: new_rows,
                                    time: std::time::Instant::now(),
                                });
                            }
                        }
                    }
                }
                self.render();
            }

            WindowEvent::RedrawRequested => {
                self.render();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(msg) = input::translate_keyboard(&event) {
                    self.runner.handle_msg(msg);
                    if self.runner.should_quit() {
                        event_loop.exit();
                        return;
                    }
                    self.render();
                    if let Some(w) = self.window.as_ref() {
                        w.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                if let Some(msg) = input::translate_mouse_button(btn_state, button, cell_w, cell_h)
                {
                    self.runner.handle_msg(msg);
                    if self.runner.should_quit() {
                        event_loop.exit();
                        return;
                    }
                    self.render();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(msg) = input::translate_cursor_moved(position, cell_w, cell_h) {
                    self.runner.handle_msg(msg);
                    self.render();
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(msg) = input::translate_mouse_wheel(delta, cell_w, cell_h) {
                    self.runner.handle_msg(msg);
                    if self.runner.should_quit() {
                        event_loop.exit();
                        return;
                    }
                    self.render();
                }
            }

            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal pollster (block on async without pulling in tokio)
// ---------------------------------------------------------------------------

fn pollster_block_on<F: std::future::Future>(f: F) -> F::Output {
    // Simple spin-poll for the adapter/device request which resolves
    // almost immediately on desktop.
    let mut f = std::pin::pin!(f);
    let waker = waker_fn::waker_fn(|| {});
    let mut cx = std::task::Context::from_waker(&waker);
    loop {
        match f.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(v) => return v,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// Minimal waker that does nothing (suitable for spin-polling).
mod waker_fn {
    use std::sync::Arc;
    use std::task::{RawWaker, RawWakerVTable, Waker};

    pub fn waker_fn<F: Fn() + Send + Sync + 'static>(f: F) -> Waker {
        let raw = Arc::into_raw(Arc::new(f)) as *const ();
        let vtable = &RawWakerVTable::new(clone_fn::<F>, wake_fn::<F>, wake_fn::<F>, drop_fn::<F>);
        unsafe { Waker::from_raw(RawWaker::new(raw, vtable)) }
    }

    unsafe fn clone_fn<F: Fn() + Send + Sync + 'static>(ptr: *const ()) -> RawWaker {
        let arc = unsafe { Arc::from_raw(ptr as *const F) };
        let _clone = arc.clone();
        std::mem::forget(arc);
        let vtable = &RawWakerVTable::new(clone_fn::<F>, wake_fn::<F>, wake_fn::<F>, drop_fn::<F>);
        RawWaker::new(Arc::into_raw(_clone) as *const (), vtable)
    }

    unsafe fn wake_fn<F: Fn() + Send + Sync + 'static>(ptr: *const ()) {
        let arc = unsafe { Arc::from_raw(ptr as *const F) };
        (arc)();
    }

    unsafe fn drop_fn<F: Fn() + Send + Sync + 'static>(ptr: *const ()) {
        drop(unsafe { Arc::from_raw(ptr as *const F) });
    }
}
