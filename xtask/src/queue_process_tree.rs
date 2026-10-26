//! A process's descendants and their process groups, read from /proc
//! (Linux; no libc in xtask). A process that ends while /proc is read is
//! skipped.

use std::collections::BTreeMap;

/// The process groups of every descendant of `root`.
#[must_use]
pub fn descendant_groups(root: u32) -> Vec<u32> {
    let table = processes();
    let mut found = vec![root];
    let mut groups = Vec::new();
    let mut at = 0;
    while at < found.len() {
        let parent = found[at];
        for (pid, (ppid, group)) in &table {
            if *ppid == parent && !found.contains(pid) {
                found.push(*pid);
                if !groups.contains(group) {
                    groups.push(*group);
                }
            }
        }
        at += 1;
    }
    groups
}

/// Each process's parent and group: pid to (ppid, pgrp).
fn processes() -> BTreeMap<u32, (u32, u32)> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return BTreeMap::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let stat = std::fs::read_to_string(entry.path().join("stat")).ok()?;
            Some((pid, parent_and_group(&stat)?))
        })
        .collect()
}

/// `/proc/<pid>/stat`'s ppid and pgrp: the 2nd and 3rd fields after the
/// command, which is in parentheses and may hold spaces.
#[must_use]
pub fn parent_and_group(stat: &str) -> Option<(u32, u32)> {
    let after = &stat[stat.rfind(')')? + 1..];
    let mut fields = after.split_whitespace().skip(1);
    Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stat_line_s_parent_and_group_are_read_past_a_command_with_spaces() {
        let stat = "4242 (cargo test (x)) S 4200 4100 4100 0 -1 4194560";
        assert_eq!(parent_and_group(stat), Some((4200, 4100)));
    }
}
