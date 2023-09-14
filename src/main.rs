mod state;
mod utils;

use crate::state::State;
use notify::Watcher;
use std::path::PathBuf;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

async fn run(
    shader_source: PathBuf,
    rx: std::sync::mpsc::Receiver<notify::Result<notify::event::Event>>,
) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("pussy")
        // TODO: make this optional
        .with_inner_size(PhysicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("create window");

    let mut app_state = State::new(window, shader_source).await;

    event_loop.run(move |ev, _, cf| {
        *cf = ControlFlow::Poll;

        match ev {
            Event::MainEventsCleared => {
                while let Ok(change) = rx.try_recv() {
                    // TODO: handle errors and other events
                    if let notify::event::EventKind::Modify(_) = change.unwrap().kind {
                        app_state.rebuild_shader();
                    }
                }

                app_state.window().request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app_state.window().id() && !app_state.input(event) => match event {
                WindowEvent::CloseRequested => *cf = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => app_state.resize(*physical_size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    app_state.resize(**new_inner_size)
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == app_state.window().id() => {
                app_state.update();
                match app_state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        app_state.resize(app_state.window().inner_size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => *cf = ControlFlow::Exit,
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::RedrawEventsCleared => app_state.window().request_redraw(),
            _ => {}
        }
    })
}

fn main() {
    env_logger::init();

    // TODO: properly handle cl arguments
    let shader_path = PathBuf::from(std::env::args().nth(1).expect("shader path"));
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher_config =
        notify::Config::default().with_poll_interval(std::time::Duration::from_millis(500));
    let mut wathcer = notify::RecommendedWatcher::new(tx, watcher_config).unwrap();

    wathcer
        .watch(&shader_path, notify::RecursiveMode::NonRecursive)
        .unwrap();

    pollster::block_on(run(shader_path, rx));
}
