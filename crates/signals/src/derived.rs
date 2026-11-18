//! `Signal::derived`: a signal kept equal to a closure of other signals.

use crate::effect::Effect;
use crate::signal::Signal;
use crate::track::untrack;

impl<T: 'static> Signal<T> {
    /// A signal holding `f()`, kept current: `f` runs once untracked
    /// for the initial value, then again whenever a signal it reads
    /// changes. Use it where a `Signal<T>` is expected but the value
    /// is computed, as in `transform={Signal::derived(move || tumble(time.get()))}`.
    ///
    /// The signal and its updating effect belong to the owner current
    /// at the call, as `Effect::new` does: in a `{for}` row they end
    /// with the row, and disposing that owner stops the updates.
    #[track_caller]
    pub fn derived(mut f: impl FnMut() -> T + 'static) -> Signal<T> {
        let signal = Signal::new(untrack(&mut f));
        Effect::new(move || signal.set(f()));
        signal
    }
}
