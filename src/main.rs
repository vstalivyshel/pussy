mod bind;
mod capture;
mod ctx;
mod pp;
mod util;

use crate::{bind::Time, ctx::WgpuContext, util::Msg};

use notify::Watcher;
use std::{path::PathBuf, time};
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

async fn draw_headless(
    shader_path: PathBuf,
    number_of_frames: usize,
    resolution: &winit::dpi::PhysicalSize<u32>,
) -> Result<(), String> {
    let instance = wgpu::Instance::default();
    let init = crate::ctx::WgpuSetup::new(&instance, None).await;

    let mut bindings = crate::bind::ShaderBindings::new(&init.device);
    let shader_src = crate::pp::ShaderSource::validate(&shader_path, &bindings)?;

    let texture = crate::ctx::create_texture(&init.device, size);

    let bgl = bindings.create_bind_group_layout(&init.device);
    let pipeline = crate::ctx::create_render_pipeline(
        &init.device,
        &bgl,
        shader_src.as_str(),
        texture.format(),
    );

    let mut time = crate::util::TimeMeasure::new();
    let channel = crate::util::Channel::new();

    for _ in 0..number_of_frames {
        let bg = bindings.create_bind_group(&init.device);
        crate::ctx::FrameBuffer::new(
            &init.device,
            &init.queue,
            &texture,
            &pipeline,
            &bg,
        ).send_self(&channel.sender);

        // TODO: do something with this shit
        bindings.time.data = Time(time.start.elapsed().as_secs_f32());
        bindings.stage(&init.queue);
        //

        time.update();
    }

    channel.send(Msg::SaveMP4 { resolution, rate: time.delta });
    channel.sender.close().unwrap();
    channel.thread_handle.join().unwrap();

    Ok(())
}

async fn draw(shader_path: PathBuf) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("puss")
        // TODO: make this optional
        .with_inner_size(PhysicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("creating window");

    let (tx, rx) = std::sync::mpsc::channel();
    let watcher_config =
        notify::Config::default().with_poll_interval(std::time::Duration::from_millis(500));
    let mut watcher = notify::RecommendedWatcher::new(tx, watcher_config).unwrap();
    watcher
        .watch(&shader_path, notify::RecursiveMode::NonRecursive)
        .expect("initialize watcher");

    let mut ctx = WgpuContext::new(window, shader_path).await;
    let mut time = crate::util::TimeMeasure::new();
    let channel = crate::util::Channel::new();

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

                ctx.window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == ctx.window.id() => {
                match event {
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::F5),
                            ..
                        } => channel.send(Msg::SavePng {
                            buffer: ctx.render_into_frame_buffer(),

                        })
                        
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
                        } => {
                            if !capturing_frames && !frames_buffer.is_empty() {
                                let fb = frames_buffer.clone();
                                frames_buffer.clear();
                                let out = crate::util::current_time_string() + ".gif";
                                log::info!("Saving {out}");
                                std::thread::spawn(move || {
                                    match crate::capture::save_raw_frames_as_gif(&out, fb, &ctx.size) {
                                        Ok(()) => log::info!("{out} saved"),
                                        Err(e) => log::error!("{e}"),
                                    }
                                });
                            }
                        }
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::F8),
                            ..
                        } => {
                            if !capturing_frames && !frames_buffer.is_empty() {
                                let fb = frames_buffer.clone();
                                frames_buffer.clear();
                                let out = crate::util::current_time_string() + ".mp4";
                                log::info!(
                                    "Saving {out} [w: {width}; h: {height}; fr: {frame_rate}]",
                                    width = ctx.size.width,
                                    height = ctx.size.height
                                );
                                std::thread::spawn(move || {
                                    match crate::capture::save_raw_frames_as_mp4(
                                        &out, fb, &ctx.size, 60,
                                    ) {
                                        Ok(()) => log::info!("{out} saved"),
                                        Err(e) => log::error!("{e}"),
                                    }
                                });
                            }
                        }
                        _ => {}
                    },
                    WindowEvent::CloseRequested => *cf = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => ctx.resize(physical_size),
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        ctx.resize(new_inner_size)
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == ctx.window.id() => {
                // TODO: do something with this shit
                ctx.bindings.time.data = Time(time.start.elapsed().as_secs_f32());
                ctx.bindings.stage(&ctx.queue);
                //

                time.update();

                if capturing_frames {
                    ctx.render_into_frame_buffer().send_self(&channel.sender);
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
    env_logger::init();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 1 {
        let path = args.get(1).unwrap();
        pollster::block_on(draw_headless(
            PathBuf::from(path),
            120,
            &PhysicalSize::new(300, 300),
        ))
        .unwrap();
    } else {
        crate::util::clear_screen();
        let path = args.first().unwrap();
        pollster::block_on(draw(PathBuf::from(path)));
    }
}
