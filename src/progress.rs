use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

/// Build a progress bar for an ffmpeg phase. If `total_us` is `Some`, use a
/// determinate bar keyed on microseconds of encoded output; otherwise use a
/// spinner. The bar auto-hides when stderr is not a TTY.
pub(crate) fn ffmpeg_bar(total_us: Option<u64>, label: &str) -> ProgressBar {
    let target = ProgressDrawTarget::stderr();
    let bar = match total_us {
        Some(total) => {
            let bar = ProgressBar::with_draw_target(Some(total), target);
            bar.set_style(
                ProgressStyle::with_template(
                    "{prefix:>12.cyan.bold} [{bar:30}] {percent:>3}% ({eta})",
                )
                .unwrap()
                .progress_chars("=> "),
            );
            bar
        }
        None => {
            let bar = ProgressBar::with_draw_target(None, target);
            bar.set_style(
                ProgressStyle::with_template("{prefix:>12.cyan.bold} {spinner} {msg}").unwrap(),
            );
            bar.enable_steady_tick(std::time::Duration::from_millis(120));
            bar
        }
    };
    bar.set_prefix(label.to_string());
    bar
}

/// `gifski::progress::ProgressReporter` implementation that ticks an
/// indicatif bar. gifski tells us a total via `init(total)` and then calls
/// `increase()` once per completed frame; `done()` clears the bar.
pub(crate) struct GifskiBar {
    bar: ProgressBar,
    // gifski's init may be called multiple times; track frames seen.
    seen: AtomicUsize,
}

impl GifskiBar {
    pub(crate) fn new() -> Self {
        let target = ProgressDrawTarget::stderr();
        let bar = ProgressBar::with_draw_target(None, target);
        bar.set_style(
            ProgressStyle::with_template(
                "{prefix:>12.cyan.bold} [{bar:30}] {percent:>3}% ({eta})",
            )
            .unwrap()
            .progress_chars("=> "),
        );
        bar.set_prefix("encoding gif");
        Self {
            bar,
            seen: AtomicUsize::new(0),
        }
    }

    pub(crate) fn inner(&self) -> &ProgressBar {
        &self.bar
    }
}

impl gifski::progress::ProgressReporter for GifskiBar {
    fn increase(&mut self) -> bool {
        let n = self.seen.fetch_add(1, Ordering::Relaxed) as u64 + 1;
        self.bar.set_position(n);
        true
    }

    fn done(&mut self, _msg: &str) {
        self.bar.finish_and_clear();
    }
}

/// Probe the input's duration in seconds via ffprobe. Returns `None` if
/// ffprobe is missing or the output can't be parsed — progress then falls
/// back to a spinner. ffprobe is a soft dependency, not required for
/// encoding.
pub(crate) fn probe_duration(input: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0)
}
