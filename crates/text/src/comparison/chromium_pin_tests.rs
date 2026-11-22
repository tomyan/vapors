//! The pin read from the tools file, and the order: an override, else the
//! pinned build, else a refusal naming where it should be.

use std::path::{Path, PathBuf};

use super::{TOOLS, pinned_binary, pinned_field, resolve};

#[test]
fn the_pinned_build_is_found_under_the_factory_directory_by_version() {
    // When
    let binary = pinned_binary(Path::new("/home/t"));

    // Then
    let version = pinned_field(TOOLS, "version").expect("pinned");
    assert_eq!(
        binary,
        PathBuf::from(format!(
            "/home/t/.local/share/factory/chromium-{version}/chrome-linux/chrome"
        ))
    );
}

#[test]
fn a_field_is_read_only_from_the_downloads_table() {
    // Given
    let tools =
        "[tools]\nchromium.version = \"wrong\"\n\n[downloads]\nchromium.version = \"157\"\n";

    // Then
    assert_eq!(pinned_field(tools, "version"), Some("157"));
}

#[test]
fn an_override_is_launched_whatever_is_installed() {
    let found = resolve(
        Some("/opt/chrome".into()),
        Some(Path::new("/home/t")),
        |_| false,
    );
    assert_eq!(found, Ok(PathBuf::from("/opt/chrome")));
}

#[test]
fn the_installed_pinned_build_is_launched() {
    let home = Path::new("/home/t");
    let found = resolve(None, Some(home), |path| path == pinned_binary(home));
    assert_eq!(found, Ok(pinned_binary(home)));
}

#[test]
fn a_pinned_build_not_installed_is_refused_naming_its_path() {
    let home = Path::new("/home/t");
    let refused = resolve(None, Some(home), |_| false);
    assert_eq!(refused, Err(pinned_binary(home)));
}
