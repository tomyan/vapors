//! The gate's floors, checked before it starts: free inodes in the temp
//! dir's filesystem, and free space under target/. On 2026-11-02 /tmp, a
//! tmpfs of 1,048,576 inodes, ran out of inodes with 23 GB free and
//! every shell on the machine failed; the disk was at 120 GB free and
//! falling. A gate that starts below either floor could be the one that
//! runs out, so it refuses and says what to remove.

use std::path::Path;
use std::process::Command;

/// Free inodes the temp dir must have. Measured 2026-11-02: a vapors
/// `cargo xtask ci` run's TMPDIR peaked at 1,275 entries (sampled every
/// 0.5 s), and /tmp's inodes in use rose by 1,069 over a touche gate
/// (12:39 to 12:46Z, other agents' work included). 50,000 is about 40
/// times either, leaving room for every other agent's shells, builds
/// and tests while the gate runs.
pub const INODE_FLOOR: u64 = 50_000;
/// Free bytes under target/. Measured 2026-11-02: over a touche gate
/// (12:39 to 12:46Z) its target directory grew from 3.4 GB to 17.7 GB,
/// 14.4 GB. 30 GB is about twice that, so a gate started at the floor
/// ends with room left for other worktrees' builds meanwhile.
pub const DISK_FLOOR: u64 = 30 * GB;

const GB: u64 = 1_000_000_000;

/// What a filesystem has free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Free {
    /// Inodes not in use; none for a filesystem that reports no inode
    /// count (a total of 0, as btrfs and some overlays do), which has no
    /// inode limit to run out of.
    pub inodes: Option<u64>,
    /// Bytes an unprivileged user may still write.
    pub bytes: u64,
}

/// Refuse when the temp dir or target/ is under its floor.
///
/// # Errors
/// Either is under its floor, or `stat` could not say, naming the fix.
pub fn before() -> Result<(), String> {
    let tmp = std::env::temp_dir();
    let target = target_dir(Path::new("."))?;
    let tmp_free = free(&tmp)?;
    if tmp_free.inodes.is_none() {
        println!(
            "floor: {} reports no inode count; its inode floor is not checked",
            tmp.display()
        );
    }
    check((&tmp, tmp_free), (&target, free(&target)?))
}

/// `<workspace>/target`, or the workspace itself before its first build
/// (target/ will be made there).
///
/// # Errors
/// `target` is a link to nothing: its space cannot be read, and the
/// workspace's would be the wrong disk.
pub fn target_dir(workspace: &Path) -> Result<std::path::PathBuf, String> {
    let target = workspace.join("target");
    match target.symlink_metadata() {
        Err(_) => Ok(workspace.to_path_buf()),
        Ok(_) if target.exists() => Ok(target),
        Ok(_) => Err(format!(
            "{} is a link to {}, which does not exist; \
             fix: mkdir -p the directory it names, as the worktree recipe in CLAUDE.md does",
            target.display(),
            std::fs::read_link(&target).map_or_else(|e| e.to_string(), |p| p.display().to_string())
        )),
    }
}

/// `stat -L -f` on `path`, judged by its exit code.
fn free(path: &Path) -> Result<Free, String> {
    let output = Command::new("stat")
        .args(["-L", "-f", "-c", "%c %d %a %S"])
        .arg(path)
        .output()
        .map_err(|e| format!("stat: {e}; fix: install coreutils"))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        return Err(format!("stat -f {}: {}", path.display(), said.trim()));
    }
    parse(&String::from_utf8_lossy(&output.stdout), path)
}

/// `stat -f -c '%c %d %a %S'`'s line: total and free inodes, free blocks
/// for users, block size.
///
/// # Errors
/// Any other shape, naming the path and the text.
pub fn parse(text: &str, path: &Path) -> Result<Free, String> {
    let numbers: Option<Vec<u64>> = text.split_whitespace().map(|n| n.parse().ok()).collect();
    match numbers.as_deref() {
        Some(&[total, inodes, blocks, size]) => Ok(Free {
            inodes: (total > 0).then_some(inodes),
            bytes: blocks.saturating_mul(size),
        }),
        _ => Err(format!(
            "stat -f {} printed `{}`, not total and free inodes, free blocks and block size",
            path.display(),
            text.trim()
        )),
    }
}

/// Refuse when `tmp` has too few free inodes or `target` too little space.
///
/// # Errors
/// Naming which, the numbers, and what to remove.
pub fn check(tmp: (&Path, Free), target: (&Path, Free)) -> Result<(), String> {
    let (tmp_path, tmp_free) = tmp;
    if let Some(inodes) = tmp_free.inodes.filter(|&free| free < INODE_FLOOR) {
        return Err(format!(
            "{} has {inodes} free inodes, under the gate's floor of {INODE_FLOOR}; \
             fix: remove what tests and browsers left there (`ls {}` for the oldest), then rerun",
            tmp_path.display(),
            tmp_path.display()
        ));
    }
    let (target_path, target_free) = target;
    if target_free.bytes < DISK_FLOOR {
        return Err(format!(
            "{} has {} GB free, under the gate's floor of {} GB; \
             fix: empty the target directories of merged branches \
             (`find ~/.cache/*-targets/<name> -mindepth 1 -maxdepth 1 -exec rm -rf {{}} +`), then rerun",
            target_path.display(),
            target_free.bytes / GB,
            DISK_FLOOR / GB
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "gate_floor_tests.rs"]
mod tests;
