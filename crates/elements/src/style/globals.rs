//! `Styles::Global` components' sheets (build-time-styles.md, R1a):
//! the generated mount registers its `<style>` itself, by the
//! component's crate and file, so compiled winners can name it.

use crate::error::ElementError;

use super::Sheets;

impl Sheets {
    /// A global component's style, named `sheet`, added once however
    /// often it is mounted. As for a scope, one that does not parse, or
    /// that holds `:global()`, is kept as a refusal for every update.
    /// False when it was already added with this text and nothing
    /// changed.
    pub(crate) fn add_global(&mut self, sheet: &str, text: &str) -> bool {
        match self.globals.get(sheet) {
            Some(added) if added == text => return false,
            Some(_) => {
                self.refused.push(refusal(
                    sheet,
                    format!("global sheet `{sheet}` was already added with another text: two components share a file name; give their files different paths"),
                ));
                return true;
            }
            None => drop(self.globals.insert(sheet.to_owned(), text.to_owned())),
        }
        if let Err(refused) = self.add_author(sheet, text) {
            self.refused.push(refused);
        }
        true
    }
}

fn refusal(sheet: &str, message: String) -> ElementError {
    ElementError::Css {
        sheet: sheet.to_owned(),
        error: vapors_css::Error::new(vapors_css::Position { line: 1, column: 1 }, message),
    }
}
