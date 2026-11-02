//! The edits the native IME makes for winit's documented sequences: one
//! answer, read by two tests. The platform's checks that `Composition`
//! makes exactly these; the elements' checks that the field takes them
//! and one Control+Z goes back to the text before the composition, as
//! on the web (review-v90's B1).

// each test binary reads a part
#![allow(dead_code)]

/// An edit: the text it was made against, the text after, the
/// selection and the composing range, in UTF-16 units.
pub type Edit = (&'static str, &'static str, (u32, u32), Option<(u32, u32)>);

/// winit's pinyin example after "Warm" (caret at 4): preedit "a", then
/// "a b", the empty preedit winit sends before a commit, then "啊不".
pub const PINYIN: [Edit; 3] = [
    ("Warm", "Warma", (5, 5), Some((4, 5))),
    ("Warma", "Warma b", (7, 7), Some((4, 7))),
    ("Warma b", "Warm啊不", (6, 6), None),
];

/// A dead key over "arm" selected in "Warm": a preedit of the grave
/// accent, the empty preedit, then the commit "é".
pub const DEAD_KEY: [Edit; 2] = [
    ("Warm", "W`", (1, 1), Some((1, 2))),
    ("W`", "Wé", (2, 2), None),
];
