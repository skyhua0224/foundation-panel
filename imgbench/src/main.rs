use image::{codecs::jpeg::JpegEncoder, imageops, DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;
use std::time::Instant;

fn synth(w: u32, h: u32) -> RgbaImage {
    let mut s: u64 = 0x2545F4914F6CDD1D;
    RgbaImage::from_fn(w, h, |x, y| {
        s ^= s << 13; s ^= s >> 7; s ^= s << 17;
        let n = (s & 0x1f) as u8;
        let r = ((x * 255 / w) as u8).wrapping_add(n);
        let g = ((y * 255 / h) as u8).wrapping_add(n);
        let b = (((x ^ y) & 0xff) as u8).wrapping_add(n);
        image::Rgba([r, g, b, 255])
    })
}
fn median(v: &mut Vec<f64>) -> f64 { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); v[v.len() / 2] }
fn main() {
    let opt = std::env::args().nth(1).unwrap_or_default();
    let src = synth(3840, 2160);
    let mut png = Vec::new();
    let mut rows: Vec<(&str, Vec<f64>)> = vec![("png_encode_4k", vec![]), ("png_decode_4k", vec![]), ("resize_triangle_4k_to_1920", vec![]), ("rgb8_jpeg_q60_encode_1920", vec![]), ("jpeg_decode_1920", vec![]), ("webp_decode_hero", vec![])];
    let small = imageops::resize(&src, 1920, 1080, imageops::FilterType::Triangle);
    let mut jpg = Vec::new();
    let mut webp_bytes = Vec::new();
    DynamicImage::ImageRgba8(imageops::resize(&src, 1024, 1024, imageops::FilterType::Triangle)).write_to(&mut Cursor::new(&mut webp_bytes), ImageFormat::WebP).unwrap();
    for _ in 0..9 {
        png.clear();
        let t = Instant::now(); DynamicImage::ImageRgba8(src.clone()).write_to(&mut Cursor::new(&mut png), ImageFormat::Png).unwrap(); rows[0].1.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now(); let d = image::load_from_memory_with_format(&png, ImageFormat::Png).unwrap().to_rgba8(); rows[1].1.push(t.elapsed().as_secs_f64() * 1e3); std::hint::black_box(&d);
        let t = Instant::now(); let r = imageops::resize(&src, 1920, 1080, imageops::FilterType::Triangle); rows[2].1.push(t.elapsed().as_secs_f64() * 1e3); std::hint::black_box(&r);
        jpg.clear();
        let t = Instant::now(); let rgb = DynamicImage::ImageRgba8(small.clone()).to_rgb8(); JpegEncoder::new_with_quality(&mut jpg, 60).encode_image(&rgb).unwrap(); rows[3].1.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now(); let d = image::load_from_memory(&jpg).unwrap().to_rgba8(); rows[4].1.push(t.elapsed().as_secs_f64() * 1e3); std::hint::black_box(&d);
        let t = Instant::now(); let d = image::load_from_memory(&webp_bytes).unwrap().to_rgba8(); rows[5].1.push(t.elapsed().as_secs_f64() * 1e3); std::hint::black_box(&d);
    }
    for (n, v) in rows.iter_mut() { println!("{opt},{n},{:.1}", median(v)); }
}
