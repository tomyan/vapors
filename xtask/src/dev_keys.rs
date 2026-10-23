//! `cargo xtask dev --key <provider>=<file>`: keys the debug page may
//! use, read from files the person keeps. Each file is checked before
//! trunk starts and read again on every request; its key is never
//! copied, cached or printed.

use std::path::PathBuf;

/// The providers that take a key, from the page's words file (`[dev]
/// words` in .config/factory.toml): each `pub const PROVIDER_<NAME>: &str
/// = "<id>";` that the file's key source maps with
/// `PROVIDER_<NAME> => Ok(&`. touche's test holds its file to this shape.
///
/// # Errors
///
/// A file in which no provider takes a key, naming `path`.
pub fn keyed_providers(words: &str, path: &str) -> Result<Vec<String>, String> {
    let keyed: Vec<String> = words
        .lines()
        .filter_map(provider_constant)
        .filter(|(name, _)| words.contains(&format!("{name} => Ok(&")))
        .map(|(_, id)| id)
        .collect();
    if keyed.is_empty() {
        return Err(format!(
            "{path}: no provider takes a key (no `pub const PROVIDER_<NAME>: &str = \"<id>\";` mapped by `PROVIDER_<NAME> => Ok(&`); fix: point `[dev] words` in .config/factory.toml at the page's words file"
        ));
    }
    Ok(keyed)
}

/// `PROVIDER_<NAME>` and its id from `pub const PROVIDER_<NAME>: &str = "<id>";`.
fn provider_constant(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix("pub const ")?;
    let (name, rest) = rest.split_once(": &str = \"")?;
    let id = rest.strip_suffix("\";")?;
    name.starts_with("PROVIDER_")
        .then(|| (name.to_owned(), id.to_owned()))
}

/// One `--key` argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevKey {
    /// One of the [`keyed_providers`].
    pub provider: String,
    /// The file holding its key.
    pub file: PathBuf,
}

/// The fix for a malformed `--key`; no message repeats the argument,
/// which may be the key itself typed in the file's place.
fn expected(providers: &[String]) -> String {
    format!(
        "fix: --key <provider>=<file>, with provider one of {} and file a path to a file holding the key, not the key",
        providers.join(", ")
    )
}

/// Parse `<provider>=<file>`, the provider one of `providers`.
///
/// # Errors
///
/// No `=`, an empty file, or a provider that takes no key.
pub fn parse(argument: &str, providers: &[String]) -> Result<DevKey, String> {
    let Some((provider, file)) = argument.split_once('=').filter(|(_, f)| !f.is_empty()) else {
        return Err(format!(
            "a --key argument is not <provider>=<file> (it is not repeated here: it may hold a key); {}",
            expected(providers)
        ));
    };
    if !providers.iter().any(|keyed| keyed == provider) {
        return Err(format!(
            "a --key argument names a provider that takes no key (not repeated here); {}",
            expected(providers)
        ));
    }
    Ok(DevKey {
        provider: provider.to_owned(),
        file: PathBuf::from(file),
    })
}

/// The key in `key`'s file, its surrounding whitespace removed. The
/// same check runs before trunk starts and on every request.
///
/// # Errors
///
/// The file readable by others, unreadable, or empty, naming the file
/// and the fix; never the key.
pub fn read(key: &DevKey) -> Result<String, String> {
    let file = key.file.display();
    check_mode(key)?;
    let text = std::fs::read_to_string(&key.file)
        .map_err(|e| format!("{file} ({} key) cannot be read: {e}", key.provider))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{file} ({} key) is empty; fix: put the key in it",
            key.provider
        ));
    }
    Ok(trimmed.to_owned())
}

/// A file that does not exist is not named: what was typed may be the
/// key. One that exists is named.
fn missing_or_unreadable(key: &DevKey, error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        return format!(
            "the file given for {} in --key does not exist (not repeated here: it may be the key); fix: --key <provider>=<file>, with file a path to a file holding the key, not the key",
            key.provider
        );
    }
    format!(
        "{} ({} key) cannot be read: {error}",
        key.file.display(),
        key.provider
    )
}

/// Refuse a key file that anyone but its owner may read or write.
fn check_mode(key: &DevKey) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let file = key.file.display();
    let mode = std::fs::metadata(&key.file)
        .map_err(|e| missing_or_unreadable(key, &e))?
        .permissions()
        .mode();
    if mode & 0o077 != 0 {
        return Err(format!(
            "{file} ({} key) has mode {:o}, which lets others read it; fix: chmod 600 {file}",
            key.provider,
            mode & 0o777
        ));
    }
    Ok(())
}

/// The line printed for `key`: its provider and file, never the key.
pub fn notice(key: &DevKey) -> String {
    format!(
        "     dev key  {} from {} (read on each request; served to the debug page only)",
        key.provider,
        key.file.display()
    )
}

#[cfg(test)]
#[path = "dev_keys_tests.rs"]
mod tests;
