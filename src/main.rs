use fast_image_resize as fr;
use image::ImageReader;
use std::{env, error::Error};

#[derive(Clone, Copy, Debug)]
enum Mode {
    Fit,
    Fill,
}

impl Mode {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "fit" | "pad" | "contain" => Some(Self::Fit),
            "fill" | "crop" | "cover" => Some(Self::Fill),
            _ => None,
        }
    }
}

fn parse_size(s: &str) -> Option<(u32, u32)> {
    if s.eq_ignore_ascii_case("youtube") || s.eq_ignore_ascii_case("yt") {
        return Some((1280, 720));
    }

    let (w, h) = s.split_once('x')?;
    let w: u32 = w.parse().ok()?;
    let h: u32 = h.parse().ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    Some((w, h))
}

fn resize_rgba8(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Result<image::RgbaImage, Box<dyn Error>> {
    let src_image = fr::images::Image::from_vec_u8(
        src_w,
        src_h,
        src_rgba.to_vec(),
        fr::PixelType::U8x4,
    )?;

    let mut dst_image = fr::images::Image::new(dst_w, dst_h, fr::PixelType::U8x4);

    let mut resizer = fr::Resizer::new();
    let mut options = fr::ResizeOptions::default();
    options.algorithm = fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3);
    resizer.resize(&src_image, &mut dst_image, &options)?;

    let out = image::RgbaImage::from_raw(dst_w, dst_h, dst_image.buffer().to_vec())
        .ok_or("Error creating final buffer")?;
    Ok(out)
}

fn resize_to_target(
    input_path: &str,
    output_path: &str,
    target_w: u32,
    target_h: u32,
    mode: Mode,
) -> Result<(), Box<dyn Error>> {
    let img = ImageReader::open(input_path)?.decode()?;
    let src_w = img.width();
    let src_h = img.height();
    if src_w == 0 || src_h == 0 {
        return Err("Image has invalid dimensions".into());
    }

    let scale_fit = (target_w as f64 / src_w as f64).min(target_h as f64 / src_h as f64);
    let scale_fill = (target_w as f64 / src_w as f64).max(target_h as f64 / src_h as f64);
    let scale = match mode {
        Mode::Fit => scale_fit,
        Mode::Fill => scale_fill,
    };

    let mut resized_w = ((src_w as f64) * scale).round().max(1.0) as u32;
    let mut resized_h = ((src_h as f64) * scale).round().max(1.0) as u32;

    if matches!(mode, Mode::Fill) {
        resized_w = resized_w.max(target_w);
        resized_h = resized_h.max(target_h);
    }

    let src_rgba = img.to_rgba8().into_raw();
    let resized = resize_rgba8(&src_rgba, src_w, src_h, resized_w, resized_h)?;

    let out: image::RgbaImage = match mode {
        Mode::Fit => {
            let background = image::Rgba([255, 255, 255, 255]);
            let mut canvas = image::RgbaImage::from_pixel(target_w, target_h, background);
            let x = ((target_w - resized_w) / 2) as i64;
            let y = ((target_h - resized_h) / 2) as i64;
            image::imageops::overlay(&mut canvas, &resized, x, y);
            canvas
        }
        Mode::Fill => {
            let crop_x = (resized_w - target_w) / 2;
            let crop_y = (resized_h - target_h) / 2;
            image::imageops::crop_imm(&resized, crop_x, crop_y, target_w, target_h).to_image()
        }
    };

    out.save(output_path)?;
    println!(
        "Saved: {} ({}x{}, mode={:?})",
        output_path, target_w, target_h, mode
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: cargo run -- <input> <output> [1280x720|youtube] [fit|fill]\n\
            Examples:\n\
            - cargo run -- in.png out.png\n\
            - cargo run -- in.png out.png 1920x1080 fit\n\
            - cargo run -- in.png out.png youtube fill"
        );
        return Ok(());
    }

    let (target_w, target_h) = args.get(3).and_then(|s| parse_size(s)).unwrap_or((1280, 720));
    let mode = args
        .get(4)
        .and_then(|s| Mode::parse(s))
        .unwrap_or(Mode::Fit);

    resize_to_target(&args[1], &args[2], target_w, target_h, mode)
}