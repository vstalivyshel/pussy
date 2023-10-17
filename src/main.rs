mod bind;
mod capture;
mod ctx;
mod pp;
mod util;

use crate::{ctx::WgpuContext, util::Msg};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

async fn draw(shader_path: &str) {
    let shader_path = std::path::PathBuf::from(shader_path);
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("puss")
        .build(&event_loop)
        .expect("create window");
    let file_watcher = match crate::util::FileWatcher::new(&shader_path) {
        Ok(watcher) => watcher,
        Err(e) => return eprintln!("{e}"),
    };
    let channel = crate::util::Channel::new();
    let mut ctx = WgpuContext::new(window, shader_path).await;
    let mut time = crate::util::Time::new();
    let mut capturing_frames = false;

    event_loop.run(move |ev, _, cf| {
        *cf = ControlFlow::Poll;

        match ev {
            Event::MainEventsCleared => {
                // TODO: handle errors and other events
                while let Ok(Ok(event)) = file_watcher.receiver.try_recv() {
                    if let notify::event::EventKind::Modify(_) = event.kind {
                        ctx.rebuild_shader()
                    }
                }
                ctx.window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == ctx.window.id() => match event {
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Q),
                        ..
                    } => *cf = ControlFlow::ExitWithCode(0),
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::F5),
                        ..
                    } => channel.send_msg(Msg::SavePng {
                        frame: ctx.render_into_frame_buffer(),
                        resolution: ctx.size,
                    }),
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::F6),
                        ..
                    } => {
                        capturing_frames = !capturing_frames;
                        if capturing_frames {
                            log::info!("Recording frames");
                        } else {
                            log::info!("Stoped recording frames");
                        }
                    }
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::F7),
                        ..
                    } if !capturing_frames => channel.send_msg(Msg::SaveMp4 {
                        rate: time.delta as _,
                        resolution: ctx.size,
                    }),
                    _ => {}
                },
                WindowEvent::CloseRequested => {
                    log::info!("Forced shutdown");
                    std::process::exit(0);
                }
                WindowEvent::Resized(physical_size) => ctx.resize(physical_size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    ctx.resize(new_inner_size)
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == ctx.window.id() => {
                ctx.bindings.update_time(&time, &ctx.queue);
                time.update();

                if capturing_frames {
                    channel.send_msg(Msg::ExtractData(ctx.render_into_frame_buffer()));
                }

                match ctx.render_frame() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        log::info!("Resizing window");
                        ctx.resize(&ctx.window.inner_size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("Out of memory, exiting");
                        *cf = ControlFlow::Exit;
                    }
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::RedrawEventsCleared => ctx.window.request_redraw(),
            _ => {}
        }
    })
}

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .unwrap()
        .block_on(async {
            env_logger::init();
            crate::util::clear_screen();
            if let Some(shader) = std::env::args().nth(1) {
                draw(&shader).await;
            } else {
                eprintln!("Shader path was not specifyed");
            }
        })
}
