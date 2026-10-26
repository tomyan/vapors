//! What `serve_verify.rs` requires of each answer, without the network:
//! reading curl's headers, the expected type and cache for a path, and
//! the judgement.

/// What a response's headers said.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub encoding: Option<String>,
    pub cache: Option<String>,
    pub content_type: Option<String>,
    pub vary: Option<String>,
}

/// What a file must come with.
#[derive(Debug, PartialEq, Eq)]
pub struct Expect {
    pub content_type: &'static str,
    pub cache: &'static str,
}

pub const IMMUTABLE: &str = "public, max-age=31536000, immutable";
pub const NO_CACHE: &str = "no-cache";

/// The status line and the four headers, from `curl -D -`'s output.
#[must_use]
pub fn answer(headers: &str) -> Option<Answer> {
    let mut lines = headers.lines();
    let status = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let mut answer = Answer {
        status,
        ..Answer::default()
    };
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = Some(value.trim().to_owned());
        match name.trim().to_ascii_lowercase().as_str() {
            "content-encoding" => answer.encoding = value,
            "cache-control" => answer.cache = value,
            "content-type" => answer.content_type = value,
            "vary" => answer.vary = value,
            _ => {}
        }
    }
    Some(answer)
}

/// The type and cache a file at `path` is served with.
#[must_use]
pub fn expect_for(path: &str) -> Expect {
    let content_type = match path.rsplit('.').next() {
        Some("wasm") => "application/wasm",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        _ => "text/html",
    };
    let cache = if hashed(path) { IMMUTABLE } else { NO_CACHE };
    Expect {
        content_type,
        cache,
    }
}

/// trunk's hashed names, as `serve_unit.rs`'s glob: `<name>-<16 hex>` then
/// `_bg.wasm`, `.js` or `.css`.
fn hashed(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let Some(stem) = ["_bg.wasm", ".js", ".css"]
        .iter()
        .find_map(|end| name.strip_suffix(end))
    else {
        return false;
    };
    stem.rsplit_once('-').is_some_and(|(_, hash)| {
        hash.len() == 16 && hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    })
}

/// `Ok` when the answer is 200, compressed, typed, varied and cached as
/// `expect` says.
///
/// # Errors
///
/// What differed, naming the URL and the fix.
pub fn judge(url: &str, got: &Answer, expect: &Expect) -> Result<(), String> {
    if got.status != 200 {
        return Err(format!(
            "{url} answered {}; fix: journalctl --user -u xtask-serve-*",
            got.status
        ));
    }
    if !matches!(got.encoding.as_deref(), Some("br" | "gzip")) {
        return Err(format!(
            "{url} came with Content-Encoding {:?}, not br or gzip; fix: check the unit runs with --compression-static and the .br copies exist (cargo xtask web)",
            got.encoding
        ));
    }
    let typed = got
        .content_type
        .as_deref()
        .and_then(|t| t.split(';').next())
        .map(str::trim);
    if !typed.is_some_and(|t| t.eq_ignore_ascii_case(expect.content_type)) {
        return Err(format!(
            "{url} came with Content-Type {:?}, not {}; fix: the server's types (serve_unit.rs)",
            got.content_type, expect.content_type
        ));
    }
    if !got
        .vary
        .as_deref()
        .is_some_and(|v| v.to_ascii_lowercase().contains("accept-encoding"))
    {
        return Err(format!(
            "{url} came with Vary {:?}, without Accept-Encoding; fix: the server's config (serve_unit.rs)",
            got.vary
        ));
    }
    if got.cache.as_deref() != Some(expect.cache) {
        return Err(format!(
            "{url} came with Cache-Control {:?}, not {:?}; fix: the server's config (serve_unit.rs)",
            got.cache, expect.cache
        ));
    }
    Ok(())
}

/// The first `<script ... src="...">` in `index` with a relative URL.
#[must_use]
pub fn first_script(index: &str) -> Option<String> {
    index.split("<script").skip(1).find_map(|tag| {
        let tag = tag.split('>').next()?;
        let src = tag.split("src=\"").nth(1)?.split('"').next()?;
        let relative = !src.is_empty()
            && !src.starts_with('/')
            && !src.contains("://")
            && !src.starts_with("data:");
        relative.then(|| src.trim_start_matches("./").to_owned())
    })
}

#[cfg(test)]
#[path = "serve_verify_rules_tests.rs"]
mod tests;
