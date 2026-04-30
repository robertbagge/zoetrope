use clap::Parser;
use std::process::ExitCode;

use zoetrope_core::{ffmpeg, pipeline};

mod args;
mod progress_term;

use progress_term::IndicatifReporter;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let batch = args::Args::parse().into_batch()?;
    ffmpeg::check_ffmpeg()?;
    ffmpeg::preflight(&batch)?;

    let n = batch.options.len();
    let mut any_failed = false;
    for (i, opts) in batch.options.iter().enumerate() {
        if n > 1 {
            eprintln!("[{}/{n}] {}", i + 1, opts.input.display());
        }
        let mut reporter = IndicatifReporter::new();
        if let Err(e) = pipeline::run(opts, &mut reporter) {
            eprintln!("error: {}: {e}", opts.input.display());
            any_failed = true;
        }
    }

    if any_failed {
        Err("one or more files failed".into())
    } else {
        Ok(())
    }
}
