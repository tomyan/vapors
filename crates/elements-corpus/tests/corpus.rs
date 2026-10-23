//! vapors-elements' layout of each fixture against Chromium's boxes in
//! `fixtures/<name>.boxes.json`. Every element's border box must be
//! within Chromium's layout unit (1/64 px), or within the tolerance a
//! fixture with text states in a `/* tolerance N */` comment, with why.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;
use vapors_elements::{BoxRect, Document, Viewport};
use vapors_elements_corpus::oracle::{fixture_names, fixtures_dir};
use vapors_elements_corpus::{Fixture, all};
use vapors_text::FontSystem;

/// Chromium lays out in 1/64 px units.
const LAYOUT_UNIT: f64 = 1.0 / 64.0;

#[allow(clippy::cast_possible_truncation)] // viewports are a few hundred pixels
fn chromium_boxes(name: &str) -> (Viewport, Vec<(String, BoxRect)>) {
    let path = fixtures_dir().join(format!("{name}.boxes.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}; run cargo run -p vapors-elements-corpus --example layout_oracle",
            path.display()
        )
    });
    let json: Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let size = |i: usize| json["viewport"][i].as_f64().expect("viewport") as f32;
    let viewport = Viewport {
        width: size(0),
        height: size(1),
        scale: 1.0,
    };
    let boxes = json["boxes"]
        .as_array()
        .expect("boxes")
        .iter()
        .map(read_box)
        .collect();
    (viewport, boxes)
}

#[allow(clippy::cast_possible_truncation)]
fn read_box(row: &Value) -> (String, BoxRect) {
    let get = |key: &str| {
        row[key]
            .as_f64()
            .unwrap_or_else(|| panic!("box {row} has no {key}")) as f32
    };
    let rect = BoxRect {
        x: get("x"),
        y: get("y"),
        width: get("width"),
        height: get("height"),
    };
    (row["tag"].as_str().expect("tag").to_owned(), rect)
}

fn our_boxes(fixture: &Fixture, viewport: Viewport) -> Vec<(String, BoxRect)> {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    doc.borrow_mut()
        .add_stylesheet(fixture.name, fixture.style)
        .expect("the fixture's style parses");
    let body = doc.borrow().body();
    (fixture.mount)(&doc, body);
    let mut doc = doc.borrow_mut();
    if let Err(errors) = doc.update(viewport) {
        panic!("{}: the update was refused: {errors:#?}", fixture.name);
    }
    let painted = doc.painted().expect("painted");
    doc.elements()
        .into_iter()
        .map(|node| {
            let tag = doc.tag(node).expect("an element").to_owned();
            let rect = painted
                .border_box(node)
                .unwrap_or_else(|| panic!("<{tag}> has no box"));
            (tag, rect)
        })
        .collect()
}

fn tolerance(style: &str) -> f64 {
    style
        .split("/* tolerance ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .map_or(LAYOUT_UNIT, |n| {
            n.parse().expect("a number after `/* tolerance`")
        })
}

fn differences(
    ours: &[(String, BoxRect)],
    theirs: &[(String, BoxRect)],
    tolerance: f64,
) -> Vec<String> {
    let mut out = Vec::new();
    for (index, ((tag, a), (their_tag, b))) in ours.iter().zip(theirs).enumerate() {
        let edges = [
            (a.x, b.x),
            (a.y, b.y),
            (a.width, b.width),
            (a.height, b.height),
        ];
        if tag != their_tag
            || edges
                .iter()
                .any(|(p, q)| f64::from(p - q).abs() > tolerance)
        {
            out.push(format!(
                "element {index}: ours <{tag}> {a:?}, Chromium's <{their_tag}> {b:?}"
            ));
        }
    }
    if ours.len() != theirs.len() {
        out.push(format!(
            "{} elements here, {} in Chromium's boxes",
            ours.len(),
            theirs.len()
        ));
    }
    out
}

#[test]
fn every_fixture_lays_out_as_chromium_does() {
    // Given
    let mut failures = Vec::new();
    for fixture in all() {
        let (viewport, theirs) = chromium_boxes(fixture.name);

        // When
        let ours = our_boxes(&fixture, viewport);

        // Then
        let found = differences(&ours, &theirs, tolerance(fixture.style));
        if !found.is_empty() {
            failures.push(format!("{}:\n  {}", fixture.name, found.join("\n  ")));
        }
    }
    assert!(
        failures.is_empty(),
        "layout differs from Chromium's:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_fixture_file_is_listed() {
    // Given
    let files = fixture_names().expect("fixtures dir");

    // When
    let listed: Vec<String> = all().iter().map(|f| f.name.to_owned()).collect();

    // Then
    assert_eq!(
        listed, files,
        "list each fixtures/*.wisp in src/lib.rs's fixtures![], sorted"
    );
}
