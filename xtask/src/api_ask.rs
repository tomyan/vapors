//! `ask_lead`, outside (`docs/plans/api-builder.md`): the session's
//! question is written to `q<N>.md` in this run's questions directory
//! and its path logged, where the lead watches; the answer is read from
//! `a<N>.md` beside it, which the lead writes as `a<N>.md.tmp` and
//! renames, so it appears whole. With no answer by the deadline the
//! session is told to write `BLOCKED.md` and stop.
//!
//! A question is the session's bytes, so it is held to [`QUESTION_BYTES`]
//! and refused if it holds a control byte other than newline and tab:
//! the lead reads `q<N>.md` in a terminal (review of #485, H2 and M1).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// How long a question waits: the lead's number (queue rev 8, item 3,
/// answer 7), so a session blocked on the lead costs at most half an
/// hour of its deadline.
pub const ANSWER_DEADLINE: Duration = Duration::from_mins(30);

/// The most bytes in one question: the lead's number (L45). A question
/// is read by a person; 64 KiB is pages of text.
pub const QUESTION_BYTES: usize = 64 * 1024;

/// The most questions in one run: the lead's number (L45). The lead
/// answers each by hand, and a slice that needs more than 20 answers
/// has a brief to rewrite, not more questions to ask; with the size cap
/// it bounds what a run writes here to 1.25 MiB.
pub const QUESTIONS: u64 = 20;

/// How often the answer file is looked for.
const POLL: Duration = Duration::from_secs(5);

/// This run's questions directory and the next question's number.
pub struct Asks {
    pub dir: PathBuf,
    next: AtomicU64,
}

impl Asks {
    /// Questions written to a new directory `run` under `parent`, so no
    /// earlier run's answer can answer this run's question (review of
    /// #485, H1), numbered from 1.
    ///
    /// # Errors
    ///
    /// The directory already there, or not made private, naming it.
    pub fn for_run(parent: &Path, run: &str) -> Result<Self, String> {
        let dir = parent.join(run);
        std::fs::create_dir(&dir).map_err(|e| {
            format!(
                "making {}: {e}; each run's questions start in a new directory",
                dir.display()
            )
        })?;
        let private = std::os::unix::fs::PermissionsExt::from_mode(0o700);
        std::fs::set_permissions(&dir, private)
            .map_err(|e| format!("making {} private: {e}", dir.display()))?;
        Ok(Self {
            dir,
            next: AtomicU64::new(1),
        })
    }

    /// `question` asked; the answer, waited for up to `deadline`.
    ///
    /// # Errors
    ///
    /// A question refused (its size, a control byte, past [`QUESTIONS`]),
    /// not written, its answer unreadable or empty, or no answer by the
    /// deadline: the text the session reads, saying what to do.
    pub fn ask(&self, question: &str, deadline: Duration) -> Result<String, String> {
        checked(question)?;
        let number = self.next.fetch_add(1, Ordering::Relaxed);
        if number > QUESTIONS {
            return Err(format!(
                "question {number} is past the {QUESTIONS} a run may ask: write what blocks you \
                 in BLOCKED.md and stop"
            ));
        }
        let asked = self.dir.join(format!("q{number}.md"));
        let answer = self.dir.join(format!("a{number}.md"));
        std::fs::write(&asked, question)
            .map_err(|e| format!("writing {}: {e}", asked.display()))?;
        eprintln!(
            "api-build: question {number} for the lead: {}; write the answer to {}.tmp, then \
             rename it to {}",
            asked.display(),
            answer.display(),
            answer.display()
        );
        waited(&answer, deadline, POLL.min(deadline)).inspect_err(|why| {
            eprintln!(
                "api-build: question {number} ({}) ended unanswered: {why}",
                asked.display()
            );
        })
    }
}

/// Refuses a question over [`QUESTION_BYTES`] or holding a control byte
/// other than newline and tab, naming its offset.
fn checked(question: &str) -> Result<(), String> {
    if question.len() > QUESTION_BYTES {
        return Err(format!(
            "a question of {} bytes, over the {QUESTION_BYTES} a question may hold",
            question.len()
        ));
    }
    let control = question
        .char_indices()
        .find(|&(_, c)| c.is_control() && c != '\n' && c != '\t');
    match control {
        Some((at, c)) => Err(format!(
            "a question holding the control character {c:?} at byte {at}: send text, newlines \
             and tabs only"
        )),
        None => Ok(()),
    }
}

/// The answer at `answer`, once it is there: absent keeps the wait going,
/// and anything else ends it (review of #485, M2): an answer that cannot
/// be read, or is empty, is refused, naming it.
fn waited(answer: &Path, deadline: Duration, poll: Duration) -> Result<String, String> {
    let started = Instant::now();
    loop {
        match std::fs::read_to_string(answer) {
            Ok(text) if text.trim().is_empty() => {
                return Err(format!("the answer {} is empty", answer.display()));
            }
            Ok(text) => return Ok(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "the answer {} could not be read: {e}",
                    answer.display()
                ));
            }
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
