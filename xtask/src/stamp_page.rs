//! `cargo xtask stamp-page`: trunk's post-build hook
//! (`crates/web/Trunk.toml`), for `cargo xtask web` and `cargo xtask
//! dev` alike. In trunk's staging directory it writes `version.json`
//! (`web_version`) and gives each of the page's own scripts and styles
//! in `index.html` a `?v=<build>` query, as `encode.js` does for the
//! worker: a new build's page asks for new URLs, so the browser's HTTP
//! cache can never serve it an old `page.js`. trunk's own files are
//! named by their hash already.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::web_version;

/// The page's own files as index.html names them: a lower-case name
/// with underscores, `.js` or `.css`, in `src="…"` or `href="…"`.
fn is_page_file(name: &str) -> bool {
    let stem = name
        .strip_suffix(".js")
        .or_else(|| name.strip_suffix(".css"));
    stem.is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
}

/// `index` with `?v=<query>` after each page file; refused when it names none.
///
/// # Errors
///
/// No page file found, naming the fix.
pub fn versioned(index: &str, query: &str) -> Result<String, String> {
    let mut out = String::with_capacity(index.len());
    let mut rest = index;
    let mut count = 0;
    while let Some(at) = rest.find("=\"") {
        let (head, tail) = rest.split_at(at + 2);
        out.push_str(head);
        let end = tail.find('"').unwrap_or(tail.len());
        let value = &tail[..end];
        let attribute = head.ends_with("src=\"") || head.ends_with("href=\"");
        out.push_str(value);
        if attribute && is_page_file(value) {
            out.push_str("?v=");
            out.push_str(query);
            count += 1;
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    if count == 0 {
        return Err("index.html names no page script or style to version; fix: keep crates/web/index.html's <script src> and stylesheet links".into());
    }
    Ok(out)
}

fn staging() -> Result<PathBuf, String> {
    std::env::var_os("TRUNK_STAGING_DIR").map(PathBuf::from).ok_or_else(|| {
        "TRUNK_STAGING_DIR is unset: stamp-page is trunk's post_build hook (crates/web/Trunk.toml); fix: run `cargo xtask web` or `cargo xtask dev`".to_owned()
    })
}

fn stamp(dir: &Path) -> Result<String, String> {
    let product = crate::config::Config::load()
        .and_then(|config| config.require("web.product").map(str::to_owned))
        .map_err(|e| {
            format!(
                "{e}; fix: name the page's product, as its wasm names itself, in `[web] product`"
            )
        })?;
    let version = web_version::write(Path::new("."), dir, &product)?;
    let path = dir.join("index.html");
    let index = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let stamped = versioned(&index, &web_version::query(&version))?;
    std::fs::write(&path, stamped).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(version)
}

/// Stamp trunk's staging directory; exit 1 naming the fix.
pub fn run() -> ExitCode {
    match staging().and_then(|dir| stamp(&dir)) {
        Ok(version) => {
            println!("ok   stamp-page  {version}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("FAIL stamp-page: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
#[path = "stamp_page_tests.rs"]
mod tests;
