//! A required prop (`#[prop] let controls: Controls;`, no default): the
//! generated props struct holds it as `Required<T>`, `Missing` until a
//! parent's tag passes it. The compiler refuses a tag of the same crate
//! that omits it; a tag it cannot check (another crate's component)
//! reaches the mount, which refuses through [`Required::take`] and
//! never fills a default.

/// A prop that has no default: given by the tag, or missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Required<T> {
    /// No tag passed it.
    Missing,
    /// The value a tag passed.
    Given(T),
}

impl<T> From<T> for Required<T> {
    fn from(value: T) -> Self {
        Required::Given(value)
    }
}

impl<T> Required<T> {
    /// The value, for the mount of `component`.
    ///
    /// # Panics
    /// When no tag passed it, naming the component, the prop and where
    /// the component declares it (`at`): a required prop is never given a
    /// default.
    #[must_use]
    pub fn take(self, component: &str, prop: &str, at: &str) -> T {
        match self {
            Required::Given(value) => value,
            Required::Missing => panic!(
                "`{component}` was mounted without its required prop `{prop}` (declared at {at}): pass `{prop}={{..}}` in its tag, or set `{prop}` in the `{component}Props` literal; reproduce with `cargo xtask test`"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Required;

    #[test]
    fn a_given_value_is_taken() {
        assert_eq!(Required::from(3).take("Card", "n", "card.wisp:1"), 3);
    }

    #[test]
    #[should_panic(
        expected = "`Card` was mounted without its required prop `n` (declared at card.wisp:1)"
    )]
    fn a_missing_value_is_refused_naming_component_and_prop() {
        let missing: Required<i32> = Required::Missing;
        let _ = missing.take("Card", "n", "card.wisp:1");
    }
}
