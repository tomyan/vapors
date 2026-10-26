//! Against archives built here and fetched over `file://`: the binary in
//! them is a script that writes a marker file if it is ever run.

use std::os::unix::fs::PermissionsExt as _;

use super::*;

struct Fixture {
    home: PathBuf,
    marker: PathBuf,
    download: Download,
}

/// An archive holding `tool/tool`, a script that touches the marker.
fn fixture(name: &str) -> Fixture {
    let root = std::env::temp_dir().join(format!("xtask-download-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let marker = root.join("ran");
    std::fs::create_dir_all(root.join("src/tool")).expect("dir");
    let script = root.join("src/tool/tool");
    std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display())).expect("script");
    let archive = root.join("tool.tar.gz");
    run(Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(root.join("src"))
        .arg("tool/tool"))
    .expect("tar");
    let download = Download {
        name: "tool".into(),
        version: "1.0.0".into(),
        url: format!("file://{}", archive.display()),
        sha256: sha256(&archive).expect("hash"),
        member: "tool/tool".into(),
        binary_sha256: sha256(&script).expect("hash"),
    };
    Fixture {
        home: root.join("home"),
        marker,
        download,
    }
}

fn leftovers(fixture: &Fixture) -> Vec<String> {
    let bin = installed_path(&fixture.download, &fixture.home)
        .parent()
        .expect("dir")
        .to_path_buf();
    std::fs::read_dir(bin)
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_matching_archive_is_installed_executable_and_never_run() {
    // Given
    let fixture = fixture("good");

    // When
    let path = ensure(&fixture.download, &fixture.home).expect("installed");

    // Then
    assert_eq!(path, installed_path(&fixture.download, &fixture.home));
    assert_eq!(
        std::fs::metadata(&path).expect("meta").permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(verified(&fixture.download, &fixture.home), Ok(path));
    assert!(!fixture.marker.exists(), "the binary was run");
    assert_eq!(leftovers(&fixture), vec!["tool-1.0.0".to_owned()]);
}

#[test]
fn an_archive_with_another_hash_is_deleted_naming_both_and_never_run() {
    // Given
    let mut fixture = fixture("archive");
    fixture.download.sha256 = "0".repeat(64);

    // When
    let refusal = ensure(&fixture.download, &fixture.home).expect_err("refused");

    // Then
    assert!(
        refusal.contains(&format!("not the pinned {}", "0".repeat(64)))
            && refusal.contains("has SHA-256 "),
        "{refusal}"
    );
    assert!(refusal.contains("nothing installed"), "{refusal}");
    assert!(leftovers(&fixture).is_empty(), "{:?}", leftovers(&fixture));
    assert!(!fixture.marker.exists(), "the binary was run");
}

#[test]
fn a_binary_with_another_hash_is_deleted_and_never_run() {
    let mut fixture = fixture("binary");
    fixture.download.binary_sha256 = "1".repeat(64);
    let refusal = ensure(&fixture.download, &fixture.home).expect_err("refused");
    assert!(refusal.starts_with("tool/tool has SHA-256 "), "{refusal}");
    assert!(leftovers(&fixture).is_empty(), "{:?}", leftovers(&fixture));
    assert!(!fixture.marker.exists(), "the binary was run");
}

#[test]
fn a_changed_installed_binary_is_refused_and_replaced_by_ensure() {
    // Given: installed, then overwritten
    let fixture = fixture("changed");
    let path = ensure(&fixture.download, &fixture.home).expect("installed");
    std::fs::write(&path, "#!/bin/sh\n").expect("overwrite");

    // When / Then
    assert!(
        verified(&fixture.download, &fixture.home)
            .expect_err("changed")
            .contains("not the pinned")
    );
    assert_eq!(ensure(&fixture.download, &fixture.home), Ok(path));
    assert!(!fixture.marker.exists(), "the binary was run");
}

#[test]
fn a_tool_not_installed_names_setup() {
    let fixture = fixture("absent");
    let refusal = verified(&fixture.download, &fixture.home).expect_err("absent");
    assert!(
        refusal.contains("is not installed; fix: cargo xtask setup"),
        "{refusal}"
    );
}
