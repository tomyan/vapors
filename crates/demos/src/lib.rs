//! The demos: one component per F2c slice, each shown at
//! `/vapors/?demo=<name>` on the web page and by
//! `cargo run -p vapors-app --example demo -- <name>` in a window, with
//! the pixels both screenshots are checked against in
//! `demos/<name>.pixels`. The plan names each slice's demo
//! (docs/F2c.md, "Demos").

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_text::FontSystem;

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

pub use vapors_pixels as pixels;

/// Mounts a demo's component into `parent`.
pub type Mount = fn(&Rc<RefCell<Document>>, NodeId);

/// One demo.
pub struct Demo {
    /// Its name in `?demo=`, and its file stem under `demos/`.
    pub name: &'static str,
    /// The component's `<style>`, added as a global sheet (F2c S1 to S4).
    pub style: &'static str,
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
            style: hello::STYLE,
            pixels: Some(include_str!("../demos/hello.pixels")),
            mount: |dom, parent| {
                std::mem::forget(hello::Hello::mount(dom, parent, hello::HelloProps {}));
            },
        },
        Demo {
            name: "counter",
            style: counter::STYLE,
            pixels: Some(include_str!("../demos/counter.pixels")),
            mount: |dom, parent| {
                std::mem::forget(counter::Counter::mount(
                    dom,
                    parent,
                    counter::CounterProps {},
                ));
            },
        },
        Demo {
            name: "boxes",
            style: boxes::STYLE,
            pixels: Some(include_str!("../demos/boxes.pixels")),
            mount: |dom, parent| {
                std::mem::forget(boxes::Boxes::mount(dom, parent, boxes::BoxesProps {}));
            },
        },
        Demo {
            name: "shell",
            style: shell::STYLE,
            pixels: Some(include_str!("../demos/shell.pixels")),
            mount: |dom, parent| {
                std::mem::forget(shell::Shell::mount(dom, parent, shell::ShellProps {}));
            },
        },
        Demo {
            name: INDEX,
            style: index::STYLE,
            pixels: None,
            mount: |dom, parent| {
                std::mem::forget(index::Index::mount(dom, parent, index::IndexProps {}));
            },
        },
    ]
}

/// A document showing `demo`: its style as a global sheet, its
/// component mounted in `body`, on the platform's fonts (the bundle,
/// then system fonts natively; the bundle alone on the web).
///
/// # Errors
/// The demo's style does not parse, naming the line.
pub fn document(demo: &Demo) -> Result<Rc<RefCell<Document>>, String> {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    document
        .borrow_mut()
        .add_stylesheet(&format!("{}.wisp", demo.name), demo.style)
        .map_err(|error| error.to_string())?;
    let body = document.borrow().body();
    (demo.mount)(&document, body);
    Ok(document)
}

/// Clicks at `point` (CSS pixels) as a mouse does: moves there,
/// presses and releases. What a `.pixels` file's `click` lines do
/// before a screenshot; the pointer stays there, hovering.
#[allow(clippy::cast_precision_loss)] // screen coordinates are small
pub fn click(document: &Rc<RefCell<Document>>, point: [u32; 2]) {
    let point = [point[0] as f32, point[1] as f32];
    Document::pointer_moved(document, point);
    Document::pointer_pressed(document, point);
    Document::pointer_released(document, point);
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
