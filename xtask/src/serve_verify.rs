//! After a publish, the page as a phone gets it: `/` and the page's wasm
//! fetched through the tailnet URL with `Accept-Encoding: br, gzip`. A
//! file sent without `Content-Encoding`, or with cache headers other than
//! the server's, fails the serve.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};

/// One fetch's deadline: the 2.8 MB wasm compressed is under 1 MB, which
/// the box sends itself over the tailnet in well under a second
/// (2026-10-26); 30 s allows a busy box.
const FETCH_DEADLINE: Duration = Duration::from_secs(30);

/// What a response's headers said.
#[derive(Debug, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub encoding: Option<String>,
    pub cache: Option<String>,
}

/// The status line and the two headers, from `curl -D -`'s output.
pub fn answer(headers: &str) -> Option<Answer> {
    let mut lines = headers.lines();
    let status = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let mut answer = Answer {
        status,
        encoding: None,
        cache: None,
    };
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = Some(value.trim().to_owned());
        match name.trim().to_ascii_lowercase().as_str() {
            "content-encoding" => answer.encoding = value,
            "cache-control" => answer.cache = value,
            _ => {}
        }
    }
    Some(answer)
}

/// `Ok` when the answer is compressed and cached as `cache` says.
///
/// # Errors
///
/// What differed, naming the URL and the fix.
pub fn judge(url: &str, answer: &Answer, cache: &str) -> Result<(), String> {
    if answer.status != 200 {
        return Err(format!(
            "{url} answered {}; fix: journalctl --user -u xtask-serve-*",
            answer.status
        ));
    }
    if !matches!(answer.encoding.as_deref(), Some("br" | "gzip")) {
        return Err(format!(
            "{url} came with Content-Encoding {:?}, not br or gzip; fix: check the unit runs with --compression-static and the .br copies exist (cargo xtask web)",
            answer.encoding
        ));
    }
    if answer.cache.as_deref() != Some(cache) {
        return Err(format!(
            "{url} came with Cache-Control {:?}, not {cache:?}; fix: the server's config (serve_static.rs)",
            answer.cache
        ));
    }
    Ok(())
}

/// Fetch `/` and the largest wasm under `dir` from `base` and judge both.
///
/// # Errors
///
/// No wasm published, curl failing, or a file judged wrong.
pub fn verify(base: &str, dir: &Path) -> Result<String, String> {
    let wasm = largest_wasm(dir)?;
    let name = wasm
        .strip_prefix(dir)
        .map_err(|e| e.to_string())?
        .display()
        .to_string();
    let page = base.to_owned();
    judge(&page, &fetch(&page)?, "no-cache")?;
    let file = format!("{base}{name}");
    let got = fetch(&file)?;
    judge(&file, &got, "public, max-age=31536000, immutable")?;
    Ok(format!(
        "{name} sent as {}",
        got.encoding.unwrap_or_default()
    ))
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

fn largest_wasm(dir: &Path) -> Result<PathBuf, String> {
    std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "wasm"))
        .max_by_key(|path| path.metadata().map_or(0, |m| m.len()))
        .ok_or_else(|| {
            format!(
                "{} has no .wasm to check; fix: cargo xtask web",
                dir.display()
            )
        })
}

#[cfg(test)]
#[path = "serve_verify_tests.rs"]
mod tests;
