//! `Styles::Global` components' sheets (build-time-styles.md, R1a):
//! the generated mount registers its `<style>` itself, by the
//! component's crate and file, so compiled winners can name it.

use crate::error::ElementError;

use super::Sheets;

impl Sheets {
    /// A global component's style, named `sheet`, added once however
    /// often it is mounted. As for a scope, one that does not parse, or
    /// that holds `:global()`, is kept as a refusal for every update.
    /// False when nothing changed: it was already added with this text,
    /// or its other text was already refused.
    ///
    /// Like a scoped sheet, it stays in the cascade after its component
    /// unmounts; unlike one, it still styles every element it matches,
    /// as Svelte's `:global` styles do after the component is destroyed
    /// (build-time-styles.md, R1a). `compile_dir_global` is for the
    /// layout corpus alone, which mounts one fixture per document.
    pub(crate) fn add_global(&mut self, sheet: &str, text: &str) -> bool {
        match self.globals.get(sheet) {
            Some(added) if added.text == text => return false,
            Some(_) => return self.refuse_other_text(sheet),
            None => drop(
                self.globals
                    .insert(sheet.to_owned(), super::compiled_add::Registered::new(text)),
            ),
        }
        match self.add_author(sheet, text) {
            Ok(()) => self.note_sheet(super::sheet_hashes::SheetKind::Global, sheet, text),
            Err(refused) => self.refused.push(refused),
        }
        true
    }

    /// Refuses `sheet` added with another text than before, once. Its
    /// name holds the crate's name, so within one build only two versions
    /// of one crate in the dependency graph can collide.
    fn refuse_other_text(&mut self, sheet: &str) -> bool {
        let reported = (self.refused.iter())
            .any(|e| matches!(e, ElementError::Css { sheet: s, .. } if s == sheet));
        if reported {
            return false;
        }
        self.refused.push(refusal(
            sheet,
            format!(
                "global sheet `{sheet}` was already added with another text: two versions of \
                 its crate are in the build; fix: depend on one version of it (`cargo tree -d` \
                 lists the duplicates)"
            ),
        ));
        true
    }
}

fn refusal(sheet: &str, message: String) -> ElementError {
    ElementError::Css {
        sheet: sheet.to_owned(),
        error: vapors_css::Error::new(vapors_css::Position { line: 1, column: 1 }, message),
    }
}

impl Sheets {
    /// Whether `sheet`'s text hashes as the compiler said it does: a
    /// sheet claiming text it does not carry is refused, naming it, for
    /// every update. A test's sheet carries no hash.
    pub(crate) fn compiled_text(&mut self, sheet: vapors_dom::CompiledSheet<'_>) -> bool {
        let Some(claimed) = sheet.hash() else {
            return true;
        };
        if claimed == vapors_css::sheet_hash::sheet_hash(sheet.text()) {
            return true;
        }
        let message = format!(
            "sheet `{}` claims hash {claimed:#x} for text that hashes otherwise: it was not made \
             by the compiler from this text; fix: register a component's sheet by mounting the \
             component, never by naming its hidden constructor; reproduce: the mount that \
             registered it",
            sheet.name()
        );
        self.refused.push(refusal(sheet.name(), message));
        false
    }
}
