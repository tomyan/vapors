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

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

pub mod pixels;

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
            name: INDEX,
            style: index::STYLE,
            pixels: None,
            mount: |dom, parent| {
                std::mem::forget(index::Index::mount(dom, parent, index::IndexProps {}));
            },
        },
    ]
}
