//! A saved reference reads as the PNG it holds, and is refused, naming
//! its file and the capture command, when it is missing, unreadable, of
//! another size, or names a state the paint tests do not have.

use std::collections::BTreeSet;
use std::path::Path;

use vapors_text::comparison::{Rgba, write_png};

use super::*;

/// A scratch directory standing in for `fixtures/references/`.
fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("vapors-references-")
        .tempdir()
        .expect("a scratch dir")
}

/// The window at `scale`, grey with one red pixel at the origin.
fn window(scale: u32) -> Rgba {
    let (width, height) = (WIDTH * scale, HEIGHT * scale);
    let mut data = [128, 128, 128, 255].repeat((width * height) as usize);
    data[..4].copy_from_slice(&[255, 0, 0, 255]);
    Rgba {
        width,
        height,
        data,
    }
}

fn base(fixture: &str, scale: u32) -> Reference {
    Reference::of(fixture, None, scale).expect("a paint fixture")
}

fn saved_into(dir: &Path, reference: &Reference, image: &Rgba) {
    write_png(&dir.join(reference.file_name()), image).expect("png");
}

#[test]
fn a_reference_reads_as_the_png_it_holds() {
    // Given
    let dir = scratch();
    let reference = base("paint_boxes", 1);
    saved_into(dir.path(), &reference, &window(1));

    // When
    let image = read(dir.path(), &reference).expect("read");

    // Then
    assert_eq!(image.data, window(1).data);
}

#[test]
fn a_missing_reference_is_refused_naming_its_file_and_the_capture_command() {
    // When
    let dir = scratch();
    let refused = read(dir.path(), &base("paint_boxes", 2)).expect_err("refused");

    // Then
    assert!(refused.contains("paint_boxes_2x.png"), "{refused}");
    assert!(refused.contains("no saved reference"), "{refused}");
    assert!(
        refused.contains("cargo xtask reference paint_boxes 2"),
        "{refused}"
    );
}

#[test]
fn an_unreadable_reference_is_refused() {
    // Given
    let dir = scratch();
    let reference = base("paint_boxes", 1);
    std::fs::write(dir.path().join(reference.file_name()), b"not a png").expect("write");

    // When
    let refused = read(dir.path(), &reference).expect_err("refused");

    // Then
    assert!(refused.contains("not a PNG"), "{refused}");
    assert!(refused.contains("cargo xtask reference"), "{refused}");
}

#[test]
fn a_reference_of_another_size_is_refused() {
    // Given: the 1x window saved as the 2x reference
    let dir = scratch();
    let reference = base("paint_boxes", 2);
    saved_into(dir.path(), &reference, &window(1));

    // When
    let refused = read(dir.path(), &reference).expect_err("refused");

    // Then
    assert!(refused.contains("640x400"), "{refused}");
    assert!(refused.contains("1280x800"), "{refused}");
}

#[test]
fn a_fixture_that_is_not_a_paint_fixture_is_refused() {
    // When: borders has no `/* paint */` in its style
    let refused = Reference::of("borders", None, 1).expect_err("refused");

    // Then
    assert!(
        refused.contains("borders is not a paint fixture"),
        "{refused}"
    );
}

#[test]
fn scale_zero_is_refused() {
    // When
    let refused = Reference::of("paint_boxes", None, 0).expect_err("refused");

    // Then
    assert!(refused.contains("scale 0"), "{refused}");
}

#[test]
fn a_state_the_paint_tests_do_not_have_is_refused() {
    // When
    let refused = Reference::of("paint_boxes", Some("card_radius_5"), 1).expect_err("refused");

    // Then
    assert!(refused.contains("card_radius_5"), "{refused}");
}

#[test]
fn a_state_of_another_fixture_is_refused() {
    // When: card_radius_4 changes paint_boxes, not icons_paint
    let refused = Reference::of("icons_paint", Some("card_radius_4"), 1).expect_err("refused");

    // Then
    assert!(refused.contains("paint_boxes"), "{refused}");
}

#[test]
fn a_states_file_and_command_name_the_state() {
    // When
    let reference = Reference::of("paint_boxes", Some("card_radius_4"), 2).expect("a state");

    // Then
    assert_eq!(reference.file_name(), "paint_boxes_card_radius_4_2x.png");
    assert_eq!(
        reference.command(),
        "cargo xtask reference paint_boxes 2 card_radius_4"
    );
}

#[test]
fn the_committed_references_are_every_one_the_paint_tests_read() {
    // Given
    let wanted: BTreeSet<String> = every_reference().iter().map(Reference::file_name).collect();

    // When
    let committed: BTreeSet<String> = std::fs::read_dir(references_dir())
        .expect("fixtures/references")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    // Then
    let missing: Vec<_> = wanted.difference(&committed).collect();
    let unread: Vec<_> = committed.difference(&wanted).collect();
    assert!(
        missing.is_empty() && unread.is_empty(),
        "fixtures/references: missing {missing:?}, read by no paint test {unread:?}; \
         fix: capture the missing with `cargo xtask reference all`, look at each, \
         commit them; remove the unread"
    );
}
