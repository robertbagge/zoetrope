use clap::{Parser, ValueEnum};
use std::path::PathBuf;

pub(crate) const SUPPORTED_INPUT_FORMATS: &[&str] = &["mov", "mp4", "webm", "mkv", "avi"];

pub(crate) struct QualitySettings {
    pub width: u32,
    pub fps: u32,
    pub gifski_quality: u8,
    pub webp_quality: u8,
}

#[derive(Clone, ValueEnum)]
pub(crate) enum Quality {
    /// 480px, 8fps — small files for chat
    Low,
    /// 960px, 12fps — default; good for GitHub PRs and docs
    Medium,
    /// 1440px, 15fps — presentations and LinkedIn
    High,
    /// 2048px, 24fps — demo reels, high-fidelity
    Ultra,
}

impl Quality {
    pub(crate) fn settings(&self) -> QualitySettings {
        match self {
            Quality::Low => QualitySettings {
                width: 480,
                fps: 8,
                gifski_quality: 60,
                webp_quality: 60,
            },
            Quality::Medium => QualitySettings {
                width: 960,
                fps: 12,
                gifski_quality: 80,
                webp_quality: 80,
            },
            Quality::High => QualitySettings {
                width: 1440,
                fps: 15,
                gifski_quality: 95,
                webp_quality: 90,
            },
            Quality::Ultra => QualitySettings {
                width: 2048,
                fps: 24,
                gifski_quality: 100,
                webp_quality: 95,
            },
        }
    }
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
pub(crate) enum Format {
    /// GIF — universal compatibility, larger files
    Gif,
    /// Animated WebP — 2-5x smaller files, 97% browser support
    Webp,
}

impl Format {
    pub(crate) fn extension(&self) -> &'static str {
        match self {
            Format::Gif => "gif",
            Format::Webp => "webp",
        }
    }
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
pub(crate) enum Playback {
    /// Standard forward playback
    Normal,
    /// Play the video backwards
    Reverse,
    /// Forward then reverse (ping-pong loop) — doubles frame count
    Boomerang,
}

#[derive(Parser)]
#[command(name = "zoetrope")]
#[command(version)]
#[command(about = "Convert screen recordings to high-quality GIFs or WebP")]
pub(crate) struct Args {
    /// Input video file (mov, mp4, webm, mkv, avi)
    pub input: PathBuf,

    /// Output file path (defaults to input with the chosen format's extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output format (defaults to gif, or inferred from --output extension)
    #[arg(short = 'F', long)]
    pub format: Option<Format>,

    /// Quality preset
    #[arg(short, long, default_value = "medium")]
    pub quality: Quality,

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
    pub playback: Playback,

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

/// Validated, ready-to-run view of the CLI arguments. Every field has been
/// parsed, normalized, and range-checked; `pipeline::run` trusts it.
pub(crate) struct Options {
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
    pub max_size: Option<u64>,
}

impl Args {
    pub(crate) fn into_options(self) -> Result<Options, String> {
        if !self.input.exists() {
            return Err(format!("file not found: {}", self.input.display()));
        }

        let ext = self
            .input
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        if !ext
            .as_deref()
            .is_some_and(|e| SUPPORTED_INPUT_FORMATS.contains(&e))
        {
            return Err(format!(
                "input must be one of: {}",
                SUPPORTED_INPUT_FORMATS.join(", ")
            ));
        }

        if let Some(speed) = self.speed {
            if !(speed.is_finite() && speed > 0.0) {
                return Err(format!("--speed must be a positive number, got {speed}"));
            }
        }

        let start = self.start.as_deref().map(parse_time_arg).transpose()?;
        let end = self.end.as_deref().map(parse_time_arg).transpose()?;
        let duration = self.duration.as_deref().map(parse_time_arg).transpose()?;

        // clap's `conflicts_with` on --end/--duration guarantees we never see both.
        let trim_duration = match (start, end, duration) {
            (s, Some(e), None) => {
                let start_val = s.unwrap_or(0.0);
                if e <= start_val {
                    return Err(format!(
                        "--end ({e}) must be greater than --start ({start_val})"
                    ));
                }
                Some(e - start_val)
            }
            (_, None, Some(d)) => {
                if d <= 0.0 {
                    return Err(format!("--duration must be positive, got {d}"));
                }
                Some(d)
            }
            (_, None, None) => None,
            (_, Some(_), Some(_)) => {
                unreachable!("clap `conflicts_with` rejects this combination")
            }
        };

        let inferred_from_output = self.output.as_deref().and_then(format_from_path);
        let format = match (self.format.as_ref(), inferred_from_output) {
            (Some(explicit), Some(inferred)) if *explicit != inferred => {
                return Err(format!(
                    "--format {} conflicts with --output extension .{} (pick one consistent format)",
                    explicit.extension(),
                    inferred.extension(),
                ));
            }
            (Some(f), _) => f.clone(),
            (None, Some(f)) => f,
            (None, None) => Format::Gif,
        };

        let quality_settings = self.quality.settings();
        let fps = self.fps.unwrap_or(quality_settings.fps);
        let width = self.width.unwrap_or(quality_settings.width);

        let max_size = self
            .max_size
            .as_deref()
            .map(parse_size_arg)
            .transpose()?;

        let output = self
            .output
            .unwrap_or_else(|| self.input.with_extension(format.extension()));

        if output.exists() && !self.force {
            return Err(format!(
                "output file already exists: {} (use --force to overwrite)",
                output.display()
            ));
        }

        Ok(Options {
            input: self.input,
            output,
            format,
            quality: self.quality,
            fps,
            width,
            speed: self.speed,
            playback: self.playback,
            start,
            duration: trim_duration,
            max_size,
        })
    }
}

fn format_from_path(path: &std::path::Path) -> Option<Format> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "gif" => Some(Format::Gif),
        "webp" => Some(Format::Webp),
        _ => None,
    }
}

pub(crate) fn parse_time(s: &str) -> Result<f64, String> {
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

fn parse_time_arg(s: &str) -> Result<f64, String> {
    parse_time(s).map_err(|e| format!("invalid time \"{s}\": {e}"))
}

/// Parse a human-readable size like `5mb`, `500kb`, `2GB`, or a raw byte count.
/// Units are decimal (1 kb = 1,000 bytes, 1 mb = 1,000,000 bytes) to match how
/// GitHub, Slack, and Discord document their upload limits.
pub(crate) fn parse_size(s: &str) -> Result<u64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty".into());
    }

    let (num_part, multiplier) = split_size_suffix(trimmed)?;
    if num_part.is_empty() {
        return Err("missing number".into());
    }

    let value: f64 = num_part.parse().map_err(|_| format!("not a number: {num_part}"))?;
    if !value.is_finite() || value <= 0.0 {
        return Err("must be positive".into());
    }

    Ok((value * multiplier as f64).round() as u64)
}

fn split_size_suffix(s: &str) -> Result<(&str, u64), String> {
    let lower_end = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &s[lower_end.len()..];
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1_000,
        "m" | "mb" => 1_000_000,
        "g" | "gb" => 1_000_000_000,
        other => return Err(format!("unknown size suffix \"{other}\" (expected b, kb, mb, gb)")),
    };
    Ok((lower_end.trim(), multiplier))
}

fn parse_size_arg(s: &str) -> Result<u64, String> {
    parse_size(s).map_err(|e| format!("invalid size \"{s}\": {e}"))
}

#[cfg(test)]
mod tests {
    use super::{parse_size, parse_time};

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

    #[test]
    fn parse_size_decimal_units() {
        assert_eq!(parse_size("5mb").unwrap(), 5_000_000);
        assert_eq!(parse_size("5MB").unwrap(), 5_000_000);
        assert_eq!(parse_size("5m").unwrap(), 5_000_000);
        assert_eq!(parse_size("500kb").unwrap(), 500_000);
        assert_eq!(parse_size("500k").unwrap(), 500_000);
        assert_eq!(parse_size("2gb").unwrap(), 2_000_000_000);
        assert_eq!(parse_size("1.5mb").unwrap(), 1_500_000);
    }

    #[test]
    fn parse_size_raw_bytes() {
        assert_eq!(parse_size("5000000").unwrap(), 5_000_000);
        assert_eq!(parse_size("1024b").unwrap(), 1024);
    }

    #[test]
    fn parse_size_rejects_garbage() {
        assert!(parse_size("").is_err());
        assert!(parse_size("5xb").is_err());
        assert!(parse_size("mb").is_err());
        assert!(parse_size("0").is_err());
        assert!(parse_size("-5mb").is_err());
    }
}
