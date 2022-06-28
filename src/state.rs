use std::iter;
use std::rc::Rc;
use instant::Instant;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration, TextureView};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::Window;
use crate::gui::Framework;
use crate::{SAMPLE_COUNT, texture};
use crate::texture::Texture;


pub struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    view: TextureView,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    frame_count: usize,
    accum_time: f32,
    pub last_update_time: Instant,
    last_frame_time: Instant,
    pub(crate) mouse_pressed: bool,
    pub gui: Framework,
    window: Rc<Window>,
    depth_texture: Texture,
}

impl State {
    pub(crate) async fn new(window: Rc<Window>, event_loop: &EventLoop<()>) -> Self {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&*window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        {
                            let mut limit = wgpu::Limits::downlevel_webgl2_defaults();
                            limit.max_texture_dimension_2d = 4096;
                            limit
                        }
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();
        let format = surface.get_supported_formats(&adapter)[0];
        let gui = Framework::new(&window, event_loop, &device, format);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);
        let view = create_multisampled_framebuffer(&device, &config);
        let last_update_time = Instant::now();
        let last_frame_time = Instant::now();
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");
        Self {
            surface,
            config,
            size,
            frame_count: 0,
            last_update_time,
            last_frame_time,
            accum_time: 0.,
            device,
            queue,
            view,
            mouse_pressed: false,
            gui,
            window,
            depth_texture
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.view = create_multisampled_framebuffer(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub(crate) fn input(&mut self, event: &WindowEvent, window: &Window) -> bool {
        // if self.menu_mode() {
        //     return false
        // }
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                self.mouse_pressed = true;
                window.set_cursor_grab(true).ok();
                window.set_cursor_visible(false);
                true
            }
            _ => false
        }
    }

    pub(crate) fn update(&mut self) {
        let now = instant::Instant::now();
        let dt = self.last_update_time.elapsed();
        self.last_update_time = now;
    }

    pub(crate) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.accum_time += self.last_frame_time.elapsed().as_secs_f32();
        self.last_frame_time = Instant::now();
        self.frame_count += 1;
        if self.frame_count == 100 {
            log::info!("FPS: {}", (self.frame_count as f32 / self.accum_time) as usize);
            self.accum_time = 0.;
            self.frame_count = 0;
        }

        let output = self.surface.get_current_texture()?;
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
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: if SAMPLE_COUNT == 1 {
                    &view
                } else {
                    &self.view
                },
                resolve_target: Some(&view).filter(|_| SAMPLE_COUNT != 1),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });
            self.gui.prepare(&self.window, &self.device, &self.queue);
            self.gui.render(&mut render_pass, &self.window);
    }
        self.queue.submit(iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    pub fn menu_mode(&self) -> bool {
        self.gui.gui.window_open
    }
    pub fn open_menu(&mut self) {
        self.gui.gui.window_open = true;
    }
}

fn create_multisampled_framebuffer(
    device: &Device,
    config: &SurfaceConfiguration,
) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}