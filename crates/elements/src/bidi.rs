//! Text is laid out left to right only (F2c S6 review). A character
//! from a script written right to left, or a right-to-left mark, is
//! refused rather than laid out in the wrong order. The test is by
//! Unicode block: the blocks whose letters are strong right to left,
//! so a few neutral characters inside them are refused too.

/// Whether `c` is in a right-to-left block, or is a right-to-left mark
/// or embedding.
pub(crate) fn right_to_left(c: char) -> bool {
    matches!(
        u32::from(c),
        0x0590..=0x08FF        // Hebrew, Arabic, Syriac, Thaana, NKo, Samaritan, Mandaic, Arabic Extended
            | 0xFB1D..=0xFDFF  // Hebrew and Arabic presentation forms
            | 0xFE70..=0xFEFF  // Arabic presentation forms B
            | 0x10800..=0x10FFF // right-to-left scripts of the supplementary plane
            | 0x1E800..=0x1EFFF // Mende Kikakui, Adlam, Arabic mathematical symbols
            | 0x200F | 0x202B | 0x202E | 0x2067 // RLM, RLE, RLO, RLI
    )
}

#[cfg(test)]
mod tests {
    use super::right_to_left;

    #[test]
    fn hebrew_arabic_and_the_rtl_mark_are_right_to_left_and_latin_is_not() {
        // Then
        assert!(right_to_left('ש') && right_to_left('ب') && right_to_left('\u{200F}'));
        assert!(!right_to_left('a') && !right_to_left('—') && !right_to_left('é'));
    }
}
