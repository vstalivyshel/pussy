mod bind;
mod capture;
mod ctx;
mod pp;
mod utils;

use crate::{ctx::WgpuContext, utils::Msg};
use wgpu_text::{
    glyph_brush::{
        BuiltInLineBreaker,
        Layout,
        Section,
        Text,
        VerticalAlign,
    },
    BrushBuilder,
};
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
    let file_watcher = match crate::utils::FileWatcher::new(&shader_path) {
        Ok(watcher) => watcher,
        Err(e) => return eprintln!("{e}"),
    };
    let channel = crate::utils::Channel::new();
    let mut ctx = WgpuContext::new(window, shader_path).await;
    let mut time = crate::utils::Time::new();
    let mut capturing_frames = false;

    // TODO:
    let font = include_bytes!("font.ttf");
    let mut text_brush = BrushBuilder::using_font_bytes(font).unwrap().build(
        &ctx.device,
        ctx.config.width,
        ctx.config.height,
        ctx.config.format,
    );
    let font_size = 25.;
    let mut section = Section::default()
        .add_text(
            Text::new("Test text")
                .with_scale(font_size)
                .with_color([0.9, 0.5, 0.5, 1.0]),
        )
        .with_bounds((ctx.config.width as f32 * 0.4, ctx.config.height as f32))
        .with_layout(
            Layout::default()
                .v_align(VerticalAlign::Center)
                .line_breaker(BuiltInLineBreaker::AnyCharLineBreaker)
        )
        .with_screen_position((50., ctx.config.height as f32 * 0.5))
        .to_owned();

    event_loop.run(move |ev, _, cf| {
        *cf = ControlFlow::Poll;

        match ev {
            Event::MainEventsCleared => {
                // TODO: handle errors and other events
                while let Ok(Ok(event)) = file_watcher.receiver.try_recv() {
                    if let notify::event::EventKind::Modify(_) = event.kind {
                        crate::utils::clear_screen();
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
                        frame: ctx.render_into_frame_buffer(&text_brush),
                        resolution: ctx.resolution,
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
                        resolution: ctx.resolution,
                    }),
                    _ => {}
                },
                WindowEvent::CloseRequested => {
                    log::info!("Forced shutdown");
                    std::process::exit(0);
                }
                WindowEvent::Resized(new_size) => {
                    // TODO:
                    ctx.resize(new_size);
                    section.bounds = (ctx.config.width as f32 * 0.4, ctx.config.height as f32);
                    section.screen_position.1 = ctx.config.height as f32 * 0.5;
                    text_brush.resize_view(ctx.config.width as f32, ctx.config.height as f32, &ctx.queue);
                },
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    // TODO:
                    ctx.resize(new_inner_size);
                    section.bounds = (ctx.config.width as f32 * 0.4, ctx.config.height as f32);
                    section.screen_position.1 = ctx.config.height as f32 * 0.5;
                    text_brush.resize_view(ctx.config.width as f32, ctx.config.height as f32, &ctx.queue);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = position.cast::<f32>();
                    ctx.bindings.mouse.update(&ctx.queue, [pos.x, pos.y])
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == ctx.window.id() => {
                let q = &ctx.queue;

                text_brush.queue(&ctx.device, &ctx.queue, vec![&section]).unwrap();

                let res = ctx.resolution.cast::<f32>();
                ctx.bindings
                    .time
                    .update(q, time.start.elapsed().as_secs_f32());
                ctx.bindings.resolution.update(q, [res.width, res.height]);
                time.update();

                if capturing_frames {
                    channel.send_msg(Msg::ExtractData(ctx.render_into_frame_buffer(&text_brush)));
                }

                match ctx.render_frame(&text_brush) {
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
            crate::utils::clear_screen();
            if let Some(shader) = std::env::args().nth(1) {
                draw(&shader).await;
            } else {
                eprintln!("ERROR: Shader path was not specifyed");
                std::process::exit(1);
            }
        })
}
