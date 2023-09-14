use std::path::Path;

pub fn create_shader_module<P: AsRef<Path>>(
    device: &wgpu::Device,
    shader_path: P,
) -> wgpu::ShaderModule {
    // TODO: catch invalid path error
    let shader_source =
        std::fs::read_to_string(shader_path.as_ref()).expect("reading shader source");
    let shader_desc = wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    };

    device.create_shader_module(shader_desc)
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    shader_module: &wgpu::ShaderModule,
    texture_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader_module,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader_module,
            entry_point: "fs_main",
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
