use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::{DynamicImage, ImageFormat};

use crate::options::Options;
use crate::progress::ProgressReporter;
use crate::settings::Format;

/// One-shot still-image conversion: decode → resize → encode. Bypasses ffmpeg
/// entirely; the input is a single image file (png/jpg/jpeg/webp), and the
/// output is a single still image in the requested format.
pub fn encode_still_image(
    opts: &Options,
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    reporter.start_phase("converting", Some(1));

    let img =
        image::open(&opts.input).map_err(|e| format!("decode {}: {e}", opts.input.display()))?;

    let (target_w, target_h) = compute_target_size(
        img.width(),
        img.height(),
        opts.width_user_set,
        opts.width,
        opts.height,
    )?;

    let resized = if (target_w, target_h) == (img.width(), img.height()) {
        img
    } else {
        img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
    };

    match opts.format {
        Format::Png => save_png(&resized, &opts.output)?,
        Format::Jpeg => save_jpeg(&resized, &opts.output, opts.encoder_quality)?,
        Format::Webp => save_webp(&resized, &opts.output, opts.encoder_quality)?,
        Format::Gif => save_gif_single_frame(&resized, &opts.output)?,
    }

    reporter.set_position(1);
    reporter.finish_phase();
    Ok(())
}

/// Resolve the output dimensions from the user's flags and the source size.
///
/// - both width and height supplied → stretch to exactly W×H
/// - only width supplied → preserve aspect from width
/// - only height supplied → preserve aspect from height
/// - neither supplied → use the preset width (already resolved into `opts.width`)
///   and preserve aspect
pub fn compute_target_size(
    src_w: u32,
    src_h: u32,
    width_user_set: bool,
    width: u32,
    height: Option<u32>,
) -> Result<(u32, u32), String> {
    if src_w == 0 || src_h == 0 {
        return Err(format!("source image has zero dimension ({src_w}x{src_h})"));
    }
    match (width_user_set, height) {
        (true, Some(h)) => Ok((width.max(1), h.max(1))),
        (true, None) => Ok((width.max(1), scale_dim(src_h, width, src_w).max(1))),
        (false, Some(h)) => Ok((scale_dim(src_w, h, src_h).max(1), h.max(1))),
        (false, None) => Ok((width.max(1), scale_dim(src_h, width, src_w).max(1))),
    }
}

/// `dim * num / den`, rounded to nearest, used to compute the aspect-preserved
/// other dimension when only one is fixed.
fn scale_dim(dim: u32, num: u32, den: u32) -> u32 {
    ((dim as u64 * num as u64 + (den as u64 / 2)) / den as u64) as u32
}

fn save_png(img: &DynamicImage, output: &Path) -> Result<(), String> {
    img.save_with_format(output, ImageFormat::Png)
        .map_err(|e| format!("write png {}: {e}", output.display()))
}

fn save_jpeg(img: &DynamicImage, output: &Path, quality: u8) -> Result<(), String> {
    let file = File::create(output).map_err(|e| format!("create {}: {e}", output.display()))?;
    let mut writer = BufWriter::new(file);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality);
    encoder
        .encode_image(img)
        .map_err(|e| format!("write jpeg {}: {e}", output.display()))
}

fn save_webp(img: &DynamicImage, output: &Path, quality: u8) -> Result<(), String> {
    // libwebp's lossy encoder takes RGBA8. Convert up front so we don't depend
    // on the input's color type matching.
    let rgba = img.to_rgba8();
    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height());
    let mem = encoder.encode(quality as f32);
    std::fs::write(output, &*mem).map_err(|e| format!("write webp {}: {e}", output.display()))
}

fn save_gif_single_frame(img: &DynamicImage, output: &Path) -> Result<(), String> {
    use image::codecs::gif::GifEncoder;
    use image::Frame;

    let rgba = img.to_rgba8();
    let file = File::create(output).map_err(|e| format!("create {}: {e}", output.display()))?;
    let mut encoder = GifEncoder::new(BufWriter::new(file));
    encoder
        .encode_frame(Frame::new(rgba))
        .map_err(|e| format!("write gif {}: {e}", output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_size_stretch_when_both_set() {
        // Both user-set → stretch
        assert_eq!(
            compute_target_size(2048, 1024, true, 432, Some(432)).unwrap(),
            (432, 432)
        );
    }

    #[test]
    fn compute_size_aspect_from_width() {
        assert_eq!(
            compute_target_size(2048, 1024, true, 432, None).unwrap(),
            (432, 216)
        );
    }

    #[test]
    fn compute_size_aspect_from_height() {
        assert_eq!(
            compute_target_size(2048, 1024, false, 999, Some(216)).unwrap(),
            (432, 216)
        );
    }

    #[test]
    fn compute_size_preset_default_aspect_preserved() {
        // Neither user-set → preset width, aspect-preserved.
        assert_eq!(
            compute_target_size(2048, 1024, false, 480, None).unwrap(),
            (480, 240)
        );
    }

    #[test]
    fn compute_size_zero_source_rejected() {
        assert!(compute_target_size(0, 100, true, 100, None).is_err());
        assert!(compute_target_size(100, 0, true, 100, None).is_err());
    }

    #[test]
    fn resize_exact_stretches_to_target() {
        // 4×2 synthetic image → 2×2 output. resize_exact must not preserve
        // aspect; we should see exactly 2×2 in the encoded PNG.
        use image::{ImageBuffer, Rgba};
        let img = DynamicImage::ImageRgba8(ImageBuffer::from_fn(4, 2, |x, _y| {
            Rgba([x as u8 * 60, 0, 0, 255])
        }));
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.png");

        let opts = Options {
            input: dir.path().join("synthetic.png"),
            output: out.clone(),
            format: Format::Png,
            encoder_quality: 90,
            fps: 12,
            width: 2,
            height: Some(2),
            width_user_set: true,
            speed: None,
            playback: crate::settings::Playback::Normal,
            start: None,
            duration: None,
            max_size: None,
        };
        img.save_with_format(&opts.input, ImageFormat::Png).unwrap();

        let mut reporter = crate::progress::NoopReporter;
        encode_still_image(&opts, &mut reporter).unwrap();

        let decoded = image::open(&out).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
    }
}
