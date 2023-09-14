use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
};
use naga::front::wgsl;
use notify::Watcher;
use std::{io::Write, path::Path};

pub type WatcherEvents = std::sync::mpsc::Receiver<notify::Result<notify::event::Event>>;

pub fn clear_screen() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, MoveTo(0, 0), Clear(ClearType::All));
    let _ = stdout.flush();
}

pub fn init_watcher(file_path: &impl AsRef<Path>) -> notify::Result<WatcherEvents> {
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher_config =
        notify::Config::default().with_poll_interval(std::time::Duration::from_millis(500));
    let mut wathcer = notify::RecommendedWatcher::new(tx, watcher_config)?;
    wathcer.watch(file_path.as_ref(), notify::RecursiveMode::NonRecursive)?;

    Ok(rx)
}

pub fn load_shader_module<P: AsRef<Path>>(shader_path: P) -> Result<String, wgsl::ParseError> {
    let shader_source = std::fs::read_to_string(shader_path).expect("reading shader source");
    wgsl::parse_str(&shader_source).map(|_| shader_source)
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    shader_src: &str,
    texture_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = &device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module,
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
