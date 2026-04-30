use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use tempfile::TempDir;

use crate::options::Options;
use crate::progress::ProgressReporter;
use crate::settings::Playback;

/// Tunable parameters the fit loop can vary between attempts.
/// Kept separate from `Options` so the loop can retry with different values
/// without rebuilding the whole config.
#[derive(Clone, Debug)]
pub struct EncodeParams {
    pub width: u32,
    pub fps: u32,
    /// Encoder quality knob — 0-100 for both gifski and libwebp.
    pub quality: u8,
}

pub fn encode_gif(
    opts: &Options,
    params: &EncodeParams,
    probe_seconds: Option<f64>,
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    let tmp = TempDir::new().map_err(|e| format!("create tempdir: {e}"))?;
    let png_pattern = tmp.path().join("frame_%06d.png");

    let total_us = effective_duration_us(opts, probe_seconds);
    reporter.start_phase("extracting", total_us);
    let extract = extract_png_frames(opts, params, &png_pattern, reporter);
    reporter.finish_phase();
    extract?;

    let mut pngs: Vec<PathBuf> = std::fs::read_dir(tmp.path())
        .map_err(|e| format!("read frame dir: {e}"))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();
    pngs.sort();

    if pngs.is_empty() {
        return Err("ffmpeg produced no frames (check trim/speed settings)".into());
    }

    encode_gif_with_gifski(
        pngs,
        &opts.output,
        params.width,
        params.fps,
        params.quality,
        reporter,
    )
}

pub fn encode_webp(
    opts: &Options,
    params: &EncodeParams,
    probe_seconds: Option<f64>,
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    let tmp = TempDir::new().map_err(|e| format!("create tempdir: {e}"))?;
    let png_pattern = tmp.path().join("frame_%06d.png");

    let total_us = effective_duration_us(opts, probe_seconds);
    reporter.start_phase("extracting", total_us);
    let extract = extract_png_frames(opts, params, &png_pattern, reporter);
    reporter.finish_phase();
    extract?;

    let mut pngs: Vec<PathBuf> = std::fs::read_dir(tmp.path())
        .map_err(|e| format!("read frame dir: {e}"))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();
    pngs.sort();

    if pngs.is_empty() {
        return Err("ffmpeg produced no frames (check trim/speed settings)".into());
    }

    encode_webp_with_webp_animation(pngs, &opts.output, params.fps, params.quality, reporter)
}

/// Effective duration of the encoded *output* in microseconds. ffmpeg's
/// `-progress` reports `out_time_us` which advances in output time, so:
///   - trim shortens it (min(clip_duration, duration))
///   - --speed 2 halves it
///   - boomerang doubles it
fn effective_duration_us(opts: &Options, probe_seconds: Option<f64>) -> Option<u64> {
    let start = opts.start.unwrap_or(0.0);
    let after_start = probe_seconds.map(|d| (d - start).max(0.0));
    let clip = match (after_start, opts.duration) {
        (Some(a), Some(d)) => Some(a.min(d)),
        (Some(a), None) => Some(a),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    }?;

    let speed_adjusted = clip / opts.speed.unwrap_or(1.0);
    let playback_factor = match opts.playback {
        Playback::Boomerang => 2.0,
        _ => 1.0,
    };
    let us = speed_adjusted * playback_factor * 1_000_000.0;
    if us.is_finite() && us > 0.0 {
        Some(us as u64)
    } else {
        None
    }
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
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    let mut cmd = ffmpeg_base_command(opts, params.fps, params.width);
    cmd.args(["-start_number", "0"]);
    cmd.arg(pattern);
    run_ffmpeg_with_progress(cmd, "frame extraction", reporter)
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

/// Run ffmpeg with `-progress pipe:1 -nostats`, streaming `out_time_us=N`
/// key-value lines and ticking the reporter. Stderr is drained on a scoped
/// thread so a verbose encoder can't deadlock by filling its stderr pipe
/// while we block on stdout.
fn run_ffmpeg_with_progress(
    mut cmd: Command,
    stage: &str,
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    use std::io::Read;

    cmd.args(["-progress", "pipe:1", "-nostats"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to run ffmpeg: {e}"))?;

    let stdout = child.stdout.take().ok_or("ffmpeg stdout missing")?;
    let mut stderr = child.stderr.take().ok_or("ffmpeg stderr missing")?;

    let stderr_buf = thread::scope(|s| -> Result<Vec<u8>, String> {
        let stderr_handle = s.spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf);
            buf
        });

        let read = BufReader::new(stdout);
        for line in read.lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix("out_time_us=") {
                if let Ok(us) = rest.trim().parse::<u64>() {
                    reporter.set_position(us);
                }
            }
        }

        stderr_handle
            .join()
            .map_err(|_| "stderr drain thread panicked".to_string())
    })?;

    let status = child.wait().map_err(|e| format!("ffmpeg wait: {e}"))?;
    if !status.success() {
        return Err(format!(
            "ffmpeg {stage} failed:\n{}",
            String::from_utf8_lossy(&stderr_buf)
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
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    use gifski::{Repeat, Settings};

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
    let total_frames = pngs.len() as u64;

    reporter.start_phase("encoding gif", Some(total_frames));

    let result = thread::scope(|s| -> Result<(), String> {
        let collector_handle = s.spawn(move || -> Result<(), String> {
            for (i, path) in pngs.into_iter().enumerate() {
                let pts = i as f64 / fps_f;
                collector
                    .add_frame_png_file(i, path, pts)
                    .map_err(|e| format!("add frame {i}: {e}"))?;
            }
            Ok(())
        });

        let mut adapter = GifskiAdapter { reporter, seen: 0 };
        let write_result = writer
            .write(file, &mut adapter)
            .map_err(|e| format!("write gif: {e}"));

        let collect_result = collector_handle
            .join()
            .map_err(|_| "collector thread panicked".to_string())?;

        write_result?;
        collect_result
    });

    // Belt-and-braces: gifski's `done()` already clears, but the error path
    // may not invoke it.
    reporter.finish_phase();
    result
}

/// Adapts a `&mut dyn ProgressReporter` to gifski's own `ProgressReporter`
/// trait. gifski has no init hook, so callers must `start_phase` before
/// constructing this and `finish_phase` after writing.
struct GifskiAdapter<'a> {
    reporter: &'a mut dyn ProgressReporter,
    seen: u64,
}

impl gifski::progress::ProgressReporter for GifskiAdapter<'_> {
    fn increase(&mut self) -> bool {
        self.seen += 1;
        self.reporter.set_position(self.seen);
        true
    }

    fn done(&mut self, _msg: &str) {}
}

fn encode_webp_with_webp_animation(
    pngs: Vec<PathBuf>,
    output: &Path,
    fps: u32,
    quality: u8,
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    use webp_animation::prelude::*;
    use webp_animation::AnimParams;

    // Decode the first frame to learn dimensions; webp-animation needs them up front.
    let (width, height, first_rgba) = decode_png_rgba(&pngs[0])?;

    let encoding_config = EncodingConfig {
        encoding_type: EncodingType::Lossy(LossyEncodingConfig::default()),
        quality: quality as f32,
        method: 4, // 0 (fast) – 6 (slower-better). 4 matches libwebp's default.
    };
    let options = EncoderOptions {
        anim_params: AnimParams { loop_count: 0 }, // 0 = infinite
        encoding_config: Some(encoding_config),
        ..Default::default()
    };

    let mut encoder = Encoder::new_with_options((width, height), options)
        .map_err(|e| format!("webp encoder init: {e:?}"))?;

    let total_frames = pngs.len() as u64;
    reporter.start_phase("encoding webp", Some(total_frames));

    let frame_ms = |i: usize| -> i32 { (i as i64 * 1000 / fps as i64) as i32 };

    encoder
        .add_frame(&first_rgba, frame_ms(0))
        .map_err(|e| format!("add frame 0: {e:?}"))?;
    reporter.set_position(1);

    for (i, path) in pngs.iter().enumerate().skip(1) {
        let (w, h, rgba) = decode_png_rgba(path)?;
        if (w, h) != (width, height) {
            return Err(format!(
                "frame {i} size {w}x{h} differs from first frame {width}x{height}"
            ));
        }
        encoder
            .add_frame(&rgba, frame_ms(i))
            .map_err(|e| format!("add frame {i}: {e:?}"))?;
        reporter.set_position(i as u64 + 1);
    }

    let final_ts = frame_ms(pngs.len());
    let data = encoder
        .finalize(final_ts)
        .map_err(|e| format!("finalize webp: {e:?}"))?;
    reporter.finish_phase();

    std::fs::write(output, &*data).map_err(|e| format!("write output {}: {e}", output.display()))
}

/// Decode a PNG file to RGBA8 bytes, returning (width, height, rgba). The
/// `EXPAND` transformation widens RGB-only PNGs (which is what ffmpeg's
/// image2 muxer emits for our pipeline) to RGBA8 — webp-animation expects
/// 4 bytes per pixel.
fn decode_png_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("png info {}: {e}", path.display()))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png decode {}: {e}", path.display()))?;
    buf.truncate(info.buffer_size());

    // After EXPAND + STRIP_16, color is one of: RGB(24), RGBA(32), GA(16),
    // Grayscale(8). The screen-recording → ffmpeg pipeline produces RGB24.
    // Pad to RGBA by appending an opaque alpha byte per pixel.
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((info.width * info.height * 4) as usize);
            for chunk in buf.chunks_exact(3) {
                out.extend_from_slice(chunk);
                out.push(0xff);
            }
            out
        }
        other => {
            return Err(format!(
                "{}: unsupported PNG color type {other:?} (expected RGB or RGBA)",
                path.display()
            ));
        }
    };

    Ok((info.width, info.height, rgba))
}
