use egui::ColorImage;
use std::sync::OnceLock;

const LOGO_PNG: &[u8] = include_bytes!("../../images/logo.png");
static LOGO_IMAGE: OnceLock<ColorImage> = OnceLock::new();

fn decode_logo(bytes: &[u8]) -> ColorImage {
    let rgba = image::load_from_memory(bytes)
        .expect("embedded logo png")
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = rgba.into_raw();
    ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels)
}

pub fn logo_color_image() -> &'static ColorImage {
    LOGO_IMAGE.get_or_init(|| decode_logo(LOGO_PNG))
}

pub fn logo_png_bytes() -> &'static [u8] {
    LOGO_PNG
}

pub fn logo_dimensions() -> [usize; 2] {
    logo_color_image().size
}
