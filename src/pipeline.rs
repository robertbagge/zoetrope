use std::path::Path;
use std::process::Command;

use crate::cli::{BatchPlan, Format, Options};
use crate::encode::{encode_gif, encode_webp, EncodeParams};
use crate::progress::probe_duration;

pub(crate) fn check_ffmpeg() -> Result<(), String> {
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
pub(crate) fn preflight(batch: &BatchPlan) -> Result<(), String> {
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

pub(crate) fn run(opts: &Options) -> Result<(), String> {
    // Probe once and cache — the fit loop may call encode up to 5 times,
    // but the input duration doesn't change between attempts.
    let probe_seconds = probe_duration(&opts.input);
    let params = initial_params(opts);
    match opts.max_size {
        Some(target) => crate::fit::fit_to_size(opts, params, target, probe_seconds)?,
        None => encode(opts, &params, probe_seconds)?,
    }
    report_done(&opts.output);
    Ok(())
}

pub(crate) fn encode(
    opts: &Options,
    params: &EncodeParams,
    probe_seconds: Option<f64>,
) -> Result<(), String> {
    match opts.format {
        Format::Gif => encode_gif(opts, params, probe_seconds),
        Format::Webp => encode_webp(opts, params, probe_seconds),
    }
}

pub(crate) fn initial_params(opts: &Options) -> EncodeParams {
    EncodeParams {
        width: opts.width,
        fps: opts.fps,
        quality: opts.encoder_quality,
    }
}

fn ffmpeg_has_encoder(name: &str) -> bool {
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

fn report_done(output: &Path) {
    let size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "done: {} ({:.1} MB)",
        output.display(),
        size as f64 / 1_048_576.0
    );
}
