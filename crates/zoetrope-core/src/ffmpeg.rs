use std::path::Path;
use std::process::Command;

use crate::options::BatchPlan;
use crate::settings::Format;

pub fn check_ffmpeg() -> Result<(), String> {
    match Command::new("ffmpeg").arg("-version").output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => Err("ffmpeg found but returned an error".into()),
        Err(_) => Err(
            "ffmpeg not found — install it (e.g. `brew install ffmpeg` on macOS, \
             `apt install ffmpeg` on Debian/Ubuntu, or see https://ffmpeg.org/)"
                .into(),
        ),
    }
}

/// Run once before the batch loop. Catches environment problems that would
/// otherwise fail per-file (e.g. ffmpeg built without libwebp when a WebP
/// output is requested), so the user sees one clean error instead of N.
pub fn preflight(batch: &BatchPlan) -> Result<(), String> {
    let needs_webp = batch.options.iter().any(|o| o.format == Format::Webp);
    if needs_webp && !ffmpeg_has_encoder("libwebp") {
        return Err(
            "ffmpeg was built without libwebp — install one that includes it \
             (e.g. `brew install ffmpeg-full` on macOS, standard `ffmpeg` on Ubuntu)"
                .into(),
        );
    }
    Ok(())
}

pub fn ffmpeg_has_encoder(name: &str) -> bool {
    let out = match Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };
    // Encoder lines look like: " V....D libwebp              libwebp WebP image"
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|line| line.split_whitespace().nth(1) == Some(name))
}

/// Probe the input's duration in seconds via ffprobe. Returns `None` if
/// ffprobe is missing or the output can't be parsed — progress then falls
/// back to a spinner. ffprobe is a soft dependency, not required for
/// encoding.
pub fn probe_duration(input: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
}
