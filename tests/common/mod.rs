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
