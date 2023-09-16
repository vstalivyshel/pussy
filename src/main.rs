mod bind;
mod ctx;
mod util;

use crate::ctx::WgpuContext;
use notify::Watcher;
use std::path::PathBuf;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// NOTE: Loading textures:
//      1. gpu::Queue::write_texture()
//      2. `image` crate for encoding-decoding, you know, iamges

// TODO: wgpu throws validation error in some cases, while naga parses successfully. why?
//      example: 2.0 * 2  -> will throw validation error (expression is invalid) by wgpu not by naga

async fn draw(shader_path: PathBuf) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("pussy")
        // TODO: make this optional
        .with_inner_size(PhysicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("create window");
    let start_time = std::time::Instant::now();

    let (tx, rx) = std::sync::mpsc::channel();
    let watcher_config =
        notify::Config::default().with_poll_interval(std::time::Duration::from_millis(500));
    let mut watcher = notify::RecommendedWatcher::new(tx, watcher_config).unwrap();
    watcher
        .watch(&shader_path, notify::RecursiveMode::NonRecursive)
        .unwrap();

    let mut ctx = WgpuContext::new(window, shader_path).await;

    event_loop.run(move |ev, _, cf| {
        *cf = ControlFlow::Poll;

        match ev {
            Event::MainEventsCleared => {
                while let Ok(Ok(event)) = rx.try_recv() {
                    // TODO: handle errors and other events
                    if let notify::event::EventKind::Modify(_) = event.kind {
                        ctx.rebuild_shader();
                    }
                }

                ctx.window().request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == ctx.window().id() && !ctx.input(event) => match event {
                WindowEvent::CloseRequested => *cf = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => ctx.resize(*physical_size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    ctx.resize(**new_inner_size)
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == ctx.window().id() => {
                ctx.update(start_time.elapsed().as_secs_f32());
                match ctx.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        ctx.resize(ctx.window().inner_size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => *cf = ControlFlow::Exit,
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::RedrawEventsCleared => ctx.window().request_redraw(),
            _ => {}
        }
    })
}

fn main() {
    env_logger::init();
    crate::util::clear_screen();
    // TODO: properly handle cl arguments
    let shader_path = PathBuf::from(std::env::args().nth(1).expect("shader path"));
    pollster::block_on(draw(shader_path));
}
