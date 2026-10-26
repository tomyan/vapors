//! The text of static-web-server's config and its user unit. The unit
//! binds 127.0.0.1 only and runs sandboxed: no new privileges, the system
//! read-only, the home directory an empty tmpfs into which exactly three
//! paths are bound read-only: the current copy (at the served path), the
//! config and the verified binary. `cargo xtask serve` restarts the unit
//! after every publish, so the bound copy is always the one just
//! published. `ProtectHome=tmpfs` with those binds, not
//! `ProtectHome=read-only`: read-only would still let the server read
//! the whole home directory (keys, other checkouts); the lead chose this
//! over read-only on 2026-10-26.

use std::path::Path;

/// The paths written into the unit, refused when they hold a character
/// systemd's unit syntax or `BindReadOnlyPaths=` would read specially.
pub struct Paths<'a> {
    pub server: &'a Path,
    pub dir: &'a Path,
    pub copy: &'a Path,
    pub config: &'a Path,
}

/// static-web-server's settings: precompressed copies by `Accept-Encoding`,
/// no directory listings, and cache headers. The later entry wins, so the
/// immutable rule for trunk's hashed names comes after the catch-all.
#[must_use]
pub fn config_text() -> String {
    let hashed = format!("**/*-{}{{_bg.wasm,.js,.css}}", "[0-9a-f]".repeat(16));
    format!(
        "[general]\ncache-control-headers = false\ncompression = true\ncompression-static = true\ndirectory-listing = false\n\n[advanced]\n\n[[advanced.headers]]\nsource = \"**/*\"\n[advanced.headers.headers]\nCache-Control = \"no-cache\"\n\n[[advanced.headers]]\nsource = \"{hashed}\"\n[advanced.headers.headers]\nCache-Control = \"public, max-age=31536000, immutable\"\n"
    )
}

/// The unit for `paths` on `port`.
///
/// # Errors
///
/// A path with whitespace, `"`, `\`, `%` or `:`, naming it and the fix.
pub fn unit_text(paths: &Paths<'_>, port: u16) -> Result<String, String> {
    let [server, dir, copy, config] =
        [paths.server, paths.dir, paths.copy, paths.config].map(plain);
    let (server, dir, copy, config) = (server?, dir?, copy?, config?);
    Ok(format!(
        "[Unit]\nDescription=cargo xtask serve: {dir} on 127.0.0.1:{port}\n\n[Service]\n# The home directory is an empty tmpfs (ProtectHome=tmpfs): only the\n# copy, its config and this binary are bound in, read-only.\nExecStart=\"{server}\" --host 127.0.0.1 --port {port} --root \"{dir}\" --config-file \"{config}\"\nRestart=on-failure\nNoNewPrivileges=yes\nProtectSystem=strict\nProtectHome=tmpfs\nPrivateTmp=yes\nBindReadOnlyPaths={copy}:{dir} {config} {server}\n\n[Install]\nWantedBy=default.target\n"
    ))
}

/// The unit's name for `dir`: `xtask-serve-<dir's name>.service`.
#[must_use]
pub fn unit_name(dir: &Path) -> String {
    let name = dir
        .file_name()
        .map_or_else(|| "page".into(), |n| n.to_string_lossy().into_owned());
    format!("xtask-serve-{name}.service")
}

fn plain(path: &Path) -> Result<String, String> {
    let text = path.to_str().ok_or_else(|| {
        format!(
            "{} is not UTF-8; fix: serve from a plain path",
            path.display()
        )
    })?;
    if text
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\\' | '%' | ':'))
    {
        return Err(format!(
            "{text} holds a space, quote, backslash, % or colon, which the unit cannot carry; fix: serve from a path without them"
        ));
    }
    Ok(text.to_owned())
}

#[cfg(test)]
#[path = "serve_unit_tests.rs"]
mod tests;
