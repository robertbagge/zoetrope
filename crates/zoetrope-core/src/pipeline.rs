use crate::encode::{encode_gif, encode_webp, EncodeParams};
use crate::ffmpeg::probe_duration;
use crate::fit;
use crate::options::Options;
use crate::progress::ProgressReporter;
use crate::settings::Format;

pub fn run(opts: &Options, reporter: &mut dyn ProgressReporter) -> Result<(), String> {
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
    }
}

pub fn initial_params(opts: &Options) -> EncodeParams {
    EncodeParams {
        width: opts.width,
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
