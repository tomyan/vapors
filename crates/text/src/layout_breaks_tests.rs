//! Line breaks in Thai, which writes words without spaces: with the
//! `complex-script-breaks` feature (default) a narrow line breaks between
//! words, found by ICU4X's dictionary; without it Thai text has no break
//! opportunity but its spaces, so it stays one line however narrow.

use super::*;
use crate::{Family, FontSystem, LayoutOptions, Style};

/// "Hello, welcome" in Thai: three words and no space.
const THAI: &str = "สวัสดีครับยินดีต้อนรับ";

fn lines_at(width: f32) -> usize {
    let mut system = FontSystem::bundle_only();
    let options = LayoutOptions {
        width: Some(width),
        ..LayoutOptions::default()
    };
    Layout::new(
        &mut system,
        THAI,
        &Style::new(Family::Geist, 16.0),
        &options,
    )
    .lines()
    .count()
}

#[cfg(feature = "complex-script-breaks")]
#[test]
fn with_complex_script_breaks_narrow_thai_breaks_between_words() {
    assert!(lines_at(40.0) > 1, "{} lines", lines_at(40.0));
}

/// Runs only under `cargo test -p vapors-text --no-default-features`:
/// ci builds the default only, so this is checked by hand when the
/// feature changes, until a ci step runs it (tomyan/vapors#121).
#[cfg(not(feature = "complex-script-breaks"))]
#[test]
fn without_complex_script_breaks_thai_stays_one_line() {
    assert_eq!(lines_at(40.0), 1);
}
