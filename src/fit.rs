use crate::cli::{Format, Options};
use crate::encode::EncodeParams;
use crate::pipeline;

const MAX_ATTEMPTS: u32 = 5;
const MIN_WIDTH: u32 = 240;
const MIN_FPS: u32 = 6;
const MIN_GIFSKI_QUALITY: u8 = 40;
const MIN_WEBP_QUALITY: u8 = 30;

/// Encode, measure, shrink, retry — until the output fits under `target`
/// or all knobs hit their floor. The output file is overwritten on each
/// attempt, so the last attempt's bytes are what the user sees.
pub(crate) fn fit_to_size(
    opts: &Options,
    start: EncodeParams,
    target: u64,
) -> Result<(), String> {
    let mut params = start;
    let mut smallest: Option<(u64, EncodeParams)> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        if attempt > 1 {
            eprintln!(
                "fit attempt {attempt}/{MAX_ATTEMPTS} ({}px, {}fps, q{})",
                params.width, params.fps, params.quality
            );
        }

        pipeline::encode(opts, &params)?;
        let size = std::fs::metadata(&opts.output)
            .map_err(|e| format!("stat output: {e}"))?
            .len();

        if size <= target {
            return Ok(());
        }

        let floor_hit = smallest
            .as_ref()
            .map(|(s, _)| size >= *s)
            .unwrap_or(false);
        if smallest.as_ref().map(|(s, _)| size < *s).unwrap_or(true) {
            smallest = Some((size, params.clone()));
        }

        match shrink_step(&opts.format, &params, size, target) {
            Some(next) => params = next,
            None => break,
        }

        // If shrink couldn't actually reduce anything (shouldn't happen, but guard).
        if floor_hit && attempt == MAX_ATTEMPTS {
            break;
        }
    }

    let (bytes, p) = smallest.expect("fit loop ran at least once");
    Err(format!(
        "could not reach {} after {MAX_ATTEMPTS} attempts (smallest: {} at {}px/{}fps/q{})",
        format_size(target),
        format_size(bytes),
        p.width,
        p.fps,
        p.quality,
    ))
}

/// Returns the next `EncodeParams` to try, or `None` when every knob has
/// already hit its floor. Order depends on format:
///   GIF  → width, then fps, then quality (width dominates file size)
///   WebP → quality, then width, then fps (quality is the dominant knob)
fn shrink_step(
    format: &Format,
    current: &EncodeParams,
    actual: u64,
    target: u64,
) -> Option<EncodeParams> {
    let min_quality = match format {
        Format::Gif => MIN_GIFSKI_QUALITY,
        Format::Webp => MIN_WEBP_QUALITY,
    };

    let width_at_floor = current.width <= MIN_WIDTH;
    let fps_at_floor = current.fps <= MIN_FPS;
    let quality_at_floor = current.quality <= min_quality;

    if width_at_floor && fps_at_floor && quality_at_floor {
        return None;
    }

    // Ratio targets the dominant knob (~sqrt because file size scales with area).
    let ratio = (target as f64 / actual as f64).sqrt().max(0.5);
    let mut next = current.clone();

    match format {
        Format::Gif => {
            if !width_at_floor {
                next.width = shrink_width(current.width, ratio);
            } else if !fps_at_floor {
                next.fps = shrink_fps(current.fps);
            } else {
                next.quality = shrink_quality(current.quality, min_quality);
            }
        }
        Format::Webp => {
            if !quality_at_floor {
                next.quality = shrink_quality(current.quality, min_quality);
            } else if !width_at_floor {
                next.width = shrink_width(current.width, ratio);
            } else {
                next.fps = shrink_fps(current.fps);
            }
        }
    }

    Some(next)
}

fn shrink_width(current: u32, ratio: f64) -> u32 {
    let scaled = (current as f64 * ratio) as u32;
    scaled.max(MIN_WIDTH).min(current.saturating_sub(1)).max(MIN_WIDTH)
}

fn shrink_fps(current: u32) -> u32 {
    // Step down ~25% per shrink, floor at MIN_FPS.
    ((current as f64 * 0.75) as u32).max(MIN_FPS).min(current.saturating_sub(1)).max(MIN_FPS)
}

fn shrink_quality(current: u8, floor: u8) -> u8 {
    current.saturating_sub(15).max(floor)
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.0} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}
