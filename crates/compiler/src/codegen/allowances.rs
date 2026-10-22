//! Clippy lints the generated code can trip for reasons the compiler
//! cannot see, because it reads syntax and not types. Each is allowed
//! on the one generated `let` that trips it, with the reason written
//! into the attribute, and never on a statement or expression the user
//! wrote: those stay checked. Two sit on `mount_before` because they
//! judge the function itself: `needless_pass_by_value` is about its
//! `props` parameter, which no script code can touch, and
//! `too_many_lines` is about its length, which follows the template's
//! size. Neither can hide another lint. A type-aware generator would
//! retire the first four; building each template node in its own
//! function would retire `too_many_lines`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Allowance {
    CloneOnCopy,
    ImplicitClone,
    NeedlessPassByValue,
    NeedlessUpdate,
    TooManyLines,
}

impl Allowance {
    fn lint(self) -> &'static str {
        match self {
            Allowance::CloneOnCopy => "clippy::clone_on_copy",
            Allowance::ImplicitClone => "clippy::implicit_clone",
            Allowance::NeedlessPassByValue => "clippy::needless_pass_by_value",
            Allowance::NeedlessUpdate => "clippy::needless_update",
            Allowance::TooManyLines => "clippy::too_many_lines",
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Allowance::CloneOnCopy => {
                "a closure gets a clone of each binding it captures; the compiler cannot see which are Copy"
            }
            Allowance::ImplicitClone => {
                "a text hole or expression attribute shows any Display value through to_string; the compiler cannot see when it is already a String"
            }
            Allowance::NeedlessPassByValue => {
                "every component takes its props struct by value, empty or not, so parents mount them alike"
            }
            Allowance::TooManyLines => {
                "mount_before builds the whole template, so its length follows the template's size"
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
