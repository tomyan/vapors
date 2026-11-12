//! `--only`: the workloads whose name contains a substring.

/// The entries of `all` whose name contains `only`, or all of them
/// without it.
///
/// # Errors
/// `only` matches no name; the message lists the names.
pub fn matching<T>(all: Vec<(String, T)>, only: Option<&str>) -> Result<Vec<(String, T)>, String> {
    let Some(only) = only else { return Ok(all) };
    let names = all.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>();
    let chosen: Vec<_> = all
        .into_iter()
        .filter(|(name, _)| name.contains(only))
        .collect();
    if chosen.is_empty() {
        return Err(format!(
            "--only {only:?} matches no workload; the names are: {}",
            names.join("; ")
        ));
    }
    Ok(chosen)
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
