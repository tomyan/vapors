//! Per-character font fallback: the bundle alone, a registered font, and
//! the system's fonts where there are any.

use super::*;
use crate::{Family, FontSystem, LayoutOptions, Style};

fn system() -> FontSystem {
    FontSystem::bundle_only()
}

fn single_line(system: &mut FontSystem, text: &str, style: &Style<'_>) -> Layout {
    Layout::new(system, text, style, &LayoutOptions::default())
}

#[test]
fn a_cjk_character_falls_back_to_notdef_with_the_bundle_only() {
    // Given
    let mut system = system();
    let style = Style::new(Family::Geist, 16.0);

    // When
    let layout = single_line(&mut system, "漢", &style);
    let glyphs: Vec<_> = layout.glyphs().collect();

    // Then
    assert_eq!(glyphs.len(), 1);
    assert!(system.is_bundled(glyphs[0].font), "stays in the bundle");
    assert_eq!(glyphs[0].id, 0, ".notdef");
}

const CJK_TEST_FONT: &[u8] = include_bytes!("../fonts/test/DroidSansFallback-subset-6F22.ttf");

#[test]
fn a_cjk_character_falls_back_per_character_to_a_registered_font() {
    // Given: the bundle plus a one-glyph font registered as the Han fallback
    let mut system = system();
    assert_eq!(
        system.register_fallback(CJK_TEST_FONT.to_vec(), *b"Hani"),
        1
    );
    let style = Style::new(Family::Geist, 16.0);

    // When
    let layout = single_line(&mut system, "a漢b", &style);
    let glyphs: Vec<_> = layout.glyphs().collect();

    // Then: a and b stay in Geist, 漢 comes from the registered font
    assert_eq!(glyphs.len(), 3);
    assert!(system.is_bundled(glyphs[0].font));
    assert!(system.is_bundled(glyphs[2].font));
    assert!(!system.is_bundled(glyphs[1].font), "漢 left the bundle");
    assert_ne!(glyphs[1].id, 0, "a real glyph, not .notdef");
}

#[test]
fn bytes_that_are_not_a_font_register_no_fallback() {
    // Given
    let mut system = system();

    // When
    let families = system.register_fallback(vec![0; 32], *b"Hani");

    // Then
    assert_eq!(families, 0);
}

/// Set to `1` where a CJK system font is installed (CI installs
/// fonts-noto-cjk): the test then requires system fallback to work.
/// Unset, the test checks the two outcomes are consistent with each
/// other, since whether this machine has such a font is unknown.
#[cfg(not(target_family = "wasm"))]
const SYSTEM_FONTS_ENV: &str = "VAPORS_SYSTEM_FONTS";

#[cfg(not(target_family = "wasm"))]
#[test]
fn a_cjk_character_resolves_to_a_system_font_or_to_notdef_never_in_between() {
    // Given
    let required = std::env::var(SYSTEM_FONTS_ENV).is_ok_and(|v| v == "1");
    let mut system = FontSystem::new();
    let style = Style::new(Family::Geist, 16.0);

    // When
    let layout = single_line(&mut system, "漢", &style);
    let glyph = layout.glyphs().next().expect("one glyph");
    let bundled = system.is_bundled(glyph.font);

    // Then
    assert_eq!(
        bundled,
        glyph.id == 0,
        "either a system glyph or the bundle's .notdef"
    );
    if required {
        assert!(
            !bundled,
            "{SYSTEM_FONTS_ENV}=1 but no system font covers U+6F22. \
             Fix: install a CJK font (apt-get install fonts-noto-cjk) or unset {SYSTEM_FONTS_ENV}"
        );
    }
}
