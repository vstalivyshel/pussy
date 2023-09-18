use chrono::offset::Local;
use image::codecs::png::PngEncoder;
use image::ImageEncoder;

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
