use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use zoetrope_core::ProgressReporter;

fn determinate_style() -> ProgressStyle {
    ProgressStyle::with_template("{prefix:>12.cyan.bold} [{bar:30}] {percent:>3}% ({eta})")
        .unwrap()
        .progress_chars("=> ")
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{prefix:>12.cyan.bold} {spinner} {msg}").unwrap()
}

/// Terminal-backed `ProgressReporter`. One bar per phase; constructed fresh
/// each `start_phase` to match indicatif's draw model. Auto-hides when
/// stderr is not a TTY (indicatif default).
pub struct IndicatifReporter {
    bar: Option<ProgressBar>,
}

impl IndicatifReporter {
    pub fn new() -> Self {
        Self { bar: None }
    }
}

impl Default for IndicatifReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for IndicatifReporter {
    fn start_phase(&mut self, label: &str, total: Option<u64>) {
        // Drop any prior bar without printing — same idempotent behavior as
        // calling finish_phase before start_phase.
        if let Some(prev) = self.bar.take() {
            prev.finish_and_clear();
        }

        let target = ProgressDrawTarget::stderr();
        let bar = match total {
            Some(t) => {
                let bar = ProgressBar::with_draw_target(Some(t), target);
                bar.set_style(determinate_style());
                bar
            }
            None => {
                let bar = ProgressBar::with_draw_target(None, target);
                bar.set_style(spinner_style());
                bar.enable_steady_tick(std::time::Duration::from_millis(120));
                bar
            }
        };
        bar.set_prefix(label.to_string());
        self.bar = Some(bar);
    }

    fn set_position(&mut self, pos: u64) {
        if let Some(bar) = self.bar.as_ref() {
            bar.set_position(pos);
        }
    }

    fn finish_phase(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }

    fn status(&mut self, msg: &str) {
        eprintln!("{msg}");
    }
}
