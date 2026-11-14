//! One merge-queue pass at a time per repository: an exclusive lock on
//! `<log>.lock`, held for the whole pass and freed when the process
//! exits, however it exits. A pass started while another runs (by hand,
//! or a dry run) is refused, naming the holder, instead of sharing the
//! run worktree.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::Path;

/// The held lock; dropping it frees the lock.
#[derive(Debug)]
pub struct Held {
    file: File,
    path: std::path::PathBuf,
}

impl Drop for Held {
    /// Unlock before the file closes: a process spawned while the lock was
    /// held has a copy of the descriptor until it execs, and the lock,
    /// which belongs to the open file, would outlive this close until then.
    fn drop(&mut self) {
        if let Err(e) = self.file.unlock() {
            eprintln!(
                "queue: could not unlock {}: {e}; it is freed when the last copy of its descriptor closes",
                self.path.display()
            );
        }
    }
}

/// Take the pass lock at `path`.
///
/// # Errors
///
/// It is held (naming the holder's pid), or the file cannot be used.
pub fn take(path: &Path) -> Result<Held, String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    lock(&file, path)?;
    file.set_len(0)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writeln!(file, "pid {}", std::process::id()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Held {
        file,
        path: path.to_path_buf(),
    })
}

/// Lock `file` without waiting; refused naming the holder's pid when
/// another pass has it.
fn lock(file: &std::fs::File, path: &Path) -> Result<(), String> {
    match file.try_lock() {
        Ok(()) => Ok(()),
        Err(TryLockError::WouldBlock) => {
            let holder = std::fs::read_to_string(path).unwrap_or_default();
            Err(format!(
                "another pass holds {} ({}); this one stops",
                path.display(),
                holder.trim()
            ))
        }
        Err(TryLockError::Error(e)) => Err(format!("{}: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_pass_is_refused_while_the_first_holds_the_lock() {
        let scratch = crate::scratch::dir("queue-lock");
        let path = scratch.join("queue.lock");
        let first = take(&path).expect("first");
        let refused = take(&path).expect_err("held");
        assert!(
            refused.contains("another pass holds") && refused.contains("pid"),
            "{refused}"
        );
        drop(first);
        assert!(take(&path).is_ok(), "freed when dropped");
    }

    #[test]
    fn dropping_the_lock_frees_it_while_a_copy_of_its_descriptor_lives() {
        // Given: a copy of the descriptor, as a child spawned by another
        // thread holds one between its fork and its exec
        let scratch = crate::scratch::dir("queue-lock-copy");
        let path = scratch.join("queue.lock");
        let first = take(&path).expect("first");
        let copy = first.file.try_clone().expect("a copy");

        // When
        drop(first);

        // Then
        assert!(take(&path).is_ok(), "the copy kept the lock");
        drop(copy);
    }
}
