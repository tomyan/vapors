//! Sheets added from Rust beside analysed elements, refused both ways
//! (build-time-styles.md, decision 2 (a), R1b): an analysed element's
//! compiled winners cannot see a sheet added with `add_stylesheet`, so
//! a document holds one or the other. An element is analysed when its
//! key carries compiled winners; a hand-written key does not.

use vapors_dom::StyleKey;

use super::Sheets;
use crate::error::ElementError;

/// What decides the refusals: the Rust-added sheets' names and the
/// first analysed element's site.
#[derive(Default)]
pub(crate) struct RustSheets {
    added: Vec<String>,
    analysed: Option<&'static str>,
}

impl Sheets {
    /// `Document::add_stylesheet`'s sheet, unless an analysed element
    /// exists.
    ///
    /// # Errors
    /// An analysed element exists, naming its site and the fix; or the
    /// CSS crate's refusal.
    pub(crate) fn add_rust(&mut self, name: &str, text: &str) -> Result<(), ElementError> {
        if let Some(site) = self.rust.analysed {
            return Err(ElementError::Refused(format!(
                "add_stylesheet(\"{name}\") after the analysed element at {site} was created: its \
                 compiled winners cannot see a sheet added from Rust; fix: put the rules in a \
                 component's <style> (a .wisp the build compiles)"
            )));
        }
        self.add_author(name, text)?;
        self.rust.added.push(name.to_owned());
        Ok(())
    }

    /// Notes an element made from `key`; the refusal for it when it is
    /// analysed and a sheet was added from Rust.
    pub(crate) fn keyed_refusal(&mut self, key: &'static StyleKey) -> Option<String> {
        key.winners.compiled()?;
        self.rust.analysed.get_or_insert(key.site);
        let name = self.rust.added.first()?;
        Some(format!(
            "the element at {}: its compiled winners cannot see \"{name}\", a sheet added from \
             Rust with add_stylesheet; fix: put that sheet's rules in a component's <style> (a \
             .wisp the build compiles), or build this element without the analysed component",
            key.site
        ))
    }
}
