use crate::{
    bind::*,
    pp::ShaderSource,
    util::{AllignedBufferSize, RawFrame},
};
use std::path::PathBuf;
use winit::{dpi::PhysicalSize, window::Window};

pub const VS_ENTRY: &str = "vs_main";
pub const FS_ENTRY: &str = "fs_main";

pub struct WgpuSetup {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
}

impl WgpuSetup {
    pub async fn new(wgpu_instance: &wgpu::Instance, surface: Option<&wgpu::Surface>) -> Self {
        let adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: surface,
            })
            .await
            .expect("find an appropriate adapter");

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

        Self {
            device,
            queue,
            adapter,
        }
    }
}

pub struct WgpuContext {
    pub window: Window,
    pub bindings: ShaderBindings,
    pub queue: wgpu::Queue,
    pub output_buffer: Option<wgpu::Buffer>,
    pub output_texture: Option<wgpu::Texture>,
    pub device: wgpu::Device,
    pub size: PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface,
    shader_path: PathBuf,
}

impl WgpuContext {
    pub async fn new(window: Window, shader_path: PathBuf) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(&window) }.expect("creating surface");
        let init = WgpuSetup::new(&instance, Some(&surface)).await;

        log::info!("Selected adapter: {:?}", init.adapter.get_info());

        let surface_caps = surface.get_capabilities(&init.adapter);
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

        surface.configure(&init.device, &config);

        let bindings = ShaderBindings::new(&init.device);
        let bind_group_layout = bindings.create_bind_group_layout(&init.device);
        let shader_src = ShaderSource::validate(&shader_path, &bindings).unwrap_or_default();
        let pipeline = create_render_pipeline(
            &init.device,
            &bind_group_layout,
            shader_src.as_str(),
            config.format,
        );
        Self {
            surface,
            device: init.device,
            queue: init.queue,
            config,
            window,
            shader_path,
            pipeline,
            bindings,
            output_buffer: None,
            output_texture: None,
            size,
        }
    }

    pub fn rebuild_shader(&mut self) {
        // crate::util::clear_screen();
        match ShaderSource::validate(&self.shader_path, &self.bindings) {
            Ok(ss) => {
                let bgl = self.bindings.create_bind_group_layout(&self.device);
                self.pipeline =
                    create_render_pipeline(&self.device, &bgl, ss.as_str(), self.config.format)
            }
            Err(err) => println!("{err}"),
        }
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.size = *new_size;
        }
    }

    pub fn render_frame(&self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let texture_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let bg = self.bindings.create_bind_group(&self.device);

        render_frame(&mut encoder, &self.pipeline, &bg, &texture_view);

        self.queue.submit(Some(encoder.finish()));
        // without this surface will not be updated
        output.present();

        Ok(())
    }

    pub fn render_into_frame_buffer(&mut self) -> FrameBuffer {
        let bg = self.bindings.create_bind_group(&self.device);
        let texture = create_texture(&self.device, &self.size);

        FrameBuffer::new(&self.device, &self.queue, &texture, &self.pipeline, &bg)
    }
}

pub struct FrameBuffer {
    pub buffer: wgpu::Buffer,
    pub buffer_size: AllignedBufferSize,
    pub submission_idx: wgpu::SubmissionIndex,
}

impl FrameBuffer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
    ) -> Self {
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_size = texture.size();
        let mut encoder = create_encoder(device);
        let buffer_size = AllignedBufferSize::new(texture_size.width, texture_size.height);
        let buffer = create_buffer(device, buffer_size.buffer_size as _);
        render_frame(&mut encoder, pipeline, bind_group, &texture_view);
        copy_texture_to_buffer(&mut encoder, texture, &buffer, &buffer_size);
        let submission_idx = queue.submit(Some(encoder.finish()));

        Self {
            buffer,
            buffer_size,
            submission_idx,
        }
    }

    pub async fn map_read(&self) {
        let buffer_slice = self.buffer.slice(..);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        receiver.await.unwrap().unwrap();
    }

    pub fn extract_data(self) -> RawFrame {
        let buffer_slice = self.buffer.slice(..);
        let padded_data = buffer_slice.get_mapped_range();
        let frame = padded_data
            .chunks(self.buffer_size.padded_bytes_per_row as _)
            .flat_map(|ch| &ch[..self.buffer_size.unpadded_bytes_per_row as _])
            .copied()
            .collect::<RawFrame>();
        drop(padded_data);
        self.buffer.unmap();

        frame
    }
}

pub fn render_frame(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    texture_view: &wgpu::TextureView,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
                store: true,
            },
        })],
        depth_stencil_attachment: None,
    });

    render_pass.set_bind_group(0, bind_group, &[]);
    render_pass.set_pipeline(pipeline);
    render_pass.draw(0..3, 0..1);
}

pub fn copy_texture_to_buffer(
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    buf_size: &AllignedBufferSize,
) {
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::ImageCopyBuffer {
            buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(buf_size.padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
}

pub fn create_texture(device: &wgpu::Device, size: &PhysicalSize<u32>) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
    })
}

pub fn create_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    shader_src: &str,
    texture_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = &device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: VS_ENTRY,
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: FS_ENTRY,
            targets: &[Some(wgpu::ColorTargetState {
                format: texture_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            // every three vertices will corespond to one triangle
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            // triangles with vertices that arranged in counter-clockwise direction are facing forward
            front_face: wgpu::FrontFace::Ccw,
            // triangles are culled (not included in the render) if they are not facing forward
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            // related to anit-aliasing
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}

// v padlu pisat' bukvy
pub fn create_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
}
