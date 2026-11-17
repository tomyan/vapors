//! The commits a push sends, checked (review of slice 4, M1): the diff
//! check reads only the branch's net change, while a push sends every
//! commit reachable from its tip. So the range from the fork to the tip
//! must be a line: each commit has one parent, which is in the range or
//! is the fork, and no other history rides along.

/// `git rev-list --parents <fork>..<tip>`, checked; how many commits.
///
/// # Errors
///
/// No commit, a commit with other than one parent, or a parent outside
/// the range and not `fork`, each named.
pub fn linear(rev_list: &str, fork: &str) -> Result<usize, String> {
    let commits: Vec<Vec<&str>> = rev_list
        .lines()
        .map(|line| line.split(' ').collect())
        .collect();
    if commits.is_empty() {
        return Err(format!(
            "no commit after {fork}: commit your work on the branch"
        ));
    }
    let in_range = |id: &str| id == fork || commits.iter().any(|commit| commit[0] == id);
    for commit in &commits {
        match commit.as_slice() {
            [_, parent] if in_range(parent) => {}
            [id, parent] => {
                return Err(format!(
                    "{id}'s parent {parent} is not on the branch since {fork}: push a line of commits from origin/main"
                ));
            }
            [id, ..] => {
                return Err(format!(
                    "{id} has {} parents: push a line of commits, no merges",
                    commit.len() - 1
                ));
            }
            [] => return Err("an empty `git rev-list --parents` line".into()),
        }
    }
    Ok(commits.len())
}

#[cfg(test)]
#[path = "api_push_history_tests.rs"]
mod tests;
