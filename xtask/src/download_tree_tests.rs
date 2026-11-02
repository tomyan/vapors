//! A `.zip` download: the whole tree installed in its own directory,
//! against an archive built here and fetched over `file://`. Its binary
//! is a script that writes a marker file if it is ever run.

use std::path::PathBuf;
use std::process::Command;

use super::super::{ensure, sha256, verified};
use crate::downloads::{Download, installed_path};

struct Fixture {
    _root: crate::scratch::Scratch,
    home: PathBuf,
    marker: PathBuf,
    download: Download,
}

/// A zip holding `tool/tool` (a script that touches the marker) and
/// `tool/data`, a file the tool needs beside it.
fn fixture(name: &str) -> Fixture {
    let root = crate::scratch::dir(&format!("download-tree-{name}"));
    let marker = root.join("ran");
    std::fs::create_dir_all(root.join("src/tool")).expect("dir");
    let script = root.join("src/tool/tool");
    std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display())).expect("script");
    std::fs::write(root.join("src/tool/data"), "beside the tool\n").expect("data");
    let archive = root.join("tool.zip");
    // python3's zipfile, which CI's runner and the dev box both have
    // (no `zip` binary on the dev box).
    let zipped = Command::new("python3")
        .args([
            "-c",
            "import shutil, sys; shutil.make_archive(sys.argv[1], 'zip', sys.argv[2], 'tool')",
        ])
        .arg(archive.with_extension(""))
        .arg(root.join("src"))
        .status()
        .expect("python3 runs");
    assert!(zipped.success(), "zip failed");
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
        _root: root,
        marker,
        download,
    }
}

#[test]
fn a_matching_zip_is_installed_whole_and_never_run() {
    // Given
    let fixture = fixture("good");

    // When
    let path = ensure(&fixture.download, &fixture.home).expect("installed");

    // Then
    assert_eq!(path, installed_path(&fixture.download, &fixture.home));
    assert!(
        path.with_file_name("data").is_file(),
        "the tree's other files"
    );
    assert_eq!(verified(&fixture.download, &fixture.home), Ok(path));
    assert!(!fixture.marker.exists(), "the binary was run");
}

#[test]
fn a_zip_whose_binary_has_another_hash_installs_nothing() {
    // Given
    let mut fixture = fixture("binary");
    fixture.download.binary_sha256 = "1".repeat(64);

    // When
    let refusal = ensure(&fixture.download, &fixture.home).expect_err("refused");

    // Then
    assert!(refusal.starts_with("tool/tool has SHA-256 "), "{refusal}");
    let factory = fixture.home.join(".local/share/factory");
    let left: Vec<_> = std::fs::read_dir(&factory)
        .map(|d| d.filter_map(Result::ok).map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "{left:?}");
    assert!(!fixture.marker.exists(), "the binary was run");
}
