//! The demos: one component per F2c slice, each shown at
//! `/vapors/?demo=<name>` on the web page and by
//! `cargo run -p vapors-app --example demo -- <name>` in a window, with
//! the pixels both screenshots are checked against in
//! `demos/<name>.pixels`. The plan names each slice's demo
//! (docs/F2c.md, "Demos").

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_text::FontSystem;

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

pub use vapors_pixels as pixels;

mod frames;
mod mounts;
use mounts::{register_thumbnails, stress_per_frame};
mod input;
pub use frames::{frame_limit, limited};
pub use input::{click, input};

/// What a demo that changes itself every frame is called with before
/// each update: how long the last update took (`None` before the first).
pub type PerFrame = Box<dyn FnMut(Option<Duration>)>;

/// Mounts a demo's component into `parent`; a demo that changes every
/// frame returns what to call before each update.
pub type Mount = fn(&Rc<RefCell<Document>>, NodeId) -> Option<PerFrame>;

/// A demo's document, and what to call before each update if it
/// changes every frame.
pub struct Shown {
    /// The document, its component mounted.
    pub document: Rc<RefCell<Document>>,
    /// See [`PerFrame`].
    pub per_frame: Option<PerFrame>,
}

/// One demo.
pub struct Demo {
    /// Its name in `?demo=`, and its file stem under `demos/`.
    pub name: &'static str,
    /// The pixel checks for its screenshots, or `None` for a demo
    /// without any (the index).
    pub pixels: Option<&'static str>,
    /// Mounts it, leaking the component so it stays mounted for the
    /// life of the page.
    pub mount: Mount,
}

/// The demo shown when the page has no `?demo=`.
pub const INDEX: &str = "index";

/// The demo named `name`.
///
/// # Errors
/// An unknown name, listing the names there are.
pub fn find(name: &str) -> Result<Demo, String> {
    let demos = all();
    let names: Vec<&str> = demos.iter().map(|d| d.name).collect();
    demos
        .into_iter()
        .find(|demo| demo.name == name)
        .ok_or_else(|| {
            format!(
                "no demo is called `{name}`; the demos are {}, and image",
                names.join(", ")
            )
        })
}

/// Every elements demo, by name.
#[must_use]
pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            name: "hello",
            pixels: Some(include_str!("../demos/hello.pixels")),
            mount: |dom, parent| {
                std::mem::forget(hello::Hello::mount(dom, parent, hello::HelloProps {}));
                None
            },
        },
        Demo {
            name: "counter",
            pixels: Some(include_str!("../demos/counter.pixels")),
            mount: |dom, parent| {
                std::mem::forget(counter::Counter::mount(
                    dom,
                    parent,
                    counter::CounterProps {},
                ));
                None
            },
        },
        Demo {
            name: "boxes",
            pixels: Some(include_str!("../demos/boxes.pixels")),
            mount: |dom, parent| {
                std::mem::forget(boxes::Boxes::mount(dom, parent, boxes::BoxesProps {}));
                None
            },
        },
        Demo {
            name: "shell",
            pixels: Some(include_str!("../demos/shell.pixels")),
            mount: |dom, parent| {
                std::mem::forget(shell::Shell::mount(dom, parent, shell::ShellProps {}));
                None
            },
        },
        Demo {
            name: "panel",
            pixels: Some(include_str!("../demos/panel.pixels")),
            mount: |dom, parent| {
                std::mem::forget(panel::Panel::mount(dom, parent, panel::PanelProps {}));
                None
            },
        },
        Demo {
            name: "text",
            pixels: Some(include_str!("../demos/text.pixels")),
            mount: |dom, parent| {
                std::mem::forget(text::Text::mount(dom, parent, text::TextProps {}));
                None
            },
        },
        Demo {
            name: "stress",
            pixels: Some(include_str!("../demos/stress.pixels")),
            mount: |dom, parent| {
                Some(stress_per_frame(stress::Stress::mount(
                    dom,
                    parent,
                    stress::StressProps {},
                )))
            },
        },
        Demo {
            name: "toasts",
            pixels: Some(include_str!("../demos/toasts.pixels")),
            mount: |dom, parent| {
                std::mem::forget(toasts::Toasts::mount(dom, parent, toasts::ToastsProps {}));
                None
            },
        },
        Demo {
            name: "scroll",
            pixels: Some(include_str!("../demos/scroll.pixels")),
            mount: |dom, parent| {
                std::mem::forget(scroll::Scroll::mount(dom, parent, scroll::ScrollProps {}));
                None
            },
        },
        Demo {
            name: "images",
            pixels: Some(include_str!("../demos/images.pixels")),
            mount: |dom, parent| {
                register_thumbnails(&mut dom.borrow_mut());
                std::mem::forget(images::Images::mount(dom, parent, images::ImagesProps {}));
                None
            },
        },
        Demo {
            name: INDEX,
            pixels: None,
            mount: |dom, parent| {
                std::mem::forget(index::Index::mount(dom, parent, index::IndexProps {}));
                None
            },
        },
    ]
}

/// A document showing `demo`: its component mounted in `body`, which
/// adds its scoped style, on the platform's fonts (the bundle, then
/// system fonts natively; the bundle alone on the web). A style that
/// does not parse is refused by the first update, naming the file.
#[must_use]
pub fn document(demo: &Demo) -> Shown {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    let body = document.borrow().body();
    let per_frame = (demo.mount)(&document, body);
    Shown {
        document,
        per_frame,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_demos_pixels_file_parses() {
        for demo in super::all() {
            if let Some(text) = demo.pixels {
                vapors_pixels::parse(demo.name, text).expect("parses");
            }
        }
    }
}
