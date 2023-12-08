use crate::ctx::FrameBuffer;
use anyhow::Context;
use chrono::offset::Local;
use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
};
use notify::Watcher;
use std::{fs::File, io::Write, sync::mpsc, time::Instant};
use winit::dpi::PhysicalSize;

pub type RawFrame = Vec<u8>;

#[allow(dead_code)]
pub struct FileWatcher {
    pub receiver: mpsc::Receiver<notify::Result<notify::Event>>,
    watcher: notify::RecommendedWatcher,
}

impl FileWatcher {
    pub fn new(file: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(sender).context("Failed to init file watcher")?;

        watcher
            .watch(file.as_ref(), notify::RecursiveMode::NonRecursive)
            .context("Failed to spawn a file watcher")?;

        Ok(Self { receiver, watcher })
    }
}

pub enum Msg {
    Exit,
    ExtractData(FrameBuffer),
    SavePng {
        frame: FrameBuffer,
        resolution: PhysicalSize<u32>,
    },
    SaveMp4 {
        rate: u32,
        resolution: PhysicalSize<u32>,
    },
}

impl std::fmt::Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Msg::Exit => "Msg::Exit",
                Msg::ExtractData(_) => "Msg::ExtractData",
                Msg::SavePng { .. } => "Msg::SavePng",
                Msg::SaveMp4 { .. } => "Msg::SaveMp4",
            }
        )
    }
}

pub struct Channel {
    // wrapping a JoinHandle in Option becouse of weird
    // behaviour of ownership inside of the EventLoop
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
    pub sender: mpsc::Sender<Msg>,
}

impl Channel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let rt_handle = tokio::runtime::Handle::current();
        let thread_handle = std::thread::spawn(move || {
            let _ = rt_handle.enter();
            // without wrapping it into Option the tokio runtime will panic
            // with the "JoinHandle polled after completion" message on Msg::SaveMp4 request
            let mut tasks = Some(Vec::<tokio::task::JoinHandle<RawFrame>>::new());

            while let Ok(msg) = rx.recv() {
                log::info!("Accepted request {msg}");
                match msg {
                    Msg::Exit => break,
                    Msg::ExtractData(frame_buffer) => {
                        if let Some(ref mut ts) = tasks {
                            ts.push(rt_handle.spawn(async move {
                                frame_buffer.map_read().await;
                                let frame = frame_buffer.extract_data();
                                log::info!("Frame data is extracted");
                                frame
                            }));
                        }
                    }
                    Msg::SavePng { frame, resolution } => {
                        rt_handle.spawn(async move {
                            frame.map_read().await;
                            match crate::capture::save_raw_frame_as_png(
                                &frame.extract_data(),
                                &resolution,
                            ) {
                                Ok(file) => log::info!("{file} saved!"),
                                Err(e) => log::error!("{e}"),
                            }
                        });
                    }
                    Msg::SaveMp4 { rate, resolution } => {
                        let frames = rt_handle.block_on(async {
                            let tasks = tasks.take().unwrap();
                            let n_tasks = tasks.len();
                            let mut frames = Vec::<RawFrame>::with_capacity(n_tasks);
                            for frame in tasks.into_iter() {
                                match frame.await {
                                    Ok(f) => frames.push(f),
                                    Err(e) => log::error!(
                                        "Error extracting data from a frame buffer: {e}"
                                    ),
                                }
                            }
                            log::info!(
                                "{n_frames}/{n_tasks} frames are extracted",
                                n_frames = frames.len(),
                            );

                            frames
                        });

                        // clear the tasks pool
                        tasks = Some(Vec::new());

                        rt_handle.spawn(async move {
                            match crate::capture::save_raw_frames_as_mp4(
                                frames,
                                &resolution,
                                rate as _,
                            ) {
                                Ok(file) => log::info!("{file} saved!"),
                                Err(e) => log::error!("{e}"),
                            }
                        });
                    }
                }
            }
        });

        Self {
            thread_handle: Some(thread_handle),
            sender: tx,
        }
    }

    pub fn send_msg(&self, msg: Msg) {
        log::info!("Requested {msg}");
        if let Err(m) = self.sender.send(msg) {
            if let Msg::Exit = m.0 {
                log::warn!("Channel is closed, forcing shutdown");
                std::process::exit(1);
            } else {
                log::error!("Channel is closed, {m} wasn't accepted", m = m.0);
            }
        }
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        log::info!("Dropping Channel");
        self.send_msg(Msg::Exit);
        if let Err(e) = self.thread_handle.take().unwrap().join() {
            log::warn!("Channel's thread wasn't closed successfully: {e:?}")
        }
    }
}

pub struct Time {
    pub start: Instant,
    pub delta: f32,
    pub frame_count: u32,
    pub accum_time: f32,
    pub last_frame_inst: Instant,
}

impl Time {
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
    pub width: u32,
    pub height: u32,
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
            width,
            height,
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
