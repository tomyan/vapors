//! The crate's sheets as the build hands them to each component's
//! analysis: every other styled component's, as the runtime registers
//! it, with the hash of the text it registers
//! (`vapors_css::sheet_hash`), for the emitted `KeyWinners`.

use vapors_css::analysis::SheetId;
use vapors_css::ast::Stylesheet;
use vapors_css::sheet_hash::sheet_hash;

use crate::ast::Component;
use crate::source::Source;

/// Every styled component's sheet in a crate, parsed once.
pub struct CrateSheets {
    sheets: Vec<(String, SheetId, Stylesheet, u64)>,
}

impl CrateSheets {
    /// The sheets of `components`. A `<style>` the build refuses is left
    /// out: that component's own generation refuses it, naming the
    /// position, so the build fails either way.
    #[must_use]
    pub fn of<'c>(components: impl IntoIterator<Item = &'c Component>) -> Self {
        let sheets = components.into_iter().filter_map(|component| {
            let section = component.style.as_ref()?;
            let source = Source::new(&component.path, &component.text);
            let (id, sheet) = super::own_sheet(component, section, &source).ok()?;
            Some((component.path.clone(), id, sheet, sheet_hash(&section.text)))
        });
        Self {
            sheets: sheets.collect(),
        }
    }

    /// The sheets but `component`'s own.
    #[must_use]
    pub fn others_for(&self, component: &Component) -> OtherSheets {
        let others = self
            .sheets
            .iter()
            .filter(|(path, ..)| *path != component.path);
        let (sheets, hashes) = others
            .map(|(_, id, sheet, hash)| ((id.clone(), sheet.clone()), *hash))
            .unzip();
        OtherSheets { sheets, hashes }
    }
}

/// The sheets of a crate's other components, for one component.
#[derive(Default)]
pub struct OtherSheets {
    pub(crate) sheets: Vec<(SheetId, Stylesheet)>,
    hashes: Vec<u64>,
}

impl OtherSheets {
    /// The hash of each sheet `styles::analyse` lists for `component`,
    /// in its order: the user-agent sheet, the component's own, then
    /// these.
    pub(crate) fn hashes_for(&self, component: &Component) -> Vec<u64> {
        let user_agent = sheet_hash(vapors_css::user_agent::TEXT);
        let own = component
            .style
            .as_ref()
            .map(|section| sheet_hash(&section.text));
        let mut hashes = vec![user_agent];
        hashes.extend(own);
        hashes.extend(&self.hashes);
        hashes
    }
}
