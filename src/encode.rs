use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use tempfile::TempDir;

use crate::cli::{Options, Playback};

/// Tunable parameters the fit loop can vary between attempts.
/// Kept separate from `Options` so the loop can retry with different values
/// without rebuilding the whole config.
#[derive(Clone, Debug)]
pub(crate) struct EncodeParams {
    pub width: u32,
    pub fps: u32,
    /// Encoder quality knob — 0-100 for both gifski and libwebp.
    pub quality: u8,
}

pub(crate) fn encode_gif(opts: &Options, params: &EncodeParams) -> Result<(), String> {
    let tmp = TempDir::new().map_err(|e| format!("create tempdir: {e}"))?;
    let png_pattern = tmp.path().join("frame_%06d.png");

    eprintln!("extracting frames... ({}px, {}fps)", params.width, params.fps);
    extract_png_frames(opts, params, &png_pattern)?;

    let mut pngs: Vec<PathBuf> = std::fs::read_dir(tmp.path())
        .map_err(|e| format!("read frame dir: {e}"))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();
    pngs.sort();

    if pngs.is_empty() {
        return Err("ffmpeg produced no frames (check trim/speed settings)".into());
    }

    eprintln!("encoding gif... ({} frames)", pngs.len());
    encode_gif_with_gifski(pngs, &opts.output, params.width, params.fps, params.quality)
}

pub(crate) fn encode_webp(opts: &Options, params: &EncodeParams) -> Result<(), String> {
    eprintln!("encoding webp... ({}px, {}fps)", params.width, params.fps);
    let mut cmd = ffmpeg_base_command(opts, params.fps, params.width);
    cmd.args(["-c:v", "libwebp"]);
    cmd.args(["-loop", "0"]);
    cmd.args(["-quality", &params.quality.to_string()]);
    cmd.arg(&opts.output);
    run_ffmpeg(cmd, "webp encoding")
}

/// Builds the ffmpeg command shared by PNG extraction and WebP encoding:
/// `-y [-ss start] -i input [-t duration] -filter_complex <chain> -map [out]`.
/// Callers append output-specific flags and the output path.
fn ffmpeg_base_command(opts: &Options, fps: u32, width: u32) -> Command {
    let filter = build_filter_complex(fps, width, opts.speed, &opts.playback);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y");
    if let Some(start) = opts.start {
        cmd.args(["-ss", &format!("{start}")]);
    }
    if let Some(duration) = opts.duration {
        cmd.args(["-t", &format!("{duration}")]);
    }
    cmd.arg("-i").arg(&opts.input);
    cmd.args(["-filter_complex", &filter]);
    cmd.args(["-map", "[out]"]);
    cmd
}

fn extract_png_frames(
    opts: &Options,
    params: &EncodeParams,
    pattern: &Path,
) -> Result<(), String> {
    let mut cmd = ffmpeg_base_command(opts, params.fps, params.width);
    cmd.args(["-start_number", "0"]);
    cmd.arg(pattern);
    run_ffmpeg(cmd, "frame extraction")
}

fn build_filter_complex(fps: u32, width: u32, speed: Option<f64>, playback: &Playback) -> String {
    let mut chain = String::from("[0:v]");

    if let Some(s) = speed {
        chain.push_str(&format!("setpts=PTS/{s},"));
    }

    match playback {
        Playback::Normal => {}
        Playback::Reverse => chain.push_str("reverse,"),
        Playback::Boomerang => {
            chain.push_str("split[a][b];[b]reverse[br];[a][br]concat=n=2:v=1,");
        }
    }

    chain.push_str(&format!("fps={fps},scale={width}:-1:flags=lanczos[out]"));
    chain
}

fn run_ffmpeg(mut cmd: Command, stage: &str) -> Result<(), String> {
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run ffmpeg: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffmpeg {stage} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn encode_gif_with_gifski(
    pngs: Vec<PathBuf>,
    output: &Path,
    width: u32,
    fps: u32,
    quality: u8,
) -> Result<(), String> {
    use gifski::{progress::NoProgress, Repeat, Settings};

    let settings = Settings {
        width: Some(width),
        height: None,
        quality,
        fast: false,
        repeat: Repeat::Infinite,
    };

    let (collector, writer) = gifski::new(settings).map_err(|e| format!("gifski init: {e}"))?;

    let file = std::fs::File::create(output)
        .map_err(|e| format!("create output {}: {}", output.display(), e))?;

    let fps_f = fps as f64;

    thread::scope(|s| -> Result<(), String> {
        let collector_handle = s.spawn(move || -> Result<(), String> {
            for (i, path) in pngs.into_iter().enumerate() {
                let pts = i as f64 / fps_f;
                collector
                    .add_frame_png_file(i, path, pts)
                    .map_err(|e| format!("add frame {i}: {e}"))?;
            }
            Ok(())
        });

        let write_result = writer
            .write(file, &mut NoProgress {})
            .map_err(|e| format!("write gif: {e}"));

        let collect_result = collector_handle
            .join()
            .map_err(|_| "collector thread panicked".to_string())?;

        write_result?;
        collect_result
    })
}
