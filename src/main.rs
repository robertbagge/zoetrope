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
    let opts = cli::Args::parse().into_options()?;
    pipeline::check_ffmpeg()?;
    pipeline::run(&opts)
}
