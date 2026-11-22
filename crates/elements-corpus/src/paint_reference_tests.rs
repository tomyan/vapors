//! A saved reference is read only when its manifest holds what the tree
//! holds now and its PNG is the one the manifest names.

use std::path::Path;

use serde_json::{Map, Value, json};
use vapors_text::comparison::{Rgba, write_png};

use super::*;

/// A 2 x 1 reference at scale 1, its fields as `expected` gives them.
fn fields() -> Map<String, Value> {
    let fields = json!({
        "fixture": "tiny", "scale": 1, "width": 2, "height": 1,
        "page_sha256": "aa", "fonts": { "Geist-wght.ttf": "bb" },
        "images": {}, "browser": { "version": "157", "binary_sha256": "cc" },
        "flags": ["--headless=new"],
    });
    let Value::Object(fields) = fields else {
        unreachable!("an object")
    };
    fields
}

fn image() -> Rgba {
    Rgba {
        width: 2,
        height: 1,
        data: vec![10, 20, 30, 255, 40, 50, 60, 255],
    }
}

/// Writes `image` and a manifest of `fields` with its hash into `dir`,
/// as a capture does.
fn saved_into(dir: &Path, fields: &Map<String, Value>, image: &Rgba) {
    let png = dir.join("tiny_1x.png");
    write_png(&png, image).expect("png");
    let mut manifest = fields.clone();
    let bytes = std::fs::read(&png).expect("read");
    manifest.insert("png_sha256".into(), json!(sha256(&bytes).expect("hash")));
    manifest.insert("host".into(), json!("a test"));
    let text = serde_json::to_string(&Value::Object(manifest)).expect("json");
    std::fs::write(dir.join("tiny_1x.json"), text).expect("manifest");
}

fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("vapors-reference-")
        .tempdir()
        .expect("a scratch dir")
}

#[test]
fn a_reference_whose_manifest_holds_the_trees_fields_is_read() {
    // Given
    let dir = scratch();
    saved_into(dir.path(), &fields(), &image());

    // When
    let read = check(dir.path(), "tiny", 1, &fields()).expect("read");

    // Then
    assert_eq!((read.width, read.height, read.data), (2, 1, image().data));
}

#[test]
fn a_missing_reference_is_refused_naming_the_file_and_the_command() {
    // Given
    let dir = scratch();

    // When
    let refused = check(dir.path(), "tiny", 1, &fields()).expect_err("refused");

    // Then
    assert!(
        refused.contains("tiny_1x.json: no saved reference"),
        "{refused}"
    );
    assert!(
        refused.contains("cargo xtask reference tiny 1"),
        "{refused}"
    );
}

#[test]
fn a_missing_png_is_refused_naming_it() {
    // Given
    let dir = scratch();
    saved_into(dir.path(), &fields(), &image());
    std::fs::remove_file(dir.path().join("tiny_1x.png")).expect("removed");

    // When
    let refused = check(dir.path(), "tiny", 1, &fields()).expect_err("refused");

    // Then
    assert!(refused.contains("tiny_1x.png: no saved PNG"), "{refused}");
}

#[test]
fn a_field_that_changed_since_the_capture_is_refused_naming_it_and_both_values() {
    for field in [
        "fixture",
        "scale",
        "width",
        "height",
        "page_sha256",
        "fonts",
        "images",
        "browser",
        "flags",
    ] {
        // Given: the tree now holds another value
        let dir = scratch();
        saved_into(dir.path(), &fields(), &image());
        let mut now = fields();
        now.insert(field.into(), json!("changed"));

        // When
        let refused = check(dir.path(), "tiny", 1, &now).expect_err(field);

        // Then
        assert!(
            refused.contains(&format!("stale: `{field}` is ")),
            "{field}: {refused}"
        );
        assert!(refused.contains("now \"changed\""), "{field}: {refused}");
        assert!(
            refused.contains("cargo xtask reference tiny 1"),
            "{field}: {refused}"
        );
    }
}

#[test]
fn a_png_that_is_not_the_one_the_manifest_names_is_refused() {
    // Given: the PNG replaced after its manifest was written
    let dir = scratch();
    saved_into(dir.path(), &fields(), &image());
    let other = Rgba {
        data: vec![0; 8],
        ..image()
    };
    write_png(&dir.path().join("tiny_1x.png"), &other).expect("png");

    // When
    let refused = check(dir.path(), "tiny", 1, &fields()).expect_err("refused");

    // Then
    assert!(refused.contains("tiny_1x.png: its sha256 is "), "{refused}");
}

#[test]
fn a_manifest_field_it_does_not_recognise_is_refused() {
    // Given: the manifest holds a field the tree does not
    let dir = scratch();
    let mut saved = fields();
    saved.insert("gamma".into(), json!(2.2));
    saved_into(dir.path(), &saved, &image());

    // When
    let refused = check(dir.path(), "tiny", 1, &fields()).expect_err("refused");

    // Then
    assert!(
        refused.contains("`gamma` is not a manifest field"),
        "{refused}"
    );
}

#[test]
fn a_png_of_another_size_than_the_manifests_is_refused() {
    // Given: a manifest of 2 x 1 at scale 1 over a 1 x 1 PNG
    let dir = scratch();
    let small = Rgba {
        width: 1,
        height: 1,
        data: vec![1, 2, 3, 255],
    };
    saved_into(dir.path(), &fields(), &small);

    // When
    let refused = check(dir.path(), "tiny", 1, &fields()).expect_err("refused");

    // Then
    assert!(
        refused.contains("it is 1x1, the manifest's 2x1"),
        "{refused}"
    );
}

#[test]
fn a_manifest_that_lacks_a_field_the_tree_holds_is_refused_naming_it() {
    // Given: a manifest captured before `flags` was recorded
    let dir = scratch();
    let mut saved = fields();
    saved.remove("flags");
    saved_into(dir.path(), &saved, &image());

    // When
    let refused = check(dir.path(), "tiny", 1, &fields()).expect_err("refused");

    // Then
    assert!(
        refused.contains("stale: no `flags` (now [\"--headless=new\"])"),
        "{refused}"
    );
}

#[test]
fn a_manifest_without_its_capture_host_is_refused() {
    // Given
    let dir = scratch();
    saved_into(dir.path(), &fields(), &image());
    let path = dir.path().join("tiny_1x.json");
    let text = std::fs::read_to_string(&path).expect("manifest");
    let mut manifest: Map<String, Value> = serde_json::from_str(&text).expect("json");
    manifest.remove("host");
    std::fs::write(&path, serde_json::to_string(&manifest).expect("json")).expect("write");

    // When
    let refused = check(dir.path(), "tiny", 1, &fields()).expect_err("refused");

    // Then
    assert!(refused.contains("no `host`"), "{refused}");
}
