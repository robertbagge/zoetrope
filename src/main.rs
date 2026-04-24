use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::process;

mod pipeline;

const SUPPORTED_INPUT_FORMATS: &[&str] = &["mov", "mp4", "webm", "mkv", "avi"];

#[derive(Clone, ValueEnum)]
pub enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

impl Quality {
    pub fn width(&self) -> u32 {
        match self {
            Quality::Low => 480,
            Quality::Medium => 960,
            Quality::High => 1440,
            Quality::Ultra => 2048,
        }
    }

    pub fn fps(&self) -> u32 {
        match self {
            Quality::Low => 8,
            Quality::Medium => 12,
            Quality::High => 15,
            Quality::Ultra => 24,
        }
    }

    pub fn gifski_quality(&self) -> u8 {
        match self {
            Quality::Low => 60,
            Quality::Medium => 80,
            Quality::High => 95,
            Quality::Ultra => 100,
        }
    }

    pub fn webp_quality(&self) -> u8 {
        match self {
            Quality::Low => 60,
            Quality::Medium => 80,
            Quality::High => 90,
            Quality::Ultra => 95,
        }
    }
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
pub enum Format {
    Gif,
    Webp,
}

impl Format {
    fn extension(&self) -> &'static str {
        match self {
            Format::Gif => "gif",
            Format::Webp => "webp",
        }
    }
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
pub enum Playback {
    Normal,
    Reverse,
    Boomerang,
}

#[derive(Parser)]
#[command(name = "zoetrope")]
#[command(version)]
#[command(about = "Convert screen recordings to high-quality GIFs or WebP")]
struct Args {
    /// Input video file (mov, mp4, webm, mkv, avi)
    input: PathBuf,

    /// Output file path (defaults to input with the chosen format's extension)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short = 'F', long, default_value = "gif")]
    format: Format,

    /// Quality preset: low (480p/8fps), medium (960p/12fps), high (1440p/15fps), ultra (2K/24fps)
    #[arg(short, long, default_value = "medium")]
    quality: Quality,

    /// Frame rate (overrides quality preset)
    #[arg(long)]
    fps: Option<u32>,

    /// Output width in pixels (overrides quality preset)
    #[arg(long)]
    width: Option<u32>,

    /// Playback speed multiplier (e.g. 2, 0.5)
    #[arg(long)]
    speed: Option<f64>,

    /// Playback mode
    #[arg(long, default_value = "normal")]
    playback: Playback,

    /// Start time (e.g. 5s, 1:30, 1:30:45)
    #[arg(long)]
    start: Option<String>,

    /// End time (mutually exclusive with --duration)
    #[arg(long, conflicts_with = "duration")]
    end: Option<String>,

    /// Duration from start (mutually exclusive with --end)
    #[arg(long, conflicts_with = "end")]
    duration: Option<String>,

    /// Overwrite output file without prompting
    #[arg(short, long)]
    force: bool,
}

fn main() {
    let args = Args::parse();

    if !args.input.exists() {
        fail(format!("file not found: {}", args.input.display()));
    }

    let ext = args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        Some(e) if SUPPORTED_INPUT_FORMATS.contains(&e) => {}
        _ => fail(format!(
            "input must be one of: {}",
            SUPPORTED_INPUT_FORMATS.join(", ")
        )),
    }

    if let Some(speed) = args.speed {
        if !(speed.is_finite() && speed > 0.0) {
            fail(format!("--speed must be a positive number, got {speed}"));
        }
    }

    let start = args.start.as_deref().map(parse_time_or_exit);
    let end = args.end.as_deref().map(parse_time_or_exit);
    let duration = args.duration.as_deref().map(parse_time_or_exit);

    let trim_duration = match (start, end, duration) {
        (_, Some(_), Some(_)) => {
            fail("--end and --duration are mutually exclusive".into());
        }
        (s, Some(e), None) => {
            let start_val = s.unwrap_or(0.0);
            if e <= start_val {
                fail(format!(
                    "--end ({e}) must be greater than --start ({start_val})"
                ));
            }
            Some(e - start_val)
        }
        (_, None, Some(d)) => {
            if d <= 0.0 {
                fail(format!("--duration must be positive, got {d}"));
            }
            Some(d)
        }
        (_, None, None) => None,
    };

    pipeline::check_ffmpeg();

    let fps = args.fps.unwrap_or_else(|| args.quality.fps());
    let width = args.width.unwrap_or_else(|| args.quality.width());

    let output = args
        .output
        .unwrap_or_else(|| args.input.with_extension(args.format.extension()));

    if output.exists() && !args.force {
        fail(format!(
            "output file already exists: {} (use --force to overwrite)",
            output.display()
        ));
    }

    let opts = pipeline::Options {
        input: args.input,
        output,
        format: args.format,
        quality: args.quality,
        fps,
        width,
        speed: args.speed,
        playback: args.playback,
        start,
        duration: trim_duration,
    };

    if let Err(e) = pipeline::run(&opts) {
        fail(e);
    }
}

fn parse_time_or_exit(s: &str) -> f64 {
    match parse_time(s) {
        Ok(v) => v,
        Err(e) => fail(format!("invalid time \"{s}\": {e}")),
    }
}

fn parse_time(s: &str) -> Result<f64, String> {
    let trimmed = s.trim();
    let without_suffix = trimmed.strip_suffix('s').unwrap_or(trimmed);

    if without_suffix.is_empty() {
        return Err("empty".into());
    }

    let parts: Vec<&str> = without_suffix.split(':').collect();
    let parse = |x: &str| -> Result<f64, String> { x.parse::<f64>().map_err(|e| e.to_string()) };

    let seconds = match parts.as_slice() {
        [s] => parse(s)?,
        [m, s] => parse(m)? * 60.0 + parse(s)?,
        [h, m, s] => parse(h)? * 3600.0 + parse(m)? * 60.0 + parse(s)?,
        _ => return Err("expected SS, MM:SS, or HH:MM:SS".into()),
    };

    if !seconds.is_finite() || seconds < 0.0 {
        return Err("must be non-negative".into());
    }
    Ok(seconds)
}

fn fail(msg: String) -> ! {
    eprintln!("error: {msg}");
    process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::parse_time;

    #[test]
    fn parse_time_seconds_plain() {
        assert_eq!(parse_time("5").unwrap(), 5.0);
        assert_eq!(parse_time("5s").unwrap(), 5.0);
        assert_eq!(parse_time("0.5").unwrap(), 0.5);
        assert_eq!(parse_time("1.25s").unwrap(), 1.25);
    }

    #[test]
    fn parse_time_mm_ss() {
        assert_eq!(parse_time("1:30").unwrap(), 90.0);
        assert_eq!(parse_time("0:05").unwrap(), 5.0);
    }

    #[test]
    fn parse_time_hh_mm_ss() {
        assert_eq!(parse_time("1:00:00").unwrap(), 3600.0);
        assert_eq!(parse_time("0:01:30").unwrap(), 90.0);
    }

    #[test]
    fn parse_time_rejects_garbage() {
        assert!(parse_time("abc").is_err());
        assert!(parse_time("").is_err());
        assert!(parse_time("1:2:3:4").is_err());
    }
}
