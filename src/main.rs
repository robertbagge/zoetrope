use clap::Parser;
use std::process::ExitCode;

mod cli;
mod encode;
mod fit;
mod pipeline;
mod progress;

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
    let batch = cli::Args::parse().into_batch()?;
    pipeline::check_ffmpeg()?;
    pipeline::preflight(&batch)?;

    let n = batch.options.len();
    let mut any_failed = false;
    for (i, opts) in batch.options.iter().enumerate() {
        if n > 1 {
            eprintln!("[{}/{n}] {}", i + 1, opts.input.display());
        }
        if let Err(e) = pipeline::run(opts) {
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
