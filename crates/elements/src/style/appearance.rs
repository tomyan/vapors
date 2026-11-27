//! `appearance` (CSS UI 4): `auto` or `none`, not inherited, `none`
//! initially and `auto` on the controls vapors draws parts of (the
//! user-agent sheet). With `none` a checkbox draws no mark, a switch
//! no track or knob, and an unsized checkbox has no content
//! (docs/appearance.md). On a range it is refused: Chromium keeps a
//! range's thumb, which vapors draws with its track as one part.

use vapors_css::property::longhands::APPEARANCE;
use vapors_css::value::Value;

use super::values::{Reader, keyword, unknown};

/// `appearance`'s used value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Appearance {
    /// The element draws no parts of its own.
    None,
    /// A checkbox draws its mark, a switch its track and knob, a range
    /// its track and thumb.
    Auto,
}

/// The element's `appearance`; a range with `none` is refused.
pub(crate) fn appearance(reader: &mut Reader<'_>, range: bool) -> Appearance {
    let appearance = reader.read(APPEARANCE, value, Appearance::Auto);
    if range && appearance == Appearance::None {
        reader.refuse(
            APPEARANCE,
            "is `none` on a range input, whose thumb Chromium still draws while vapors draws the track and thumb as one part; remove it, or style the range through its --vapors-* parts (docs/appearance.md; reproduce: cargo test -p vapors-elements --test tests appearance)".to_owned(),
        );
    }
    appearance
}

fn value(value: &Value) -> Result<Appearance, String> {
    match keyword(value)? {
        "auto" => Ok(Appearance::Auto),
        "none" => Ok(Appearance::None),
        other => Err(unknown(other)),
    }
}
