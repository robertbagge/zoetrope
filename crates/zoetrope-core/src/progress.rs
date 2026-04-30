/// Core's progress hook. The CLI plugs in indicatif; a desktop app can plug in
/// a signal-driven impl. All methods take `&mut self` so impls can mutate
/// internal state without an interior-mutability dance.
pub trait ProgressReporter: Send {
    /// Begin a phase. `total` is `Some(N)` for determinate work (frame count
    /// or out_time_us); `None` indicates spinner-style "we don't know".
    fn start_phase(&mut self, label: &str, total: Option<u64>);

    /// Update progress for the current phase. Caller guarantees this is only
    /// called between `start_phase` and `finish_phase`.
    fn set_position(&mut self, pos: u64);

    /// End the current phase. Idempotent — safe to call after the impl has
    /// already cleaned up (e.g. via gifski's own `done()` callback).
    fn finish_phase(&mut self);

    /// Out-of-band status message ("fit attempt 2/5 (...)", "done: out.gif").
    /// CLI writes to stderr; a GUI updates a status line.
    fn status(&mut self, msg: &str);
}

/// A no-op reporter. Use when the caller doesn't care about progress (tests,
/// library use without UI).
pub struct NoopReporter;

impl ProgressReporter for NoopReporter {
    fn start_phase(&mut self, _label: &str, _total: Option<u64>) {}
    fn set_position(&mut self, _pos: u64) {}
    fn finish_phase(&mut self) {}
    fn status(&mut self, _msg: &str) {}
}
