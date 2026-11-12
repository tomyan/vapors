//! The page as a browser gets it from `base` (the loopback before the
//! mount, the tailnet URL after): `/`, every `.wasm` in the copy, and the
//! first script `index.html` loads, each fetched with `Accept-Encoding:
//! br, gzip`. Each must answer 200, compressed, with its Content-Type,
//! `Vary: Accept-Encoding` and the Cache-Control `serve_unit.rs` sets.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::serve_verify_rules::{Answer, Expect, answer, expect_for, first_script, judge};

/// One fetch's deadline: the 2.8 MB wasm compressed is under 1 MB, which
/// the box sends itself over the tailnet in well under a second
/// (2026-10-26); 30 s allows a busy box.
const FETCH_DEADLINE: Duration = Duration::from_secs(30);

/// Fetch and judge each file of `dir` at `base`, which ends in `/`.
///
/// # Errors
///
/// No wasm or no script to check, curl failing, or a file judged wrong.
pub fn verify(base: &str, dir: &Path) -> Result<String, String> {
    let mut checked = Vec::new();
    for (path, expect) in targets(dir)? {
        let url = format!("{base}{path}");
        let got = fetch(&url)?;
        judge(&url, &got, &expect)?;
        checked.push(format!(
            "{} as {}",
            if path.is_empty() { "/" } else { &path },
            got.encoding.unwrap_or_default()
        ));
    }
    Ok(checked.join(", "))
}

/// `/`, every wasm, and the first script index.html loads.
fn targets(dir: &Path) -> Result<Vec<(String, Expect)>, String> {
    let mut targets = vec![(String::new(), expect_for("index.html"))];
    let wasm = wasm_under(dir, dir)?;
    if wasm.is_empty() {
        return Err(format!(
            "{} has no .wasm to check; fix: cargo xtask web",
            dir.display()
        ));
    }
    targets.extend(wasm.into_iter().map(|path| {
        let expect = expect_for(&path);
        (path, expect)
    }));
    let index = std::fs::read_to_string(dir.join("index.html"))
        .map_err(|e| format!("{}: {e}", dir.join("index.html").display()))?;
    let script = first_script(&index).ok_or_else(|| {
        format!(
            "{}/index.html loads no relative script to check",
            dir.display()
        )
    })?;
    let expect = expect_for(script.split('?').next().unwrap_or(&script));
    targets.push((script, expect));
    Ok(targets)
}

fn fetch(url: &str) -> Result<Answer, String> {
    let mut command = Command::new("curl");
    command.args([
        "-sS",
        "-o",
        "/dev/null",
        "-D",
        "-",
        "-H",
        "Accept-Encoding: br, gzip",
        url,
    ]);
    match run_with_deadline(command, FETCH_DEADLINE)? {
        Finished::Exited {
            success: true,
            output,
        } => answer(&output).ok_or_else(|| format!("{url}: curl printed no status line: {output}")),
        Finished::Exited { output, .. } => Err(format!("curl {url} failed: {output}")),
        Finished::TimedOut { .. } => Err(format!(
            "{url} did not answer within {} s",
            FETCH_DEADLINE.as_secs()
        )),
    }
}

/// Every `.wasm` under `dir`, at any depth, relative to `root`.
fn wasm_under(dir: &Path, root: &Path) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path: PathBuf = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.is_dir() {
            found.extend(wasm_under(&path, root)?);
        } else if path.extension().is_some_and(|e| e == "wasm") {
            found.push(
                path.strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .display()
                    .to_string(),
            );
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
#[path = "serve_verify_tests.rs"]
mod tests;
