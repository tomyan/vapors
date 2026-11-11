//! Instructions retired by the calling thread in user space, from Linux
//! perf events opened inside the process. The count moves with the code
//! run, not with machine load, so it can judge a change while other
//! builds run. It counts this thread only: GPU work on other threads
//! (lavapipe's workers) is not in it.

/// The sysctl that gates perf events, and its path.
const PARANOID: &str = "/proc/sys/kernel/perf_event_paranoid";

/// A counter of the calling thread's user-space instructions.
pub struct InstructionCounter(imp::Counter);

impl InstructionCounter {
    /// Opens and starts a counter on the calling thread.
    ///
    /// # Errors
    /// Why perf events cannot be opened, naming the sysctl and the value
    /// that allows them; there is no fallback to time.
    pub fn open() -> Result<Self, String> {
        imp::open().map(Self).map_err(|why| refusal(&why))
    }

    /// Instructions retired since [`Self::open`].
    ///
    /// # Errors
    /// The read's failure.
    pub fn read(&mut self) -> Result<u64, String> {
        imp::read(&mut self.0)
    }
}

fn refusal(why: &str) -> String {
    let current = std::fs::read_to_string(PARANOID)
        .map_or_else(|e| format!("unreadable ({e})"), |v| v.trim().to_owned());
    format!(
        "--instructions needs Linux perf events: {why}; kernel.perf_event_paranoid is {current}, \
         user-space counts need 2 or lower: sudo sysctl kernel.perf_event_paranoid=1 \
         (set again after a reboot)"
    )
}

#[cfg(target_os = "linux")]
mod imp {
    use perf_event::Builder;
    pub use perf_event::Counter;
    use perf_event::events::Hardware;

    pub fn open() -> Result<Counter, String> {
        let mut counter = Builder::new(Hardware::INSTRUCTIONS)
            .exclude_kernel(true)
            .exclude_hv(true)
            .build()
            .map_err(|e| format!("perf_event_open failed: {e}"))?;
        counter
            .enable()
            .map_err(|e| format!("enabling the counter failed: {e}"))?;
        Ok(counter)
    }

    pub fn read(counter: &mut Counter) -> Result<u64, String> {
        counter
            .read()
            .map_err(|e| format!("reading the counter failed: {e}"))
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    pub struct Counter;

    pub fn open() -> Result<Counter, String> {
        Err("this is not Linux".to_owned())
    }

    pub fn read(_: &mut Counter) -> Result<u64, String> {
        Err("this is not Linux".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{InstructionCounter, refusal};

    #[test]
    fn the_refusal_names_the_sysctl_and_the_value_it_needs() {
        let message = refusal("perf_event_open failed: denied");
        assert!(message.contains("perf_event_paranoid"), "{message}");
        assert!(message.contains("2 or lower"), "{message}");
        assert!(
            message.contains("sudo sysctl kernel.perf_event_paranoid=1"),
            "{message}"
        );
    }

    #[test]
    fn the_count_rises_with_work_done() {
        let mut counter = InstructionCounter::open().expect("perf events");
        let before = counter.read().expect("reads");
        std::hint::black_box((0..10_000u64).sum::<u64>());
        assert!(counter.read().expect("reads") >= before + 10_000);
    }
}
