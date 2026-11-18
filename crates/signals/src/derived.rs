//! `Signal::derived`: a signal kept equal to a closure of other signals.

use crate::effect::Effect;
use crate::read_signal::ReadSignal;
use crate::signal::Signal;
use crate::track::untrack;

impl<T: 'static> Signal<T> {
    /// A read-only value holding `f()`, kept current: `f` runs once
    /// untracked for the initial value, then again whenever a signal it
    /// reads changes. Use it where a `ReadSignal<T>` is expected but the
    /// value is computed, as in `transform={Signal::derived(move || tumble(time.get()))}`.
    /// It has no setter, since a write would be overwritten at the next
    /// change of an input:
    ///
    /// ```compile_fail
    /// use vapors_signals::Signal;
    /// Signal::derived(|| 1).set(2);
    /// ```
    ///
    /// Built on an effect, not a `Memo`, because a memo needs
    /// `T: PartialEq` and `derived` takes any `T`; the effect runs
    /// synchronously, so the value never lags its inputs.
    ///
    /// The signal and its updating effect belong to the owner current
    /// at the call, as `Effect::new` does: in a `{for}` row they end
    /// with the row, and disposing that owner stops the updates.
    #[track_caller]
    pub fn derived(mut f: impl FnMut() -> T + 'static) -> ReadSignal<T> {
        let signal = Signal::new(untrack(&mut f));
        Effect::new(move || signal.set(f()));
        signal.into()
    }
}
