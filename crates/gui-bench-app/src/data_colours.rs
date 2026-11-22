//! The data colours `style:background-color` and `style:color` read: the
//! spec's avatar palette and status pill colours as typed `Rgba` by index.
//! An index past the palette is refused, never clamped (`FLAGS.md`).
use crate::dashboard::{AVATAR_COLORS, Status};
use vapors_elements::Rgba;

const REPRODUCE: &str = "cargo test -p vapors-gui-bench-app data_colours";

/// A spec `0xRRGGBB` colour, opaque.
fn rgb(color: u32) -> Rgba {
    let [_, r, g, b] = color.to_be_bytes();
    Rgba::rgb(r, g, b)
}

/// The avatar background of palette entry `index`.
pub fn avatar(index: usize) -> Rgba {
    let Some(color) = AVATAR_COLORS.get(index) else {
        panic!(
            "avatar colour {index} is outside the {} palette entries: pass an index below it ({REPRODUCE})",
            AVATAR_COLORS.len()
        );
    };
    rgb(*color)
}

/// The pill background of checkbox row `index` (`Status::ALL`).
pub fn pill_background(index: usize) -> Rgba {
    rgb(status(index).colors().0)
}

/// The pill text colour of checkbox row `index` (`Status::ALL`).
pub fn pill_text(index: usize) -> Rgba {
    rgb(status(index).colors().1)
}

fn status(index: usize) -> Status {
    let Some(status) = Status::ALL.get(index) else {
        panic!(
            "status {index} is outside the {} pill colours: pass an index below it ({REPRODUCE})",
            Status::ALL.len()
        );
    };
    *status
}
