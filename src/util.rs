use anyhow::Context;
use chrono::offset::Local;
use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
};
use std::{fs::File, io::Write};
use winit::dpi::PhysicalSize;

pub type RawFrame = Vec<u8>;
pub type FrameBufferSender = OneshotSender<Result<wgpu::Buffer, wgpu::BufferAsyncError>>;

#[Clone, Copy]
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
