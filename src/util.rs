use anyhow::Context;
use chrono::offset::Local;
use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
};
use std::{fs::File, io::Write, sync::mpsc, time::Instant};
use winit::dpi::PhysicalSize;

pub type RawFrame = Vec<u8>;
pub type FrameBufferSender = mpsc::Sender<Result<Msg, wgpu::BufferAsyncError>>;

pub enum Msg {
    ExtractData(crate::ctx::FrameBuffer),
    SavePng {
        buffer: crate::ctx::FrameBuffer,
        resolution: PhysicalSize<u32>,
    },
    SaveMp4 {
        rate: u32,
        resolution: PhysicalSize<u32>,
    },
    SaveGif {
        resolution: PhysicalSize<u32>,
    },
}

pub struct Channel {
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
    pub sender: FrameBufferSender,
}

impl Channel {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread_handle = std::thread::spawn(move || {
            let mut frames = Vec::<crate::util::RawFrame>::new();
            while let Ok(msg) = receiver.recv() {
                match msg {
                    Ok(Msg::ExtractData(buffer)) => frames.push(buffer.extract_data()),
                    Ok(Msg::SavePng { buffer, resolution }) => {
                        match crate::capture::save_raw_frame_as_png(
                            &buffer.extract_data(),
                            &resolution,
                        ) {
                            Ok(()) => log::info!("png saved"),
                            Err(e) => log::error!("{e}"),
                        }
                    }
                    Ok(Msg::SaveMp4 { resolution, rate }) => {
                        match crate::capture::save_raw_frames_as_mp4(
                            frames.clone(),
                            &resolution,
                            rate as _,
                        ) {
                            Ok(()) => log::info!("mp4 saved"),
                            Err(e) => log::error!("{e}"),
                        }
                        frames.clear();
                    }
                    Ok(Msg::SaveGif { resolution }) => {
                        match crate::capture::save_raw_frames_as_gif(frames.clone(), &resolution) {
                            Ok(()) => log::info!("Gif saved"),
                            Err(e) => log::error!("{e}"),
                        }
                        frames.clear();
                    }
                    Err(e) => log::error!("Error receiving message from main thread:\n\t{e}"),
                }
            }
        });

        Self {
            thread_handle: Some(thread_handle),
            sender,
        }
    }
    pub fn send_msg(&self, msg: Msg) {
        match msg {
            Msg::ExtractData(buffer) => {
                let buffer_slice = buffer.buffer.slice(..);
                let sender = self.sender.clone();
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    sender
                        .send(result.map(|_| Msg::ExtractData(buffer)))
                        .unwrap();
                });
            }
            Msg::SavePng { buffer, resolution } => {
                let buffer_slice = buffer.buffer.slice(..);
                let sender = self.sender.clone();
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    sender
                        .send(result.map(|_| Msg::SavePng { buffer, resolution }))
                        .unwrap();
                });
            }
            _ => self.sender.send(Ok(msg)).unwrap(),
        }
    }

    pub fn finish(&mut self) {
        self.thread_handle.take().unwrap().join().unwrap();
    }
}

pub struct TimeMeasure {
    pub start: Instant,
    pub delta: f32,
    pub frame_count: u32,
    pub accum_time: f32,
    pub last_frame_inst: Instant,
}

impl TimeMeasure {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            delta: 0.0,
            frame_count: 0,
            accum_time: 0.0,
            last_frame_inst: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        self.accum_time += self.last_frame_inst.elapsed().as_secs_f32();
        self.last_frame_inst = Instant::now();
        self.frame_count += 1;
        if self.frame_count == 10 {
            self.delta = self.frame_count as f32 / self.accum_time;
            log::debug!("rate: {r}", r = self.delta);
            self.accum_time = 0.0;
            self.frame_count = 0;
        }
    }
}

#[derive(Clone, Copy)]
pub struct AllignedBufferSize {
    pub buffer_size: u32,
    pub padded_bytes_per_row: u32,
    pub unpadded_bytes_per_row: u32,
}

impl AllignedBufferSize {
    pub fn new(width: u32, height: u32) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>() as u32;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;
        let buffer_size = padded_bytes_per_row * height;

        Self {
            buffer_size,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
        }
    }
}

impl From<&PhysicalSize<u32>> for AllignedBufferSize {
    fn from(size: &PhysicalSize<u32>) -> Self {
        Self::new(size.width, size.height)
    }
}

pub fn current_time_string() -> String {
    Local::now().time().format("%H-%M-%S-%3f").to_string()
}

pub fn clear_screen() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, MoveTo(0, 0), Clear(ClearType::All));
    let _ = stdout.flush();
}

pub fn create_file_cwd(file_name: &str) -> anyhow::Result<File> {
    let mut target = std::env::current_dir().context("Failed to get current dir")?;
    target.push(file_name);

    let file = File::create(target)?;

    Ok(file)
}
