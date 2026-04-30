use std::path::Path;
use std::process::Command;

use crate::options::BatchPlan;

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

/// Run once before the batch loop. Currently a no-op — kept as a hook for
/// future environment checks that would otherwise fail per-file. (The libwebp
/// check that previously lived here is gone: WebP encoding now uses the
/// statically-linked libwebp bundled into the binary, not ffmpeg's encoder.)
pub fn preflight(_batch: &BatchPlan) -> Result<(), String> {
    Ok(())
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
