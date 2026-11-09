//! A node as a driver names it (R1): its role and exact name, within an
//! ancestor's when the name alone is not enough (`a11y_find`).

/// A node by its role and exact name, within an ancestor's when given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxTarget {
    /// The role.
    pub role: String,
    /// The exact accessible name.
    pub name: String,
    /// An ancestor holding it.
    pub within: Option<AxWithin>,
}

/// An ancestor's role and exact name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxWithin {
    /// The role.
    pub role: String,
    /// The exact accessible name.
    pub name: String,
}

impl AxTarget {
    /// The ancestor as [`Document::ax_find`] takes it.
    #[must_use]
    pub fn within(&self) -> Option<(&str, &str)> {
        self.within
            .as_ref()
            .map(|w| (w.role.as_str(), w.name.as_str()))
    }
}
