//! A byte range's fragments: one per line it is on, with its x extent.

use crate::{Family, FontSystem, Layout, LayoutOptions, Style};

fn laid_out(text: &str, width: Option<f32>) -> Layout {
    let options = LayoutOptions {
        width,
        ..LayoutOptions::default()
    };
    Layout::new(
        &mut FontSystem::bundle_only(),
        text,
        &Style::new(Family::Geist, 16.0),
        &options,
    )
}

#[test]
fn a_range_on_one_line_is_one_fragment_from_its_first_cluster_to_its_end() {
    // Given
    let layout = laid_out("Hello world", None);

    // When
    let fragments = layout.fragments(6..11);

    // Then
    let [fragment] = fragments.as_slice() else {
        panic!("one fragment, got {fragments:?}");
    };
    assert_eq!(fragment.line, 0);
    assert!(
        (fragment.x - layout.cluster_x(6)).abs() < 0.01,
        "{fragment:?}"
    );
    assert!(
        (fragment.x + fragment.width - layout.width()).abs() < 0.01,
        "{fragment:?} ends where the line does ({})",
        layout.width()
    );
}

#[test]
fn a_range_across_a_line_break_has_a_fragment_on_each_line() {
    // Given: two words that do not fit on one 60 px line
    let layout = laid_out("Hello world", Some(60.0));

    // When
    let fragments = layout.fragments(0..11);

    // Then
    let lines: Vec<usize> = fragments.iter().map(|f| f.line).collect();
    assert_eq!(lines, [0, 1], "{fragments:?}");
    assert!(fragments.iter().all(|f| f.width > 0.0), "{fragments:?}");
}

#[test]
fn an_empty_range_has_no_fragments() {
    // Given
    let layout = laid_out("Hello", None);

    // When
    let fragments = layout.fragments(2..2);

    // Then
    assert_eq!(fragments, Vec::new());
}

#[test]
fn the_space_a_line_breaks_after_is_not_in_a_fragment() {
    // Given: "Hello world" broken after "Hello "
    let layout = laid_out("Hello world", Some(60.0));
    let hello = laid_out("Hello", None);

    // When
    let fragments = layout.fragments(0..11);

    // Then: the first line's fragment is as wide as "Hello" alone
    assert!(
        (fragments[0].width - hello.width()).abs() < 0.01,
        "{fragments:?} against {}",
        hello.width()
    );
}
