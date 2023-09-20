use chrono::offset::Local;
use image::{
    codecs::{
        gif::{GifEncoder, Repeat},
        png::PngEncoder,
    },
    ImageEncoder,
};

pub fn save_png(image_data: &[u8], width: u32, height: u32) {
    // TODO: handle unwraps
    let mut target = std::env::current_dir().unwrap();
    let file_name = format!("{name}.png", name = Local::now().time());
    target.push(file_name);
    let target_file = std::fs::File::create(target).unwrap();
    PngEncoder::new(target_file)
        .write_image(image_data, width, height, image::ColorType::Rgba8)
        .unwrap()
}

pub fn save_gif(frames: Vec<Vec<u8>>, speed: i32, width: u32, height: u32) {
    let mut target = std::env::current_dir().unwrap();
    let file_name = format!("{name}.gif", name = Local::now().time());
    target.push(file_name);
    let target_file = std::fs::File::create(target).unwrap();
    let mut encoder = GifEncoder::new_with_speed(target_file, speed);
    encoder.set_repeat(Repeat::Infinite).unwrap();
    let frames = frames
        .into_iter()
        .map(|f| image::Frame::new(image::ImageBuffer::from_raw(width, height, f).unwrap()));
    encoder.encode_frames(frames).unwrap()
}
