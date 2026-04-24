use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use tempfile::TempDir;

use crate::{Format, Playback, Quality};

pub struct Options {
    pub input: PathBuf,
    pub output: PathBuf,
    pub format: Format,
    pub quality: Quality,
    pub fps: u32,
    pub width: u32,
    pub speed: Option<f64>,
    pub playback: Playback,
    pub start: Option<f64>,
    pub duration: Option<f64>,
}

pub fn check_ffmpeg() {
    match Command::new("ffmpeg").arg("-version").output() {
        Ok(output) if output.status.success() => {}
        Ok(_) => {
            eprintln!("error: ffmpeg found but returned an error");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("error: ffmpeg not found — install it with `brew install ffmpeg`");
            std::process::exit(1);
        }
    }
}

pub fn run(opts: &Options) -> Result<(), String> {
    match opts.format {
        Format::Gif => run_gif(opts),
        Format::Webp => run_webp(opts),
    }
}

fn run_gif(opts: &Options) -> Result<(), String> {
    let tmp = TempDir::new().map_err(|e| format!("create tempdir: {e}"))?;
    let png_pattern = tmp.path().join("frame_%06d.png");

    eprintln!("extracting frames... ({}px, {}fps)", opts.width, opts.fps);
    extract_png_frames(opts, &png_pattern)?;

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
    encode_gif_with_gifski(
        pngs,
        &opts.output,
        opts.width,
        opts.fps,
        opts.quality.gifski_quality(),
    )?;

    report_done(&opts.output);
    Ok(())
}

fn run_webp(opts: &Options) -> Result<(), String> {
    if !ffmpeg_has_encoder("libwebp") {
        return Err(
            "ffmpeg was built without libwebp — install one that includes it \
             (e.g. `brew install ffmpeg-full` on macOS, standard `ffmpeg` on Ubuntu)"
                .into(),
        );
    }

    eprintln!("encoding webp... ({}px, {}fps)", opts.width, opts.fps);
    encode_webp(opts)?;
    report_done(&opts.output);
    Ok(())
}

fn ffmpeg_has_encoder(name: &str) -> bool {
    let out = match Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().any(|line| {
        // Encoder lines look like: " V....D libwebp              libwebp WebP image"
        let trimmed = line.trim_start();
        let mut parts = trimmed.splitn(3, char::is_whitespace);
        let _flags = parts.next();
        parts.next() == Some(name)
    })
}

fn report_done(output: &Path) {
    let size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "done: {} ({:.1} MB)",
        output.display(),
        size as f64 / 1_048_576.0
    );
}

fn extract_png_frames(opts: &Options, pattern: &Path) -> Result<(), String> {
    let filter = build_filter_complex(opts.fps, opts.width, opts.speed, &opts.playback);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y");
    apply_trim(&mut cmd, opts);
    cmd.arg("-i").arg(&opts.input);
    cmd.args(["-filter_complex", &filter]);
    cmd.args(["-map", "[out]"]);
    cmd.args(["-start_number", "0"]);
    cmd.arg(pattern);

    run_ffmpeg(cmd, "frame extraction")
}

fn encode_webp(opts: &Options) -> Result<(), String> {
    let filter = build_filter_complex(opts.fps, opts.width, opts.speed, &opts.playback);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y");
    apply_trim(&mut cmd, opts);
    cmd.arg("-i").arg(&opts.input);
    cmd.args(["-filter_complex", &filter]);
    cmd.args(["-map", "[out]"]);
    cmd.args(["-c:v", "libwebp"]);
    cmd.args(["-loop", "0"]);
    cmd.args(["-quality", &opts.quality.webp_quality().to_string()]);
    cmd.arg(&opts.output);

    run_ffmpeg(cmd, "webp encoding")
}

fn apply_trim(cmd: &mut Command, opts: &Options) {
    if let Some(start) = opts.start {
        cmd.args(["-ss", &format!("{start}")]);
    }
    if let Some(duration) = opts.duration {
        cmd.args(["-t", &format!("{duration}")]);
    }
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
    let collector_handle = thread::spawn(move || -> Result<(), String> {
        for (i, path) in pngs.into_iter().enumerate() {
            let pts = i as f64 / fps_f;
            collector
                .add_frame_png_file(i, path, pts)
                .map_err(|e| format!("add frame {i}: {e}"))?;
        }
        Ok(())
    });

    writer
        .write(file, &mut NoProgress {})
        .map_err(|e| format!("write gif: {e}"))?;

    collector_handle
        .join()
        .map_err(|_| "collector thread panicked".to_string())??;

    Ok(())
}
