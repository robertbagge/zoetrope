use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use zoetrope_core::parse::{parse_size, parse_time};
use zoetrope_core::{BatchInputs, BatchPlan, Format, Platform, Playback, Quality};

const EXAMPLES: &str = "\
Examples:
  # Basics
  zoetrope demo.mov                            # → demo.gif (medium quality)
  zoetrope demo.mov -o clip.gif                # custom output filename
  zoetrope demo.mov -F webp                    # → demo.webp (2-5x smaller)
  zoetrope demo.mov --force                    # overwrite existing output

  # Quality and size
  zoetrope demo.mov -q high                    # 1440px, 15fps preset
  zoetrope demo.mov --width 640                # override width only
  zoetrope demo.mov --fps 20                   # override frame rate only
  zoetrope demo.mov --max-size 500kb           # shrink iteratively to fit
  zoetrope demo.mov --for slack                # platform preset with auto-fit
  zoetrope demo.mov --for slack --fps 15       # preset + manual override

  # Trim, speed, playback
  zoetrope demo.mov --start 5s --end 12s       # 7-second clip
  zoetrope demo.mov --start 1:30 --duration 10s
  zoetrope demo.mov --end 10s                  # first 10 seconds
  zoetrope demo.mov --speed 2                  # 2x speedup
  zoetrope demo.mov --speed 0.5                # slow motion
  zoetrope demo.mov --playback reverse
  zoetrope demo.mov --playback boomerang       # forward then reverse

  # Batch
  zoetrope *.mov                               # each → .gif next to input
  zoetrope a.mov b.mp4 c.webm                  # mixed formats
  zoetrope *.mov --output-dir ./gifs/          # collect outputs in one dir
  zoetrope *.mov --for slack --output-dir ./slack/
";

#[derive(Clone, ValueEnum)]
pub enum CliQuality {
    /// 480px, 8fps — small files for chat
    Low,
    /// 960px, 12fps — default; good for GitHub PRs and docs
    Medium,
    /// 1440px, 15fps — presentations and LinkedIn
    High,
    /// 2048px, 24fps — demo reels, high-fidelity
    Ultra,
}

impl CliQuality {
    fn into_core(self) -> Quality {
        match self {
            CliQuality::Low => Quality::Low,
            CliQuality::Medium => Quality::Medium,
            CliQuality::High => Quality::High,
            CliQuality::Ultra => Quality::Ultra,
        }
    }
}

#[derive(Clone, ValueEnum)]
pub enum CliFormat {
    /// GIF — universal compatibility, larger files
    Gif,
    /// Animated WebP — 2-5x smaller files, 97% browser support
    Webp,
}

impl CliFormat {
    fn into_core(self) -> Format {
        match self {
            CliFormat::Gif => Format::Gif,
            CliFormat::Webp => Format::Webp,
        }
    }
}

#[derive(Clone, ValueEnum)]
pub enum CliPlayback {
    /// Standard forward playback
    Normal,
    /// Play the video backwards
    Reverse,
    /// Forward then reverse (ping-pong loop) — doubles frame count
    Boomerang,
}

impl CliPlayback {
    fn into_core(self) -> Playback {
        match self {
            CliPlayback::Normal => Playback::Normal,
            CliPlayback::Reverse => Playback::Reverse,
            CliPlayback::Boomerang => Playback::Boomerang,
        }
    }
}

#[derive(Clone, ValueEnum)]
pub enum CliPlatform {
    /// ≤5 MB, 480px, 10fps — tight for chat
    Slack,
    /// ≤10 MB, 960px, 12fps — PR/issue attachments
    Github,
    /// ≤8 MB, 640px, 12fps
    Discord,
    /// ≤5 MB, 480px, 10fps
    Twitter,
    /// ≤500 KB, 320px, 8fps — inline-friendly
    Email,
}

impl CliPlatform {
    fn into_core(self) -> Platform {
        match self {
            CliPlatform::Slack => Platform::Slack,
            CliPlatform::Github => Platform::Github,
            CliPlatform::Discord => Platform::Discord,
            CliPlatform::Twitter => Platform::Twitter,
            CliPlatform::Email => Platform::Email,
        }
    }
}

#[derive(Parser)]
#[command(name = "zoetrope")]
#[command(version)]
#[command(about = "Convert screen recordings to high-quality GIFs or WebP")]
#[command(after_help = EXAMPLES)]
pub struct Args {
    /// Input video file(s) (mov, mp4, webm, mkv, avi). Pass multiple for batch mode.
    #[arg(required = true, num_args = 1..)]
    pub inputs: Vec<PathBuf>,

    /// Output file path (single-input mode only; incompatible with --output-dir)
    #[arg(short, long, conflicts_with = "output_dir")]
    pub output: Option<PathBuf>,

    /// Output directory for batch mode (created if missing)
    #[arg(long)]
    pub output_dir: Option<PathBuf>,

    /// Output format (defaults to gif, or inferred from --output extension)
    #[arg(short = 'F', long)]
    pub format: Option<CliFormat>,

    /// Quality preset [default: medium]
    #[arg(short, long)]
    pub quality: Option<CliQuality>,

    /// Frame rate (overrides quality preset)
    #[arg(long)]
    pub fps: Option<u32>,

    /// Output width in pixels (overrides quality preset)
    #[arg(long)]
    pub width: Option<u32>,

    /// Playback speed multiplier (e.g. 2, 0.5)
    #[arg(long)]
    pub speed: Option<f64>,

    /// Playback mode
    #[arg(long, default_value = "normal")]
    pub playback: CliPlayback,

    /// Platform preset (slack, github, discord, twitter, email). Locks format=gif
    /// and enforces a size limit; explicit --fps/--width/--max-size override.
    #[arg(long = "for")]
    pub for_: Option<CliPlatform>,

    /// Start time (e.g. 5s, 1:30, 1:30:45)
    #[arg(long)]
    pub start: Option<String>,

    /// End time (mutually exclusive with --duration)
    #[arg(long, conflicts_with = "duration")]
    pub end: Option<String>,

    /// Duration from start (mutually exclusive with --end)
    #[arg(long, conflicts_with = "end")]
    pub duration: Option<String>,

    /// Target max file size (e.g. 5mb, 500kb). Sizes use decimal units (1mb = 1,000,000 bytes).
    #[arg(long)]
    pub max_size: Option<String>,

    /// Overwrite output file without prompting
    #[arg(short, long)]
    pub force: bool,
}

impl Args {
    pub fn into_batch(self) -> Result<BatchPlan, String> {
        let start_secs = self.start.as_deref().map(parse_time_arg).transpose()?;
        let end_secs = self.end.as_deref().map(parse_time_arg).transpose()?;
        let duration_secs = self.duration.as_deref().map(parse_time_arg).transpose()?;
        let max_size_bytes = self.max_size.as_deref().map(parse_size_arg).transpose()?;

        let inputs = BatchInputs {
            inputs: self.inputs,
            output: self.output,
            output_dir: self.output_dir,
            format: self.format.map(CliFormat::into_core),
            quality: self.quality.map(CliQuality::into_core),
            fps: self.fps,
            width: self.width,
            speed: self.speed,
            playback: self.playback.into_core(),
            platform: self.for_.map(CliPlatform::into_core),
            start_secs,
            end_secs,
            duration_secs,
            max_size_bytes,
            force: self.force,
        };

        BatchPlan::build(inputs)
    }
}

fn parse_time_arg(s: &str) -> Result<f64, String> {
    parse_time(s).map_err(|e| format!("invalid time \"{s}\": {e}"))
}

fn parse_size_arg(s: &str) -> Result<u64, String> {
    parse_size(s).map_err(|e| format!("invalid size \"{s}\": {e}"))
}
