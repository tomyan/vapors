//! The data colours: every palette entry, foreground and background,
//! refusal of an index past it, and the rows that bind them (Chromium's
//! view of the same colours: `data_colours_oracle.rs`).
use crate::dashboard::{AVATAR_COLORS, Status};
use crate::data_colours::{avatar, pill_background, pill_text};
use vapors_elements::Rgba;

fn rgb(color: u32) -> Rgba {
    let [_, r, g, b] = color.to_be_bytes();
    Rgba::rgb(r, g, b)
}

#[test]
fn every_avatar_entry_is_its_palette_colour() {
    assert_eq!(AVATAR_COLORS.len(), 8);
    for (i, color) in AVATAR_COLORS.iter().enumerate() {
        assert_eq!(avatar(i), rgb(*color), "avatar {i}");
    }
}

#[test]
fn every_pill_has_its_status_foreground_and_background() {
    for (i, status) in Status::ALL.iter().enumerate() {
        let (back, fore) = status.colors();
        assert_eq!(pill_background(i), rgb(back), "pill {i} background");
        assert_eq!(pill_text(i), rgb(fore), "pill {i} text");
    }
}

#[test]
#[should_panic(expected = "avatar colour 8 is outside the 8 palette entries")]
fn an_avatar_index_past_the_palette_is_refused() {
    avatar(AVATAR_COLORS.len());
}

#[test]
#[should_panic(expected = "status 4 is outside the 4 pill colours")]
fn a_pill_background_past_the_statuses_is_refused() {
    pill_background(Status::ALL.len());
}

#[test]
#[should_panic(expected = "status 4 is outside the 4 pill colours")]
fn a_pill_text_past_the_statuses_is_refused() {
    pill_text(Status::ALL.len());
}
