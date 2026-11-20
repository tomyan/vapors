//! Clippy lints the generated code can trip for reasons the compiler
//! cannot see, because it reads syntax and not types. Each is allowed
//! on the one generated `let` that trips it, with the reason written
//! into the attribute, and never on a statement or expression the user
//! wrote: those stay checked. Two sit on `mount_before` because they
//! judge the function itself: `too_many_lines` is about its length,
//! which follows the template's size, and `missing_panics_doc`, when a numeric or checked
//! binding's listener expects what its control's own event always
//! carries (a missing one is a bug in the tree, said loudly). None can hide
//! another lint. `needless_pass_by_value` needs
//! none: `mount_before` moves `props` whole into a one-element tuple before it
//! destructures it, so the parameter is consumed whatever the types of
//! its fields (Copy ones included). A type-aware generator would
//! retire the first three; building each template node in its own
//! function would retire `too_many_lines`. Generated node names need no
//! allowance: their counters share one width per component, so clippy's
//! `similar_names` never sees two that differ by an inserted digit.

use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Allowance {
    CloneOnCopy,
    CloneOnCopyKey,
    ImplicitClone,
    MissingPanicsDoc,
    NeedlessUpdate,
    TooManyLines,
    UselessConversion,
}

impl Allowance {
    fn lint(self) -> &'static str {
        match self {
            Allowance::CloneOnCopy | Allowance::CloneOnCopyKey => "clippy::clone_on_copy",
            Allowance::ImplicitClone => "clippy::implicit_clone",
            Allowance::MissingPanicsDoc => "clippy::missing_panics_doc",
            Allowance::NeedlessUpdate => "clippy::needless_update",
            Allowance::TooManyLines => "clippy::too_many_lines",
            Allowance::UselessConversion => "clippy::useless_conversion",
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Allowance::CloneOnCopy => {
                "a closure gets a clone of each binding it captures; the compiler cannot see which are Copy"
            }
            Allowance::CloneOnCopyKey => {
                "a row's key is cloned into the map and kept for the cleanup; the compiler cannot see a Copy key"
            }
            Allowance::ImplicitClone => {
                "a text hole or expression attribute shows any Display value through to_string; the compiler cannot see when it is already a String"
            }
            Allowance::MissingPanicsDoc => {
                "a numeric or checked binding's listener expects what its control's own input or change event always carries; a missing one is a bug in the tree"
            }
            Allowance::TooManyLines => {
                "mount_before builds the whole template, so its length follows the template's size"
            }
            Allowance::UselessConversion => {
                "a prop that is a signal or memo binding converts into the child's prop type (ReadSignal or Signal); the compiler cannot see when the types already match"
            }
            Allowance::NeedlessUpdate => {
                "a child's props end with ..Default::default(); the compiler cannot see when every prop is already given"
            }
        }
    }
}

impl Allowance {
    /// The attribute, for the line before the `let` it covers.
    pub(crate) fn attribute(self) -> String {
        format!("#[allow({}, reason = {:?})]", self.lint(), self.reason())
    }
}

/// The lines that turn an expression into the text shown: the user's
/// expression as its own statement, then the allowed `to_string`.
pub(crate) fn shown_text(expr: &str) -> [String; 3] {
    [
        format!("let shown = &({expr});"),
        Allowance::ImplicitClone.attribute(),
        "let value = shown.to_string();".to_string(),
    ]
}

/// Lints on `mount_before` that depend on types the compiler cannot see:
/// its length and the expectation a numeric binding's listener makes (`element_bind`).
pub(crate) fn mount_allowances(body: &str) -> String {
    let mut allowances = format!("    {}\n", Allowance::TooManyLines.attribute());
    if body.contains("event.value_as_number().expect(") || body.contains("event.checked().expect(")
    {
        let _ = writeln!(
            allowances,
            "    {}",
            Allowance::MissingPanicsDoc.attribute()
        );
    }
    allowances
}
