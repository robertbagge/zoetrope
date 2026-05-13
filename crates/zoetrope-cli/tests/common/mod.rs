use std::path::{Path, PathBuf};
use std::process::Command;

pub fn fixture(dir: &Path, name: &str, ext: &str) -> PathBuf {
    let path = dir.join(format!("{name}.{ext}"));
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=640x480:rate=30",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&path)
        .output()
        .expect("ffmpeg not available — required for tests");
    assert!(
        output.status.success(),
        "fixture generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    path
}

pub fn mov_fixture(dir: &Path) -> PathBuf {
    fixture(dir, "in", "mov")
}

/// Generate a synthetic still-image fixture of the requested size via
/// ffmpeg's `testsrc` source, capturing a single frame in the requested
/// format. Mirrors the video `fixture()` helper so image-input smoke tests
/// stay consistent with the rest of the suite.
pub fn image_fixture(dir: &Path, name: &str, ext: &str, size: (u32, u32)) -> PathBuf {
    let path = dir.join(format!("{name}.{ext}"));
    let (w, h) = size;

    // Homebrew's ffmpeg ships without a WebP encoder (zoetrope itself switched
    // to in-process libwebp for that reason). For .webp fixtures, generate the
    // pixel data via ffmpeg → PNG, then re-encode through the `image` crate's
    // lossless WebP encoder, which is bundled into the test binary.
    if ext.eq_ignore_ascii_case("webp") {
        let png_path = dir.join(format!("{name}.tmp.png"));
        let output = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("testsrc=duration=1:size={w}x{h}:rate=1"),
                "-frames:v",
                "1",
            ])
            .arg(&png_path)
            .output()
            .expect("ffmpeg not available — required for tests");
        assert!(
            output.status.success(),
            "image fixture (webp via png) generation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let decoded = image::open(&png_path).expect("decode intermediate png");
        decoded
            .save_with_format(&path, image::ImageFormat::WebP)
            .expect("encode webp fixture");
        let _ = std::fs::remove_file(&png_path);
        return path;
    }

    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=duration=1:size={w}x{h}:rate=1"),
            "-frames:v",
            "1",
        ])
        .arg(&path)
        .output()
        .expect("ffmpeg not available — required for tests");
    assert!(
        output.status.success(),
        "image fixture generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    path
}

pub fn png_fixture(dir: &Path, size: (u32, u32)) -> PathBuf {
    image_fixture(dir, "in", "png", size)
}

pub fn decode_gif(path: &Path) -> (u16, u16, usize) {
    let file = std::fs::File::open(path).expect("open gif");
    let mut decoder = gif::DecodeOptions::new()
        .read_info(file)
        .expect("read gif info");
    let (w, h) = (decoder.width(), decoder.height());
    let mut frames = 0;
    while decoder
        .read_next_frame()
        .expect("read next frame")
        .is_some()
    {
        frames += 1;
    }
    (w, h, frames)
}

/// Decode the dimensions of any still image format the `image` crate
/// understands (png, jpg, webp, gif first frame). Returns `(width, height)`.
pub fn decode_image_dims(path: &Path) -> (u32, u32) {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("decode {}: {e}", path.display()));
    (img.width(), img.height())
}

/// Returns true if the WebP file at `path` is an animated WebP (contains an
/// `ANIM` chunk in the RIFF container). A still WebP only has `VP8`/`VP8L`/
/// `VP8X` chunks. Used to assert that still-image inputs produce still
/// WebPs, not 1-frame animations.
pub fn is_animated_webp(path: &Path) -> bool {
    let bytes = std::fs::read(path).expect("read webp");
    // RIFF header is 12 bytes; chunks start at offset 12.
    // Each chunk: 4-byte FourCC + 4-byte LE size + payload (padded to even).
    // For animated WebPs the VP8X header's ANIM bit is set and an `ANIM`
    // chunk follows. A simple substring scan of the chunk-FourCC region is
    // sufficient here — these test fixtures are small.
    bytes.windows(4).any(|w| w == b"ANIM" || w == b"ANMF")
}
