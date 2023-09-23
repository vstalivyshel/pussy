mod bind;
mod capture;
mod ctx;
mod pp;
mod util;

use crate::bind::Time;
use crate::ctx::WgpuContext;
use notify::Watcher;
use std::path::PathBuf;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

async fn draw(shader_path: PathBuf) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("puss")
        // TODO: make this optional
        .with_inner_size(PhysicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("creating window");

    // setup watcher for shader file
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher_config =
        notify::Config::default().with_poll_interval(std::time::Duration::from_millis(500));
    let mut watcher = notify::RecommendedWatcher::new(tx, watcher_config).unwrap();
    watcher
        .watch(&shader_path, notify::RecursiveMode::NonRecursive)
        .expect("initialize watcher");

    let mut ctx = WgpuContext::new(window, shader_path).await;

    let start_time = std::time::Instant::now();
    let mut capturing_frames = false;
    let mut frames_buffer = Vec::<Vec<u8>>::new();

    event_loop.run(move |ev, _, cf| {
        *cf = ControlFlow::Poll;

        match ev {
            Event::MainEventsCleared => {
                // TODO: handle errors and other events
                while let Ok(Ok(event)) = rx.try_recv() {
                    if let notify::event::EventKind::Modify(_) = event.kind {
                        ctx.rebuild_shader();
                    }
                }

                ctx.window().request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == ctx.window().id() => match event {
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::F5),
                        ..
                    } => {
                        log::info!("Capturing frame");
                        let size = ctx.window().inner_size();
                        let frame = pollster::block_on(ctx.capture_frame());
                        match crate::capture::save_png(&frame, size.width, size.height) {
                            Ok(()) => log::info!(".png saved"),
                            Err(e) => log::error!("{e}"),
                        }
                    }
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::F6),
                        ..
                    } => {
                        capturing_frames = !capturing_frames;
                        log::info!("Recording frames: {capturing_frames}");
                    }
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::F7),
                        ..
                    } => {
                        if !capturing_frames && !frames_buffer.is_empty() {
                            log::info!("Saving .gif");
                            let frames = frames_buffer.clone();
                            frames_buffer.clear();
                            let size = ctx.window().inner_size();
                            std::thread::spawn(move || {
                                match crate::capture::save_gif(frames, 1, size.width, size.height) {
                                    Ok(()) => log::info!(".gif saved"),
                                    Err(e) => log::error!("{e}"),
                                }
                            });
                        }
                    }
                    _ => {}
                },
                WindowEvent::CloseRequested => *cf = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => ctx.resize(*physical_size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    ctx.resize(**new_inner_size)
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == ctx.window().id() => {
                ctx.update_bindings(|b| b.time.data = Time(start_time.elapsed().as_secs_f32()));

                match ctx.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        ctx.resize(ctx.window().inner_size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => *cf = ControlFlow::Exit,
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }

                if capturing_frames {
                    let frame = pollster::block_on(ctx.capture_frame());
                    frames_buffer.push(frame);
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
    if let Some(shader) = std::env::args().nth(1) {
        pollster::block_on(draw(PathBuf::from(shader)));
    }
}
