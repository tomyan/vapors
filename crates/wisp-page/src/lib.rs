//! A page whose crate does not depend on `vapors-dom` and whose `#[prop]`
//! type names an item the script imports. It builds, or the generated
//! code names something this crate cannot reach (#508).

/// What the page's prop type names through the script's `use`.
pub mod taps {
    /// A set of faces.
    #[derive(Default)]
    pub struct Faces(pub Vec<u8>);
}

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

#[cfg(test)]
mod tests {
    #[test]
    fn a_prop_type_may_name_an_imported_item() {
        // When
        let props = crate::spin_cube::SpinCubeProps::default();

        // Then
        assert_eq!(props.faces.0.len(), 0);
    }
}
