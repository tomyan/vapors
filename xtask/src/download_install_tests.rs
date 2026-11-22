//! Against archives built here and fetched over `file://`: the binary in
//! them is a script that writes a marker file if it is ever run.

use std::os::unix::fs::PermissionsExt as _;

use super::*;

struct Fixture {
    _root: crate::scratch::Scratch,
    home: PathBuf,
    marker: PathBuf,
    download: Download,
}

/// An archive holding `tool/tool`, a script that touches the marker.
fn fixture(name: &str) -> Fixture {
    let root = crate::scratch::dir(&format!("download-{name}"));
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
        archive: crate::downloads::Archive::TarGz,
        tree: None,
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

/// A zip holding `tree/tool` (the marker script) and `tree/data` beside it.
fn zip_fixture(name: &str) -> Fixture {
    let mut fixture = fixture(name);
    let src = fixture.home.parent().expect("root").join("src");
    std::fs::create_dir_all(src.join("tree")).expect("dir");
    std::fs::copy(src.join("tool/tool"), src.join("tree/tool")).expect("copy");
    std::fs::write(src.join("tree/data"), "beside").expect("data");
    let archive = src.join("tree.zip");
    let script = "import sys, zipfile, os\nwith zipfile.ZipFile(sys.argv[1], 'w') as z:\n    for f in ('tree/tool', 'tree/data'):\n        z.write(os.path.join(sys.argv[2], f), f)\n";
    run(Command::new("python3")
        .args(["-I", "-c", script])
        .arg(&archive)
        .arg(&src))
    .expect("zip");
    fixture.download.archive = crate::downloads::Archive::Zip;
    fixture.download.tree = Some("tree".into());
    fixture.download.member = "tree/tool".into();
    fixture.download.url = format!("file://{}", archive.display());
    fixture.download.sha256 = sha256(&archive).expect("hash");
    fixture
}

#[test]
fn a_zip_tree_is_installed_whole_after_both_hashes_match_and_never_run() {
    // Given
    let fixture = zip_fixture("zip-tree");

    // When
    let path = ensure(&fixture.download, &fixture.home).expect("installed");

    // Then: the binary verified and executable, its neighbour beside it
    assert_eq!(path, installed_path(&fixture.download, &fixture.home));
    assert!(path.metadata().expect("installed").permissions().mode() & 0o111 != 0);
    assert_eq!(
        std::fs::read_to_string(path.with_file_name("data")).expect("data"),
        "beside"
    );
    assert!(!fixture.marker.exists(), "the binary was run");
}
