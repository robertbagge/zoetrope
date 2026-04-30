use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::settings::{Format, Platform, Playback, Quality, SUPPORTED_INPUT_FORMATS};

/// Validated, ready-to-run view of the inputs for a single encode. Every
/// field has been parsed, normalized, and range-checked; `pipeline::run`
/// trusts it.
pub struct Options {
    pub input: PathBuf,
    pub output: PathBuf,
    pub format: Format,
    /// Encoder quality knob (0-100). For GIF this is gifski_quality;
    /// for WebP it's libwebp's quality. Already resolved from --quality,
    /// platform preset, and defaults.
    pub encoder_quality: u8,
    pub fps: u32,
    pub width: u32,
    pub speed: Option<f64>,
    pub playback: Playback,
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub max_size: Option<u64>,
}

pub struct BatchPlan {
    pub options: Vec<Options>,
}

/// Pre-parsed, typed inputs to the validator. Strings have already been
/// converted (start/end/duration → seconds, max_size → bytes). The CLI's
/// clap layer does the string parsing via `crate::parse::*`; a desktop app
/// populates this from form state directly.
pub struct BatchInputs {
    pub inputs: Vec<PathBuf>,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub format: Option<Format>,
    pub quality: Option<Quality>,
    pub fps: Option<u32>,
    pub width: Option<u32>,
    pub speed: Option<f64>,
    pub playback: Playback,
    pub platform: Option<Platform>,
    pub start_secs: Option<f64>,
    pub end_secs: Option<f64>,
    pub duration_secs: Option<f64>,
    pub max_size_bytes: Option<u64>,
    pub force: bool,
}

impl BatchPlan {
    pub fn build(inputs: BatchInputs) -> Result<Self, String> {
        let n = inputs.inputs.len();

        if inputs.output.is_some() && n > 1 {
            return Err(format!(
                "-o/--output expects a single output path; got {n} inputs. \
                 Pass --output-dir for batch mode, or drop -o to write next to each input."
            ));
        }

        if let Some(speed) = inputs.speed {
            if !(speed.is_finite() && speed > 0.0) {
                return Err(format!("--speed must be a positive number, got {speed}"));
            }
        }

        let trim_duration = match (inputs.start_secs, inputs.end_secs, inputs.duration_secs) {
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
                return Err("--end and --duration are mutually exclusive".into())
            }
        };

        let inferred_from_output = inputs.output.as_deref().and_then(format_from_path);
        let format = match (inputs.format.as_ref(), inferred_from_output) {
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

        // Platform presets lock to GIF. Reject explicit or inferred WebP
        // rather than silently ignoring it.
        if let (Some(platform), Format::Webp) = (inputs.platform.as_ref(), &format) {
            return Err(format!(
                "--for {} produces GIF; drop --format webp (or the .webp output path)",
                platform.name(),
            ));
        }

        let platform_defaults = inputs.platform.as_ref().map(|p| p.settings());

        // Precedence (low → high): quality-medium default, platform, user -q, explicit flag.
        let user_set_quality = inputs.quality.is_some();
        let quality_preset = inputs.quality.unwrap_or(Quality::Medium);
        let quality_settings = quality_preset.settings();

        let width = inputs.width.unwrap_or(preset_pick(
            user_set_quality,
            quality_settings.width,
            platform_defaults.as_ref().map(|p| p.width),
        ));
        let fps = inputs.fps.unwrap_or(preset_pick(
            user_set_quality,
            quality_settings.fps,
            platform_defaults.as_ref().map(|p| p.fps),
        ));

        let max_size = inputs
            .max_size_bytes
            .or_else(|| platform_defaults.as_ref().map(|p| p.max_size));

        // --for locks format to GIF, so `gifski_quality` is the right knob when
        // the platform default wins.
        let format_quality = match format {
            Format::Gif => quality_settings.gifski_quality,
            Format::Webp => quality_settings.webp_quality,
        };
        let encoder_quality = preset_pick(
            user_set_quality,
            format_quality,
            platform_defaults.as_ref().map(|p| p.gifski_quality),
        );

        if let Some(dir) = inputs.output_dir.as_ref() {
            if dir.exists() && !dir.is_dir() {
                return Err(format!(
                    "--output-dir exists but is not a directory: {}",
                    dir.display()
                ));
            }
            if !dir.exists() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| format!("create --output-dir {}: {e}", dir.display()))?;
            }
        }

        let mut options = Vec::with_capacity(n);
        // Tracks which input first claimed each output path, so we can reject
        // batches where two inputs resolve to the same output (e.g. same stem
        // across directories into --output-dir, or the same file passed twice).
        // Left undetected, the second encode would silently overwrite the first.
        let mut claimed: HashMap<PathBuf, PathBuf> = HashMap::new();
        for input in &inputs.inputs {
            if !input.exists() {
                return Err(format!("file not found: {}", input.display()));
            }

            let ext = input
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase());
            if !ext
                .as_deref()
                .is_some_and(|e| SUPPORTED_INPUT_FORMATS.contains(&e))
            {
                return Err(format!(
                    "{}: input must be one of: {}",
                    input.display(),
                    SUPPORTED_INPUT_FORMATS.join(", ")
                ));
            }

            let output = resolve_output_path(
                input,
                inputs.output.as_deref(),
                inputs.output_dir.as_deref(),
                &format,
            );

            if let Some(prior) = claimed.get(&output) {
                return Err(format!(
                    "two inputs map to the same output {}: {} and {}. \
                     Drop the duplicate, or rename one.",
                    output.display(),
                    prior.display(),
                    input.display(),
                ));
            }

            if output.exists() && !inputs.force {
                return Err(format!(
                    "output file already exists: {} (use --force to overwrite)",
                    output.display()
                ));
            }

            claimed.insert(output.clone(), input.clone());
            options.push(Options {
                input: input.clone(),
                output,
                format: format.clone(),
                encoder_quality,
                fps,
                width,
                speed: inputs.speed,
                playback: inputs.playback.clone(),
                start: inputs.start_secs,
                duration: trim_duration,
                max_size,
            });
        }

        Ok(BatchPlan { options })
    }
}

fn resolve_output_path(
    input: &Path,
    explicit_output: Option<&Path>,
    output_dir: Option<&Path>,
    format: &Format,
) -> PathBuf {
    if let Some(o) = explicit_output {
        return o.to_path_buf();
    }
    let file_name = input
        .file_stem()
        .map(|s| PathBuf::from(s).with_extension(format.extension()))
        .unwrap_or_else(|| PathBuf::from(format!("out.{}", format.extension())));
    match output_dir {
        Some(dir) => dir.join(file_name),
        None => input.with_extension(format.extension()),
    }
}

/// Pick a preset value under the `quality preset (if user set -q) else platform
/// else quality default` precedence rule shared by width, fps, and encoder quality.
fn preset_pick<T: Copy>(user_set_quality: bool, from_quality: T, platform: Option<T>) -> T {
    if user_set_quality {
        from_quality
    } else {
        platform.unwrap_or(from_quality)
    }
}

fn format_from_path(path: &Path) -> Option<Format> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "gif" => Some(Format::Gif),
        "webp" => Some(Format::Webp),
        _ => None,
    }
}
