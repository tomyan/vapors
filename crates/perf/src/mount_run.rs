//! `--mount`: the Mount page's anonymous memory at each size of
//! [`SIZES`] and a 0-row baseline, one process per size
//! (`mount_memory`). Its first frame's times are `--first-frame`'s
//! (`first_frame_run`), one page per process, as a user's launch is.

use crate::mount_memory::{in_processes, memory_table};
use crate::scaling::SIZES;

/// Prints the memory table.
///
/// # Errors
/// A child process that failed, ran past its deadline or printed no
/// memory line, or readings that are not a growth.
pub fn run() -> Result<(), String> {
    let sizes = std::iter::once(0).chain(SIZES).collect::<Vec<_>>();
    println!("### Anonymous memory, first mount and frame, one process per size\n");
    println!("{}", memory_table(&in_processes(&sizes)?)?);
    Ok(())
}
