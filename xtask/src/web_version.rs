//! `version.json` beside the built page: the build it is, as the page's
//! wasm names itself (in touche, `touche_web::version::VERSION`), for
//! the page's update check. The name is `[web] product` in
//! .config/factory.toml. The stamp rule is `stamp.rs` beside this file,
//! which the page's build script includes too; in touche, `check_dist`
//! holds the file and the wasm together. Only a page whose trunk hook
//! runs `cargo xtask stamp-page` uses this; vapors' does not.

use std::path::Path;

#[path = "stamp.rs"]
#[allow(
    dead_code,
    reason = "the page's build script reads the rerun paths; this reads the text"
)]
mod stamp;

/// The file's name in the dist directory.
pub const FILE: &str = "version.json";

/// The version this checkout builds now: `<product> <version>+<stamp>`.
///
/// # Errors
///
/// The stamp's refusal outside a git checkout.
pub fn current(dir: &Path, product: &str) -> Result<String, String> {
    let stamp = stamp::stamp(dir)?;
    Ok(format!(
        "{product} {}+{}",
        env!("CARGO_PKG_VERSION"),
        stamp.text
    ))
}

/// The file's contents for `version`.
#[must_use]
pub fn json(version: &str) -> String {
    format!("{}\n", serde_json::json!({ "version": version }))
}

/// `version` as a URL query value, as the page's `encodeURIComponent`
/// writes it: unreserved characters kept, every other byte `%XX`.
#[must_use]
pub fn query(version: &str) -> String {
    version
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                char::from(b).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// Write `version.json` for the checkout at `root` into `dist`.
///
/// # Errors
///
/// The stamp's refusal, or the write's, naming the file.
pub fn write(root: &Path, dist: &Path, product: &str) -> Result<String, String> {
    let version = current(root, product)?;
    let path = dist.join(FILE);
    std::fs::write(&path, json(&version)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_one_json_object_naming_the_version() {
        // When
        let text = json("touche 0.0.0+abc1234");

        // Then
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(
            value,
            serde_json::json!({ "version": "touche 0.0.0+abc1234" })
        );
    }

    #[test]
    fn the_query_value_escapes_the_space_and_the_plus() {
        // Then
        assert_eq!(
            query("touche 0.0.0+abc1234-dirty"),
            "touche%200.0.0%2Babc1234-dirty"
        );
    }

    #[test]
    fn the_current_version_names_the_workspace_version_and_a_stamp() {
        // When
        let version = current(Path::new(env!("CARGO_MANIFEST_DIR")), "touche").expect("a checkout");

        // Then
        let rest = version.strip_prefix("touche 0.0.0+").expect(&version);
        assert!(rest.len() >= 7, "{version}");
    }
}
