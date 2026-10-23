//! The layout corpus: each `fixtures/<name>.wisp` is a component with a
//! `<style>` and static HTML and no script, so the same file is a page
//! Chromium can load. `fixtures/<name>.boxes.json` holds Chromium's
//! border box for every element, written by
//! `cargo run -p vapors-elements-corpus --example layout_oracle`, and
//! `tests/corpus.rs` checks vapors-elements' layout against it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_elements::Document;

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

pub mod oracle;
pub mod page;
pub mod paint_check;

/// Mounts a fixture's component into `parent`.
pub type Mount = fn(&Rc<RefCell<Document>>, NodeId);

/// A fixture: its name (the file stem), its `<style>`, and its mount.
pub struct Fixture {
    /// The file stem under `fixtures/`.
    pub name: &'static str,
    /// The component's `<style>` text.
    pub style: &'static str,
    /// Mounts it, leaking the component so it stays mounted.
    pub mount: Mount,
}

macro_rules! fixtures {
    ($($module:ident :: $component:ident),* $(,)?) => {
        /// Every fixture, by name. A `.wisp` under `fixtures/` that is not
        /// listed here fails `every_fixture_file_is_listed`.
        #[must_use]
        pub fn all() -> Vec<Fixture> {
            vec![$(Fixture {
                name: stringify!($module),
                style: $module::STYLE,
                mount: |dom, parent| {
                    std::mem::forget($module::$component::mount(dom, parent, Default::default()));
                },
            }),*]
        }
    };
}

fixtures![
    block_margins::BlockMargins,
    borders::Borders,
    centred_panel::CentredPanel,
    flex_row::FlexRow,
    grid_tracks::GridTracks,
    hello_box::HelloBox,
    inline_text::InlineText,
    paint_boxes::PaintBoxes,
    shell::Shell,
    sizing::Sizing,
    text_wrap::TextWrap,
];
