mod camera;
mod depth;
mod light;
mod model;

use camera::{CameraUniform, OrbitCamera, OrbitController, Projection};
use glam::{Quat, Vec3};
use model::{Instance, Vertex, INDICES, VERTICES};
use std::{
    f32::consts,
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

const ORBIT_SENSITIVITY: f32 = 0.003;
const PAN_SENSITIVITY: f32 = 0.001;
const ZOOM_SENSITIVITY: f32 = 0.1;

struct State {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    instance_count: u32,
    instance_buffer: wgpu::Buffer,
    depth_texture: depth::Texture,
    camera: OrbitCamera,
    projection: Projection,
    camera_controller: OrbitController,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    light_uniform: light::LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    orbit_pressed: bool,
    pan_pressed: bool,
    fps_counter: FpsCounter,
}

struct FpsCounter {
    last_tick: Instant,
    frames: u32,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let gpu = init_gpu(window.clone()).await;
        let depth_texture =
            depth::Texture::create_depth_texture(&gpu.device, &gpu.config, "depth_texture");

        let (instances, dimensions) = load_initial_scene();
        let (camera, projection, camera_controller) =
            init_camera(dimensions, gpu.config.width, gpu.config.height);

        let buffers =
            create_buffers(&gpu.device, &instances, &camera, &projection, dimensions);
        let bind_groups =
            create_bind_groups(&gpu.device, &buffers.camera_buffer, &buffers.light_buffer);
        let pipelines = create_pipelines(
            &gpu.device,
            gpu.config.format,
            &bind_groups.camera_layout,
            &bind_groups.light_layout,
        );

        Self {
            instance: gpu.instance,
            adapter: gpu.adapter,
            surface: gpu.surface,
            window,
            device: gpu.device,
            queue: gpu.queue,
            config: gpu.config,
            size: gpu.size,
            render_pipeline: pipelines.render_pipeline,
            light_render_pipeline: pipelines.light_render_pipeline,
            vertex_buffer: buffers.vertex_buffer,
            index_buffer: buffers.index_buffer,
            num_indices: buffers.num_indices,
            instance_count: instances.len() as u32,
            instance_buffer: buffers.instance_buffer,
            depth_texture,
            camera,
            projection,
            camera_controller,
            camera_buffer: buffers.camera_buffer,
            camera_bind_group: bind_groups.camera_bind_group,
            light_uniform: buffers.light_uniform,
            light_buffer: buffers.light_buffer,
            light_bind_group: bind_groups.light_bind_group,
            orbit_pressed: false,
            pan_pressed: false,
            fps_counter: FpsCounter { last_tick: Instant::now(), frames: 0 },
        }
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn apply_scene(&mut self, instances: Vec<Instance>, dimensions: Vec3) {
        self.instance_count = instances.len() as u32;
        self.instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.camera = OrbitCamera {
            target: dimensions / 2.0,
            distance: dimensions.z * 2.0,
            yaw: -consts::FRAC_PI_2,
            pitch: consts::FRAC_PI_6,
        };
        self.projection.set_z_far(10.0 * dimensions.z);
        self.light_uniform.position = dimensions.to_array();
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.projection.resize(new_size.width, new_size.height);
            self.depth_texture =
                depth::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.orbit_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                self.pan_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::DroppedFile(path_buf) => {
                if path_buf
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("vox"))
                {
                    if let Some(path) = path_buf.as_os_str().to_str() {
                        match model::vox::load(path) {
                            Ok((instances, dimensions)) => self.apply_scene(instances, dimensions),
                            Err(e) => log::error!("Failed to load {path}: {e}"),
                        }
                        return true;
                    }
                    log::warn!("could not read path {path_buf:?}");
                } else {
                    log::warn!("File {path_buf:?} is not MagicaVox file");
                }
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[CameraUniform::from((&self.camera, &self.projection))]),
        );

        let old_position: Vec3 = self.light_uniform.position.into();
        self.light_uniform.position =
            (Quat::from_axis_angle(Vec3::Y, f32::to_radians(45.0) * dt.as_secs_f32())
                * old_position)
                .into();
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }

    fn render(&mut self) -> Result<(), String> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Lost => return Err("Lost".into()),
            wgpu::CurrentSurfaceTexture::Outdated => return Err("Outdated".into()),
            wgpu::CurrentSurfaceTexture::Timeout => return Err("Timeout".into()),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(()), // window hidden, skip frame
            wgpu::CurrentSurfaceTexture::Validation => return Err("Validation".into()),
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instance_count);

            render_pass.set_pipeline(&self.light_render_pipeline);
            // Light cube uses same bind groups and buffers, except it doesn't use instance_buffer
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.fps_counter.frames += 1;
        if self.fps_counter.last_tick.elapsed().as_secs() >= 1 {
            self.window().set_title(&format!(
                "MagicaVox viewer using wgpu, fps: {}",
                self.fps_counter.frames
            ));
            self.fps_counter.last_tick = Instant::now();
            self.fps_counter.frames = 0;
        }

        Ok(())
    }
}

// Initialization helpers

struct GpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

struct SceneBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    instance_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    light_uniform: light::LightUniform,
    light_buffer: wgpu::Buffer,
}

struct BindGroups {
    camera_layout: wgpu::BindGroupLayout,
    camera_bind_group: wgpu::BindGroup,
    light_layout: wgpu::BindGroupLayout,
    light_bind_group: wgpu::BindGroup,
}

struct Pipelines {
    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,
}

async fn init_gpu(window: Arc<Window>) -> GpuContext {
    let size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        #[cfg(not(target_arch = "wasm32"))]
        backends: wgpu::Backends::PRIMARY,
        #[cfg(target_arch = "wasm32")]
        backends: wgpu::Backends::GL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    // Arc<Window> gives Surface<'static> — window stays alive as long as surface does
    let surface = instance.create_surface(window).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap_or_else(|_| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: true, // software rendering fallback
            }))
            .unwrap()
        });

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    GpuContext { instance, adapter, surface, device, queue, config, size }
}

fn load_initial_scene() -> (Vec<Instance>, Vec3) {
    let vox_path = std::env::args().nth(1);
    let initial_path = vox_path.as_deref().unwrap_or("assets/snow.vox");
    match model::vox::load(initial_path) {
        Ok(scene) => scene,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn init_camera(dimensions: Vec3, width: u32, height: u32) -> (OrbitCamera, Projection, OrbitController) {
    let camera = OrbitCamera {
        target: dimensions / 2.0,
        distance: dimensions.z * 2.0,
        yaw: -consts::FRAC_PI_2,
        pitch: consts::FRAC_PI_6,
    };
    let projection = Projection::new(width, height, consts::FRAC_PI_4, 0.1, 10.0 * dimensions.z);
    let controller = OrbitController::new(ORBIT_SENSITIVITY, PAN_SENSITIVITY, ZOOM_SENSITIVITY);
    (camera, projection, controller)
}

fn create_buffers(
    device: &wgpu::Device,
    instances: &[Instance],
    camera: &OrbitCamera,
    projection: &Projection,
    dimensions: Vec3,
) -> SceneBuffers {
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });
    let num_indices = INDICES.len() as u32;
    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(instances),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[CameraUniform::from((camera, projection))]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let light_uniform = light::LightUniform {
        position: dimensions.to_array(),
        _padding: 0,
        color: [1.0, 1.0, 1.0],
        _padding2: 0,
    };
    let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Light VB"),
        contents: bytemuck::cast_slice(&[light_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    SceneBuffers { vertex_buffer, index_buffer, num_indices, instance_buffer, camera_buffer, light_uniform, light_buffer }
}

fn create_bind_groups(
    device: &wgpu::Device,
    camera_buffer: &wgpu::Buffer,
    light_buffer: &wgpu::Buffer,
) -> BindGroups {
    let uniform_layout_entry = wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };

    let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[uniform_layout_entry],
        label: Some("camera_bind_group_layout"),
    });
    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });

    let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[uniform_layout_entry],
        label: Some("light_bind_group_layout"),
    });
    let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &light_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: light_buffer.as_entire_binding(),
        }],
        label: Some("light_bind_group"),
    });

    BindGroups { camera_layout, camera_bind_group, light_layout, light_bind_group }
}

fn create_pipelines(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    light_layout: &wgpu::BindGroupLayout,
) -> Pipelines {
    let bind_group_layouts = &[Some(camera_layout), Some(light_layout)];

    let main_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts,
        immediate_size: 0,
    });
    let render_pipeline = create_render_pipeline(
        device,
        &main_layout,
        color_format,
        Some(depth::Texture::DEPTH_FORMAT),
        &[Vertex::desc(), Instance::desc()],
        wgpu::include_wgsl!("shader.wgsl"),
    );

    let light_layout_desc = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Light Pipeline Layout"),
        bind_group_layouts,
        immediate_size: 0,
    });
    let light_render_pipeline = create_render_pipeline(
        device,
        &light_layout_desc,
        color_format,
        Some(depth::Texture::DEPTH_FORMAT),
        &[Vertex::desc()],
        wgpu::include_wgsl!("light.wgsl"),
    );

    Pipelines { render_pipeline, light_render_pipeline }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: vertex_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}

struct App {
    state: Option<State>,
    last_render_time: Instant,
    surface_configured: bool,
    // wasm: State::new is async and can't be awaited inside resumed().
    // We keep the window separately so about_to_wait can request redraws while
    // State is still initializing, ensuring window_event is called to pick up
    // pending_state once spawn_local completes.
    #[cfg(target_arch = "wasm32")]
    window: Option<Arc<Window>>,
    #[cfg(target_arch = "wasm32")]
    pending_state: std::rc::Rc<std::cell::RefCell<Option<State>>>,
}

impl App {
    fn new() -> Self {
        Self {
            state: None,
            last_render_time: Instant::now(),
            surface_configured: false,
            #[cfg(target_arch = "wasm32")]
            window: None,
            #[cfg(target_arch = "wasm32")]
            pending_state: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes().with_title("MagicaVox viewer using wgpu"),
                )
                .unwrap(),
        );

        #[cfg(target_arch = "wasm32")]
        {
            use winit::dpi::PhysicalSize;
            use winit::platform::web::WindowExtWebSys;

            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("canvas")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
            let _ = window.request_inner_size(PhysicalSize::new(450, 400));

            self.window = Some(window.clone());
            let pending = self.pending_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let state = State::new(window).await;
                *pending.borrow_mut() = Some(state);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window)));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // wasm: promote pending state once async init completes
        #[cfg(target_arch = "wasm32")]
        if self.state.is_none() {
            self.state = self.pending_state.borrow_mut().take();
        }

        if self
            .state
            .as_ref()
            .map_or(true, |s| s.window().id() != window_id)
        {
            return;
        }

        if self.state.as_mut().unwrap().input(&event) {
            return;
        }

        match event {
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::CloseRequested => event_loop.exit(),
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                log::info!("physical_size: {physical_size:?}");
                self.surface_configured = true;
                self.state.as_mut().unwrap().resize(physical_size);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = self.state.as_ref().unwrap().window().inner_size();
                self.state.as_mut().unwrap().resize(size);
            }
            WindowEvent::RedrawRequested => {
                if !self.surface_configured {
                    return;
                }

                let now = Instant::now();
                let dt = now - self.last_render_time;
                self.last_render_time = now;
                self.state.as_mut().unwrap().update(dt);

                match self.state.as_mut().unwrap().render() {
                    Ok(()) => {}
                    Err(e) if e == "Lost" || e == "Outdated" => {
                        let size = self.state.as_ref().unwrap().size;
                        self.state.as_mut().unwrap().resize(size);
                    }
                    Err(e) if e == "Timeout" => {
                        log::warn!("Surface timeout");
                    }
                    Err(e) => {
                        log::error!("Render error: {e}");
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window().request_redraw();
        } else {
            // wasm: keep ticking so window_event is called to pick up pending_state
            #[cfg(target_arch = "wasm32")]
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(state) = &mut self.state {
                if state.orbit_pressed {
                    state.camera_controller.process_orbit(delta.0, delta.1);
                } else if state.pan_pressed {
                    state.camera_controller.process_pan(delta.0, delta.1);
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(app);
        } else {
            event_loop.run_app(&mut app).unwrap();
        }
    }
}
