//! The curls forwarding requests, by pid, so a run whose drain deadline
//! passes can end the ones still in flight (review of #476-#478, N3):
//! each holds the key and may still be generating billed tokens that no
//! one will read.

use std::collections::BTreeSet;
use std::io;
use std::process::{Child, Command, ExitStatus};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The pids of the curls started and not yet reaped.
#[derive(Default)]
pub struct Curls(Mutex<BTreeSet<u32>>);

impl Curls {
    /// `curl` is in flight.
    pub fn started(&self, curl: &Child) {
        self.held().insert(curl.id());
    }

    /// `curl` reaped, once its output has ended (until then it is in
    /// flight and [`Curls::killed`] reaches it). Its pid is taken out
    /// first, under the lock `killed` holds, so a kill never reaches a
    /// pid that has been reaped and could belong to another process.
    pub fn reaped(&self, curl: &mut Child) -> io::Result<ExitStatus> {
        self.held().remove(&curl.id());
        curl.wait()
    }

    /// SIGKILL to every curl still in flight; how many there were.
    ///
    /// # Errors
    ///
    /// `kill` failing for one, naming its pid.
    pub fn killed(&self) -> Result<usize, String> {
        let held = self.held();
        for pid in held.iter() {
            let sent = Command::new("kill")
                .args(["-s", "KILL", "--", &pid.to_string()])
                .output();
            match sent {
                Ok(out) if out.status.success() => {}
                Ok(out) => {
                    return Err(format!(
                        "kill {pid} failed: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    ));
                }
                Err(error) => return Err(format!("kill {pid} did not run: {error}")),
            }
        }
        Ok(held.len())
    }

    fn held(&self) -> MutexGuard<'_, BTreeSet<u32>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
