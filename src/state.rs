use crate::utils::{create_render_pipeline, create_shader_module};
use std::path::PathBuf;
use winit::{dpi::PhysicalSize, event::*, window::Window};

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    shader_source: PathBuf,
    window: Window,
}

impl State {
    pub async fn new(window: Window, shader_source: PathBuf) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to specified surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("find an appropriate adapter");

        log::info!("Selected adapter: {:?}", adapter.get_info());

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    // TODO: verify that the adapter supports the feature
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .expect("create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            // TODO: verify that the adapter supports this
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let shader = create_shader_module(&device, &shader_source);
        let render_pipeline = create_render_pipeline(&device, &shader, config.format);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            shader_source,
            window,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&self) {}

    pub fn rebuild_shader(&mut self) {
        let shader = create_shader_module(&self.device, &self.shader_source);
        self.render_pipeline = create_render_pipeline(&self.device, &shader, self.config.format);
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // wait for the surface to provide a new SurfaceTexture that will be rendered
        let output = self.surface.get_current_texture()?;

        // create TextureView with default settings that will allow to control how the render code
        // interacts with the texture
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // create CommandEncoder that will build a command buffer
        // for the commands that will be send to the gpu
        let mut cmd_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // block will borrow encoder (&mut self) and we need to drop borrowed variable
        // before calling submit() method
        {
            let clear_color = wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            };

            let mut render_pass = cmd_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // describe were to draw a color
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(cmd_encoder.finish()));
        output.present();

        Ok(())
    }
}
