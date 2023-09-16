use crate::bind::*;
use crate::util::{create_render_pipeline, load_shader_from_path};
use std::path::PathBuf;
use winit::{dpi::PhysicalSize, event::*, window::Window};

pub struct WgpuContext {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    shader_path: PathBuf,
    bindings: ShaderBindings,
    window: Window,
}

impl WgpuContext {
    pub async fn new(window: Window, shader_path: PathBuf) -> Self {
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
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);
        let bindings = ShaderBindings::new(&device);

        // TODO: handle error
        let shader_source = load_shader_from_path(&shader_path).expect("loading shader source");
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bindings.create_bind_group_layout(&device)],
            push_constant_ranges: &[],
        });
        let pipeline =
            create_render_pipeline(&device, &pipeline_layout, &shader_source, config.format);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            pipeline_layout,
            window,
            shader_path,
            bindings,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self, time: f32) {
        // TODO: change this
        self.bindings.time.data = Time(time);
        self.bindings.stage(&self.queue);
    }

    pub fn rebuild_shader(&mut self) {
        crate::util::clear_screen();
        match load_shader_from_path(&self.shader_path) {
            Ok(src) => {
                self.pipeline = create_render_pipeline(
                    &self.device,
                    &self.pipeline_layout,
                    &src,
                    self.config.format,
                )
            }
            Err(err) => println!("{err}"),
        }
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

        let bindings_bind_group_layout = self.bindings.create_bind_group_layout(&self.device);
        let bindings_bind_group = self
            .bindings
            .create_bind_group(&self.device, &bindings_bind_group_layout);

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
                //
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

            render_pass.set_bind_group(0, &bindings_bind_group, &[]);
            render_pass.set_pipeline(&self.pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(cmd_encoder.finish()));
        output.present();

        Ok(())
    }
}
