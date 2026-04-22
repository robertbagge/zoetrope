use clap::{Parser, ValueEnum};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use tempfile::Builder;

#[derive(Clone, ValueEnum)]
enum Quality {
    Low,
    Medium,
    High,
}

impl Quality {
    fn width(&self) -> u32 {
        match self {
            Quality::Low => 480,
            Quality::Medium => 960,
            Quality::High => 1440,
        }
    }

    fn fps(&self) -> u32 {
        match self {
            Quality::Low => 8,
            Quality::Medium => 12,
            Quality::High => 15,
        }
    }
}

#[derive(Parser)]
#[command(name = "zoetrope")]
#[command(version)]
#[command(about = "Convert .mov files to high-quality gifs using ffmpeg")]
struct Args {
    /// Input .mov file
    input: PathBuf,

    /// Output .gif path (defaults to input with .gif extension)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Quality preset: low (480p/8fps), medium (960p/12fps), high (1440p/15fps)
    #[arg(short, long, default_value = "medium")]
    quality: Quality,

    /// Frame rate (overrides quality preset)
    #[arg(long)]
    fps: Option<u32>,

    /// Output width in pixels (overrides quality preset)
    #[arg(long)]
    width: Option<u32>,

    /// Overwrite output file without prompting
    #[arg(short, long)]
    force: bool,
}

fn main() {
    let args = Args::parse();

    if !args.input.exists() {
        eprintln!("error: file not found: {}", args.input.display());
        process::exit(1);
    }

    if args.input.extension().and_then(|e| e.to_str()) != Some("mov") {
        eprintln!("error: input must be a .mov file");
        process::exit(1);
    }

    check_ffmpeg();

    let fps = args.fps.unwrap_or_else(|| args.quality.fps());
    let width = args.width.unwrap_or_else(|| args.quality.width());

    let output = args
        .output
        .unwrap_or_else(|| args.input.with_extension("gif"));

    if output.exists() && !args.force {
        eprintln!(
            "error: output file already exists: {} (use --force to overwrite)",
            output.display()
        );
        process::exit(1);
    }

    let palette = Builder::new()
        .suffix(".png")
        .tempfile()
        .expect("failed to create temp file");
    let palette_path = palette.path();

    eprintln!("generating palette... ({width}px, {fps}fps)");
    run_ffmpeg_pass1(&args.input, palette_path, fps, width);

    eprintln!("encoding gif...");
    run_ffmpeg_pass2(&args.input, palette_path, &output, fps, width);

    drop(palette);

    let size = std::fs::metadata(&output)
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!("done: {} ({:.1} MB)", output.display(), size as f64 / 1_048_576.0);
}

fn check_ffmpeg() {
    match Command::new("ffmpeg").arg("-version").output() {
        Ok(output) if output.status.success() => {}
        Ok(_) => {
            eprintln!("error: ffmpeg found but returned an error");
            process::exit(1);
        }
        Err(_) => {
            eprintln!("error: ffmpeg not found — install it with `brew install ffmpeg`");
            process::exit(1);
        }
    }
}

fn run_ffmpeg_pass1(input: &Path, palette: &Path, fps: u32, width: u32) {
    let filter = format!("fps={fps},scale={width}:-1:flags=lanczos,palettegen=stats_mode=diff");

    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input)
        .args(["-vf", &filter])
        .arg(palette)
        .output()
        .expect("failed to run ffmpeg");

    if !status.status.success() {
        eprintln!("ffmpeg palette generation failed:");
        eprintln!("{}", String::from_utf8_lossy(&status.stderr));
        process::exit(1);
    }
}

fn run_ffmpeg_pass2(input: &Path, palette: &Path, output: &Path, fps: u32, width: u32) {
    let filter = format!(
        "fps={fps},scale={width}:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5"
    );

    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input)
        .args(["-i"])
        .arg(palette)
        .args(["-lavfi", &filter])
        .arg(output)
        .output()
        .expect("failed to run ffmpeg");

    if !status.status.success() {
        eprintln!("ffmpeg gif encoding failed:");
        eprintln!("{}", String::from_utf8_lossy(&status.stderr));
        process::exit(1);
    }
}
