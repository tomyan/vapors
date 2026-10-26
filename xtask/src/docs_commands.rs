//! The CLAUDE.md rule of `cargo xtask check-docs`: every command it
//! names must exist. A `cargo xtask <name>` must be an xtask subcommand;
//! a `cargo run -p <package> --example <name>` an example of that
//! package; and a `cargo run -p <app> -- <command>` a command in the
//! app's `--help`, for the app `[docs] app` names. A `cargo run` in
//! neither shape is refused. Each failure names the file and line, and
//! the fix.

/// The app whose command line CLAUDE.md documents, and its commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    /// Its package.
    pub package: String,
    /// The commands its `--help` lists.
    pub commands: Vec<String>,
}

/// Every command `text` names that does not exist, as `file:line: why`.
#[must_use]
pub fn check(
    file: &str,
    text: &str,
    xtask: &[&str],
    app: Option<&App>,
    examples: &dyn Fn(&str, &str) -> bool,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let at = format!("{file}:{}", n + 1);
        problems.extend(xtask_problems(&at, line, xtask));
        if let Some(run) = line.find("cargo run ").map(|i| &line[i..]) {
            problems.extend(run_problem(&at, run, app, examples));
        }
    }
    problems
}

fn xtask_problems(at: &str, line: &str, xtask: &[&str]) -> Vec<String> {
    line.match_indices("cargo xtask ")
        .filter_map(|(i, prefix)| {
            let name: String = line[i + prefix.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            (!name.is_empty() && !name.starts_with('-') && !xtask.contains(&name.as_str())).then(|| {
                format!("{at}: `cargo xtask {name}` is not an xtask command; fix: name one in `cargo xtask --help`, or remove it")
            })
        })
        .collect()
}

/// The problem with one `cargo run …` (up to a backtick or the end).
fn run_problem(
    at: &str,
    run: &str,
    app: Option<&App>,
    examples: &dyn Fn(&str, &str) -> bool,
) -> Option<String> {
    let run = run.split('`').next().unwrap_or(run).trim_end();
    let words: Vec<&str> = run.split_whitespace().collect();
    let package = words
        .iter()
        .position(|w| *w == "-p")
        .and_then(|i| words.get(i + 1))
        .copied()?;
    let after = |flag: &str| {
        words
            .iter()
            .position(|w| *w == flag)
            .and_then(|i| words.get(i + 1))
            .copied()
    };
    if let Some(example) = after("--example") {
        return (!examples(package, example)).then(|| {
            format!("{at}: `--example {example}` is not an example of {package}; fix: name one in that package's examples/, or remove it")
        });
    }
    let Some(command) = after("--") else {
        return Some(format!(
            "{at}: `{run}` names no example and no subcommand; fix: add one, or remove the line"
        ));
    };
    match app {
        Some(app) if app.package == package => (!app.commands.iter().any(|c| c == command)).then(|| {
            format!("{at}: `{package} {command}` is not a {package} command; fix: name one in `cargo run -p {package} -- --help`")
        }),
        _ => Some(format!(
            "{at}: `{run}` runs {package}, which [docs] app does not name; fix: set [docs] app = \"{package}\" in .config/factory.toml"
        )),
    }
}

/// The command names under `Commands:` in clap's help.
///
/// # Errors
///
/// A help with no `Commands:` list.
pub fn commands_in_help(help: &str) -> Result<Vec<String>, String> {
    let mut lines = help.lines().skip_while(|l| l.trim() != "Commands:");
    if lines.next().is_none() {
        return Err("the app's --help has no Commands: list".into());
    }
    Ok(lines
        .take_while(|l| l.starts_with("  "))
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect())
}

#[cfg(test)]
#[path = "docs_commands_tests.rs"]
mod tests;
