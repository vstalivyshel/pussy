use anyhow::Context;
use image::{
    codecs::{
        gif::{GifEncoder, Repeat},
        png::PngEncoder,
    },
    ImageEncoder,
};

pub fn save_png(image_data: &[u8], width: u32, height: u32) -> anyhow::Result<()> {
    let target_file = crate::util::create_file_cwd(".png")?;
    PngEncoder::new(target_file)
        .write_image(image_data, width, height, image::ColorType::Rgba8)
        .context("Failed to write a .png image")
}

pub fn save_gif(frames: Vec<Vec<u8>>, speed: i32, width: u32, height: u32) -> anyhow::Result<()> {
    let target_file = crate::util::create_file_cwd(".gif")?;
    let mut encoder = GifEncoder::new_with_speed(target_file, speed);
    encoder.set_repeat(Repeat::Infinite)?;
    let frames = frames.into_iter().map(|f| {
        // TODO:
        let image_buffer = image::ImageBuffer::from_raw(width, height, f).unwrap();
        image::Frame::new(image_buffer)
    });

    encoder
        .encode_frames(frames)
        .context("Failed to ecnode the provided frames into .gif")
}
