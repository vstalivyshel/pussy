mod state;
mod utils;

use crate::state::State;
use std::path::PathBuf;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

async fn run(shader_path: PathBuf, watcher_events: crate::utils::WatcherEvents) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("pussy")
        // TODO: make this optional
        .with_inner_size(PhysicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("create window");

    let mut app_state = State::new(window, shader_path).await;

    event_loop.run(move |ev, _, cf| {
        *cf = ControlFlow::Poll;

        match ev {
            Event::MainEventsCleared => {
                while let Ok(event) = watcher_events.try_recv() {
                    // TODO: handle errors and other events
                    if let notify::event::EventKind::Modify(_) = event.unwrap().kind {
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
    crate::utils::clear_screen();
    // TODO: properly handle cl arguments
    let shader_path = PathBuf::from(std::env::args().nth(1).expect("shader path"));
    let watcher_events = crate::utils::init_watcher(&shader_path).expect("init watcher");
    pollster::block_on(run(shader_path, watcher_events));
}
