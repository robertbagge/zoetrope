use crate::encode::{encode_gif, encode_webp, EncodeParams};
use crate::ffmpeg::probe_duration;
use crate::fit;
use crate::options::Options;
use crate::progress::ProgressReporter;
use crate::settings::Format;
use crate::still::encode_still_image;

pub fn run(opts: &Options, reporter: &mut dyn ProgressReporter) -> Result<(), String> {
    // Still-image input bypasses the video pipeline entirely: no ffmpeg probe,
    // no fit loop, no animation encoder. Options validation has already
    // rejected --max-size and the temporal flags for this branch.
    if opts.is_still_input() {
        encode_still_image(opts, reporter)?;
        report_done(opts, reporter);
        return Ok(());
    }

    // Probe once and cache — the fit loop may call encode up to 5 times,
    // but the input duration doesn't change between attempts.
    let probe_seconds = probe_duration(&opts.input);
    let params = initial_params(opts);
    match opts.max_size {
        Some(target) => fit::fit_to_size(opts, params, target, probe_seconds, reporter)?,
        None => encode(opts, &params, probe_seconds, reporter)?,
    }
    report_done(opts, reporter);
    Ok(())
}

pub fn encode(
    opts: &Options,
    params: &EncodeParams,
    probe_seconds: Option<f64>,
    reporter: &mut dyn ProgressReporter,
) -> Result<(), String> {
    match opts.format {
        Format::Gif => encode_gif(opts, params, probe_seconds, reporter),
        Format::Webp => encode_webp(opts, params, probe_seconds, reporter),
        Format::Png | Format::Jpeg => {
            // Reached only if the dispatcher above failed to short-circuit.
            // Options validation guarantees a still input for these formats.
            unreachable!("still-only formats must route through encode_still_image")
        }
    }
}

pub fn initial_params(opts: &Options) -> EncodeParams {
    EncodeParams {
        width: opts.width,
        height: opts.height,
        fps: opts.fps,
        quality: opts.encoder_quality,
    }
}

fn report_done(opts: &Options, reporter: &mut dyn ProgressReporter) {
    let size = std::fs::metadata(&opts.output)
        .map(|m| m.len())
        .unwrap_or(0);
    reporter.status(&format!(
        "done: {} ({:.1} MB)",
        opts.output.display(),
        size as f64 / 1_048_576.0
    ));
}
