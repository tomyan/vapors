//! Clippy lints the generated code can trip for reasons the compiler
//! cannot see, because it reads syntax and not types. Each is allowed
//! on `mount_before` only when the code that trips it was generated,
//! with the reason written into the attribute. A type-aware generator
//! would retire them.

use std::collections::BTreeSet;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Allowance {
    CloneOnCopy,
    ImplicitClone,
    NeedlessPassByValue,
    NeedlessUpdate,
}

impl Allowance {
    fn lint(self) -> &'static str {
        match self {
            Allowance::CloneOnCopy => "clippy::clone_on_copy",
            Allowance::ImplicitClone => "clippy::implicit_clone",
            Allowance::NeedlessPassByValue => "clippy::needless_pass_by_value",
            Allowance::NeedlessUpdate => "clippy::needless_update",
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Allowance::CloneOnCopy => {
                "a closure gets a clone of each binding it captures; the compiler cannot see which are Copy"
            }
            Allowance::ImplicitClone => {
                "a text hole shows any Display value through to_string; the compiler cannot see when it is already a String"
            }
            Allowance::NeedlessPassByValue => {
                "every component takes its props struct by value, empty or not, so parents mount them alike"
            }
            Allowance::NeedlessUpdate => {
                "a child's props end with ..Default::default(); the compiler cannot see when every prop is already given"
            }
        }
    }
}

/// One `#[allow]` line per allowance, indented for a method.
pub(crate) fn attributes(allowances: &BTreeSet<Allowance>) -> String {
    let mut text = String::new();
    for allowance in allowances {
        let _ = writeln!(
            text,
            "    #[allow({}, reason = {:?})]",
            allowance.lint(),
            allowance.reason()
        );
    }
    text
}
