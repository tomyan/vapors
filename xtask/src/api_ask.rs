//! `ask_lead`, outside (`docs/plans/api-builder.md`): the session's
//! question is written to `q<N>.md` in the builder's questions directory
//! and its path logged, where the lead watches; the answer is read from
//! `a<N>.md` beside it. With no answer by the deadline the session is
//! told to write `BLOCKED.md` and stop.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// How long a question waits: the lead's number (queue rev 8, item 3,
/// answer 7), so a session blocked on the lead costs at most half an
/// hour of its deadline.
pub const ANSWER_DEADLINE: Duration = Duration::from_mins(30);

/// How often the answer file is looked for.
const POLL: Duration = Duration::from_secs(5);

/// The questions directory and the next question's number.
pub struct Asks {
    pub dir: PathBuf,
    next: AtomicU64,
}

impl Asks {
    /// Questions written to `dir`, numbered from 1.
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            next: AtomicU64::new(1),
        }
    }

    /// `question` asked; the answer, waited for up to `deadline`.
    ///
    /// # Errors
    ///
    /// The question not written, or no answer by the deadline: the text
    /// the session reads, saying what to do.
    pub fn ask(&self, question: &str, deadline: Duration) -> Result<String, String> {
        let number = self.next.fetch_add(1, Ordering::Relaxed);
        let (asked, answer) = (
            self.dir.join(format!("q{number}.md")),
            self.dir.join(format!("a{number}.md")),
        );
        std::fs::write(&asked, question)
            .map_err(|e| format!("writing {}: {e}", asked.display()))?;
        eprintln!(
            "api-build: question {number} for the lead: {}; write the answer to {}",
            asked.display(),
            answer.display()
        );
        waited(&answer, deadline, POLL.min(deadline))
    }
}

fn waited(answer: &Path, deadline: Duration, poll: Duration) -> Result<String, String> {
    let started = Instant::now();
    loop {
        if let Ok(text) = std::fs::read_to_string(answer)
            && !text.trim().is_empty()
        {
            return Ok(text);
        }
        if started.elapsed() >= deadline {
            return Err(format!(
                "no answer from the lead within {} minutes: write what blocks you in BLOCKED.md at \
                 the clone's root and stop",
                deadline.as_secs() / 60
            ));
        }
        thread::sleep(poll);
    }
}

#[cfg(test)]
#[path = "api_ask_tests.rs"]
mod tests;
