use crate::ctx::{FS_ENTRY, VS_ENTRY};
use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
};
use std::io::Write;

pub fn clear_screen() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, MoveTo(0, 0), Clear(ClearType::All));
    let _ = stdout.flush();
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
