//! The `class` text of an element with `class:name={bool}` directives:
//! its literal names, then each directive's name while it is on.

/// `literal` followed by each name in `toggles` whose flag is true,
/// separated by single spaces.
#[must_use]
pub fn class_names(literal: &str, toggles: &[(&str, bool)]) -> String {
    let mut value = literal.to_string();
    for (name, _) in toggles.iter().filter(|(_, on)| *on) {
        if !value.is_empty() {
            value.push(' ');
        }
        value.push_str(name);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::class_names;

    #[test]
    fn the_literal_names_come_first_then_each_name_that_is_on() {
        // When
        let value = class_names("card", &[("lit", true), ("dim", false), ("wide", true)]);

        // Then
        assert_eq!(value, "card lit wide");
    }

    #[test]
    fn with_no_literal_names_there_is_no_leading_space() {
        // When
        let value = class_names("", &[("lit", false), ("dim", true)]);

        // Then
        assert_eq!(value, "dim");
    }
}
