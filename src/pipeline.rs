use std::path::Path;
use std::process::Command;

use crate::cli::{Format, Options};
use crate::encode::{encode_gif, encode_webp, EncodeParams};

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

pub(crate) fn run(opts: &Options) -> Result<(), String> {
    if opts.format == Format::Webp && !ffmpeg_has_encoder("libwebp") {
        return Err(
            "ffmpeg was built without libwebp — install one that includes it \
             (e.g. `brew install ffmpeg-full` on macOS, standard `ffmpeg` on Ubuntu)"
                .into(),
        );
    }

    let params = initial_params(opts);
    match opts.max_size {
        Some(target) => crate::fit::fit_to_size(opts, params, target)?,
        None => encode(opts, &params)?,
    }
    report_done(&opts.output);
    Ok(())
}

pub(crate) fn encode(opts: &Options, params: &EncodeParams) -> Result<(), String> {
    match opts.format {
        Format::Gif => encode_gif(opts, params),
        Format::Webp => encode_webp(opts, params),
    }
}

pub(crate) fn initial_params(opts: &Options) -> EncodeParams {
    let q = opts.quality.settings();
    let quality = match opts.format {
        Format::Gif => q.gifski_quality,
        Format::Webp => q.webp_quality,
    };
    EncodeParams {
        width: opts.width,
        fps: opts.fps,
        quality,
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
