//! One `key = "value"` line of the settings file.

use crate::config::ConfigError;

pub fn key_value(line: &str, number: usize) -> Result<(String, String), ConfigError> {
    let refuse = |reason: &str| ConfigError {
        line: number,
        reason: format!("{reason}: `{line}`"),
    };
    let (key, rest) = split_key(line).ok_or_else(|| refuse("expected `key = \"value\"`"))?;
    let value = rest
        .trim()
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| refuse("value must be a double-quoted string"))?;
    let key = unquoted_key(key.trim()).ok_or_else(|| {
        refuse("key must be bare (no quotes or spaces) or a double-quoted string without escapes")
    })?;
    if key.is_empty() || value.is_empty() {
        return Err(refuse("key and value must both be non-empty"));
    }
    Ok((key, value.to_owned()))
}

/// `key` and the rest after its `=`: a quoted key ends at its closing
/// quote, so it may hold `=` (a gate setup line such as `:lang=zh`).
fn split_key(line: &str) -> Option<(&str, &str)> {
    let quote = line.chars().next().filter(|c| *c == '"' || *c == '\'');
    let quoted = quote.and_then(|quote| {
        let end = line[1..].find(quote)? + 2;
        let rest = line[end..].trim_start().strip_prefix('=')?;
        Some((&line[..end], rest))
    });
    // Not a quoted key followed by `=`: split at the first `=`, so a
    // malformed key is refused by name below.
    quoted.or_else(|| line.split_once('='))
}

/// A bare key as is; a double-quoted key without its quotes; a
/// single-quoted key (TOML's literal string, which may hold double
/// quotes) as written between them; `None` for anything else (a quote
/// inside, a backslash in a double-quoted key, a space in a bare key).
fn unquoted_key(key: &str) -> Option<String> {
    if let Some(inner) = key.strip_prefix('\'') {
        let inner = inner.strip_suffix('\'')?;
        return (!inner.contains('\'')).then(|| inner.to_owned());
    }
    if let Some(inner) = key.strip_prefix('"') {
        let inner = inner.strip_suffix('"')?;
        return (!inner.contains(['"', '\\'])).then(|| inner.to_owned());
    }
    (!key.contains(['"', ' ', '\t'])).then(|| key.to_owned())
}
