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
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::F5),
                            ..
                        },
                    ..
                } => crate::util::handle_render_result(ctx.capture_frame(), &mut ctx, cf),
                WindowEvent::CloseRequested => *cf = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => ctx.resize(*physical_size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    ctx.resize(**new_inner_size)
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == ctx.window().id() => {
                ctx.update_bindings(|b| b.time.data = Time(start_time.elapsed().as_secs_f32()));
                crate::util::handle_render_result(ctx.render(), &mut ctx, cf);
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
