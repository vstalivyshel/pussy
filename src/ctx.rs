use crate::{bind::*, pp::ShaderSource};
use std::path::PathBuf;
use winit::{dpi::PhysicalSize, window::Window};

pub const VS_ENTRY: &str = "vs_main";
pub const FS_ENTRY: &str = "fs_main";

pub struct WgpuContext {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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
        let surface = unsafe { instance.create_surface(&window) }.expect("creating surface");

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
        let shader_source = ShaderSource::validate(&shader_path, &bindings)
            .unwrap_or_default()
            .into_inner();
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

    pub fn update_bindings<F>(&mut self, change: F)
    where
        F: FnOnce(&mut ShaderBindings),
    {
        change(&mut self.bindings);
        self.bindings.stage(&self.queue);
    }

    pub fn rebuild_shader(&mut self) {
        // crate::util::clear_screen();
        match ShaderSource::validate(&self.shader_path, &self.bindings) {
            Ok(src) => {
                self.pipeline = create_render_pipeline(
                    &self.device,
                    &self.pipeline_layout,
                    src.as_str(),
                    self.config.format,
                )
            }
            Err(err) => println!("{err}"),
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
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
        let texture_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // create CommandEncoder that will build a command buffer
        // for the commands that will be send to the gpu
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.render_frame(&mut encoder, &texture_view);
        self.queue.submit(Some(encoder.finish()));
        // need to be called after any work on the texture Queue::submit()
        output.present();

        Ok(())
    }

    fn render_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture_view: &wgpu::TextureView,
    ) {
        let bindings_bind_group_layout = self.bindings.create_bind_group_layout(&self.device);
        let bindings_bind_group = self
            .bindings
            .create_bind_group(&self.device, &bindings_bind_group_layout);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
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

        render_pass.set_bind_group(0, &bindings_bind_group, &[]);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw(0..3, 0..1);

        drop(render_pass);
    }

    pub async fn capture_frame(&mut self) -> Vec<u8> {
        // window size or surface size? is there any difference?
        let size = self.window.inner_size();

        // wgpu requires texture -> buffer copies to be aligned using
        // wgpu::COPY_BYTES_PER_ROW_ALIGNMENT. Because of this we'll
        // need to save both the padded_bytes_per_row as well as the
        // unpadded_bytes_per_row
        let bytes_per_pixel = std::mem::size_of::<u32>() as u32;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded_bytes_per_row = size.width * bytes_per_pixel;
        let padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;

        // create a buffer to copy the texture so we can get the data
        let buffer_size = (padded_bytes_per_row * size.height) as wgpu::BufferAddress;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Texture Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TEMP texture"),
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
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.render_frame(&mut encoder, &texture_view);

        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );

        let submission_idx = self.queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        // Poll the device in a blocking manner so that our future resolves.
        // In an actual application, `device.poll(...)` should
        // be called in an event loop or on another thread.
        //
        // We pass our submission index so we don't need to wait for any other possible submissions.
        self.device
            .poll(wgpu::Maintain::WaitForSubmissionIndex(submission_idx));
        let _ = rx
            .receive()
            .await
            // supposed to return OK(()) indicating that buffer mapping is finished
            // otherwise will return BufferAsyncError
            .expect("receiving good news from the gpu");

        let padded_data = buffer_slice.get_mapped_range();
        let frame_data = padded_data
            .chunks(padded_bytes_per_row as _)
            .flat_map(|ch| &ch[..unpadded_bytes_per_row as _])
            .copied()
            .collect::<Vec<_>>();
        drop(padded_data);
        output_buffer.unmap();

        frame_data
    }
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader_src: &str,
    texture_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = &device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(pipeline_layout),
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
            // tell to use all of the samples to be active (only one in this case)
            mask: !0,
            // related to anit-aliasing
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}
