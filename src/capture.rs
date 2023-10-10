use crate::util::RawFrame;
use anyhow::Context;
use image::{
    codecs::{
        gif::{GifEncoder, Repeat},
        png::PngEncoder,
    },
    ImageEncoder,
};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use winit::dpi::PhysicalSize;

pub fn save_raw_frame_as_png(frame: &[u8], size: &PhysicalSize<u32>) -> anyhow::Result<()> {
    let out_name = crate::util::current_time_string() + ".png";
    log::info!("Saving png as {out_name}");
    let target_file = crate::util::create_file_cwd(&out_name)
        .context("Failed to create file for saving raw buffer as png")?;
    PngEncoder::new(target_file)
        .write_image(frame, size.width, size.height, image::ColorType::Rgba8)
        .context("Failed to save raw frame as png")
}

pub fn save_raw_frames_as_gif(
    frames: Vec<RawFrame>,
    size: &PhysicalSize<u32>,
) -> anyhow::Result<()> {
    let out_name = crate::util::current_time_string() + ".gif";
    log::info!("Saving gif as {out_name}");
    let target_file = crate::util::create_file_cwd(&out_name)
        .context("Failed to create file for saving raw buffer as gif")?;
    let mut encoder = GifEncoder::new_with_speed(target_file, 10);
    encoder
        .set_repeat(Repeat::Infinite)
        .context("GifEncoder::set_repeat(Repeat::Infinite) failed")?;
    let frames = frames.into_iter().map(|f| {
        let image_buffer = image::ImageBuffer::from_raw(size.width, size.height, f).unwrap();
        image::Frame::new(image_buffer)
    });

    encoder
        .encode_frames(frames)
        .context("Failed to save raw frames as gif")
}

#[rustfmt::skip]
pub fn save_raw_frames_as_mp4(frames: Vec<RawFrame>, size: &PhysicalSize<u32>, rate: u32) -> anyhow::Result<()> {
    let out_name = crate::util::current_time_string() + ".mp4";
    log::info!("Saving video as {out_name}");
    let size = format!("{width}x{height}", width = size.width, height = size.height);
    let rate = format!("{rate}");
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            // overwrite file if it already exists
            "-y",
            // accept raw data from stdin
            "-f", "rawvideo",
            "-pix_fmt", "rgba",
            "-s", &size,
            // frame rate
            "-r", &rate,
            // don't expect any audio in the stream
            "-an",
            // get the data from stdin
            "-i", "-",
            // encode to h264
            "-c:v", "libx264",
            &out_name
        ])
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn ffmpeg")?;

    let stdin = ffmpeg.stdin.as_mut()
        .context("Failed to get ffmpeg's stdin")?;

    let frames = frames.into_iter().flatten().collect::<Vec<u8>>();
    stdin.write_all(&frames)?;
    let _ = ffmpeg.wait_with_output()?;

    Ok(())
}
