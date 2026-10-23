//! `calc()` values that still hold a % term after the cascade: Taffy
//! holds each as an opaque handle (the address of the computed `Calc`,
//! never dereferenced) and asks for its value against a basis during
//! layout. This is the table it asks: clones of the calcs, by the
//! address they had while the computed styles were alive.

use std::cell::RefCell;
use std::collections::HashMap;

use vapors_css::cascade::ComputedStyle;
use vapors_css::value::{Calc, Context, Value};

/// One calc, and where it came from, for a failure to name.
struct Entry {
    calc: Calc,
    /// The element, as errors name it (`<div class="panel">`).
    element: String,
    property: &'static str,
}

/// The calcs of every styled element, by handle.
#[derive(Default)]
pub(crate) struct Calcs {
    by_handle: HashMap<usize, Entry>,
    failure: RefCell<Option<String>>,
}

impl Calcs {
    /// Adds the calcs in `style`, keyed by the address Taffy is given;
    /// `element` describes the element they were computed for, called
    /// only when there is one.
    pub(crate) fn add(&mut self, style: &ComputedStyle, element: impl Fn() -> String) {
        for (property, value) in style.iter() {
            if let Value::Calc(calc) = value {
                let entry = Entry {
                    calc: calc.clone(),
                    element: element(),
                    property: property.name(),
                };
                self.by_handle.insert(handle(calc), entry);
            }
        }
    }

    /// The value of the calc with `handle` when % means `basis` px. A
    /// handle not in the table, or a calc that does not evaluate (a
    /// division by zero), is recorded as the layout's failure and lays
    /// out as 0; `failure` then refuses the update.
    #[allow(clippy::cast_possible_truncation)] // CSS lengths are far inside f32
    pub(crate) fn resolve(&self, handle_ptr: *const (), basis: f32) -> f32 {
        let Some(entry) = self.by_handle.get(&(handle_ptr as usize)) else {
            self.fail(format!(
                "layout asked for a calc() it was never given (handle {handle_ptr:p})"
            ));
            return 0.0;
        };
        match entry.calc.evaluate(&context(basis)) {
            Ok(px) => px as f32,
            Err(error) => {
                self.fail(format!(
                    "{}: `{}: {}` does not evaluate when % is {basis} px: {error}",
                    entry.element,
                    entry.property,
                    Value::Calc(entry.calc.clone()).to_css()
                ));
                0.0
            }
        }
    }

    fn fail(&self, message: String) {
        self.failure.borrow_mut().get_or_insert(message);
    }

    /// The first failure, if any calc did not resolve.
    pub(crate) fn failure(&self) -> Option<String> {
        self.failure.borrow().clone()
    }
}

/// The handle Taffy holds for `calc`: its address.
fn handle(calc: &Calc) -> usize {
    std::ptr::from_ref(calc) as usize
}

/// Only `%` is left in a computed calc: the cascade resolved every other
/// unit, so the rest of the context is never read.
fn context(basis: f32) -> Context {
    Context {
        font_size: 0.0,
        root_font_size: 0.0,
        viewport_width: 0.0,
        viewport_height: 0.0,
        zero_advance: 0.0,
        percent_base: Some(f64::from(basis)),
    }
}
