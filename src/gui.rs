use crate::SAMPLE_COUNT;
use egui::{ClippedPrimitive, Context, TexturesDelta};
use egui_wgpu::renderer::{RenderPass, ScreenDescriptor};
use wgpu::{Device, Queue};
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct Framework {
    egui_ctx: Context,
    egui_state: egui_winit::State,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,
    // State for the GUI
    pub(crate) gui: GUI,
    pub scale_factor: f32,
    pub egui_rpass: RenderPass
}

impl Framework {
    pub fn new(window: &Window, event_loop: &EventLoop<()>, device: &Device, format: wgpu::TextureFormat) -> Self {

        let scale_factor = window.scale_factor() as f32;
        let mut egui_ctx = Context::default();
        egui_ctx.set_pixels_per_point(scale_factor);
        let egui_state =
            egui_winit::State::new(event_loop);
        let mut egui_rpass = RenderPass::new(&device, format, SAMPLE_COUNT);
        let textures = TexturesDelta::default();
        let gui = GUI { scale: 10, window_open: false };
        Self {
            egui_ctx,
            egui_state,
            egui_rpass,
            paint_jobs: Vec::new(),
            textures,
            gui,
            scale_factor,
        }
    }

    pub(crate) fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        self.egui_state.on_event(&self.egui_ctx, event);
    }

    pub(crate) fn prepare(&mut self, window: &Window, device: &Device, queue: &Queue) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let output = self.egui_ctx.run(raw_input, |egui_ctx| {
            // Draw the demo application.
            self.gui.ui(egui_ctx);
        });

        self.textures.append(output.textures_delta);
        self.egui_state
            .handle_platform_output(window, &self.egui_ctx, output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(output.shapes);

        for (id, image_delta) in &self.textures.set {
            self.egui_rpass.update_texture(&device, &queue, *id, image_delta);
        }
        for id in &self.textures.free {
            self.egui_rpass.free_texture(id);
        }
        self.egui_rpass.update_buffers(&device, &queue, &self.paint_jobs, &Self::get_screen_desc(window));
    }
    fn get_screen_desc(window: &Window) -> ScreenDescriptor {
        let window_size = window.inner_size();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [window_size.width, window_size.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        screen_descriptor
    }
    pub fn render<'a, 'b: 'a>(&'b mut self, render_pass: &mut wgpu::RenderPass<'a>, window: &Window) {

        self.egui_rpass.execute_with_renderpass(
            render_pass,
            &self.paint_jobs,
            &Self::get_screen_desc(window),
        );
        self.textures.clear()
    }
}

pub struct GUI {
    pub scale: u32,
    pub window_open: bool
}

impl GUI {
    fn ui(&mut self, ctx: &Context) {
        egui::Window::new("console").open(&mut self.window_open).show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.scale, 1..=20).text("Scale"));
        });
    }
}
