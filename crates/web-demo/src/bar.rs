//! The bar above the canvas that steps through the demos: a link to the
//! demo before, the demo's title and its line, and a link to the demo
//! after, from `vapors_demos::DEMOS`. This decides what it shows; the
//! page's `vaporsShowBar` (index.html) draws it in `#demo-bar` and
//! makes `[` and `]` click its links, as demo-harness chrome above the
//! canvas, not over it (the page contract holds both). The page script
//! draws it because `web_sys` stays in vapors-platform
//! (`[imports.web_sys]`, .config/factory.toml).

use vapors_demos::{listed, neighbours};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    /// index.html's: the bar, its link before (href, title), the demo's
    /// title and line, and its link after.
    #[wasm_bindgen(js_name = vaporsShowBar, catch)]
    fn show_bar(
        before: Box<[JsValue]>,
        title: &str,
        line: &str,
        after: Box<[JsValue]>,
    ) -> Result<(), JsValue>;
}

/// Shows the bar for the demo `name`; nothing for a name not listed.
///
/// # Errors
/// The page's `vaporsShowBar` missing or failing, naming the fix.
pub fn show(name: &str) -> Result<(), String> {
    let (Some((before, after)), Some(shown)) = (neighbours(name), listed(name)) else {
        return Ok(());
    };
    let link = |to: &vapors_demos::Listed| -> Box<[JsValue]> {
        Box::new([JsValue::from_str(&to.href()), JsValue::from_str(to.title)])
    };
    show_bar(link(before), shown.title, shown.line, link(after)).map_err(|error| {
        format!(
            "the demo bar: index.html's vaporsShowBar failed ({error:?}); it must fill \
             <nav id=\"demo-bar\"> before the canvas"
        )
    })
}
