//! The bundled typefaces, embedded so native and web render the same
//! bytes. See `fonts/README.md` for version and source.

/// Geist, variable weight 100..900. The UI face.
pub const GEIST: &[u8] = include_bytes!("../fonts/Geist-wght.ttf");

/// Geist Mono, variable weight 100..900. Code and numeric readouts.
pub const GEIST_MONO: &[u8] = include_bytes!("../fonts/GeistMono-wght.ttf");

/// Family name of [`GEIST`] as the font declares it.
pub const GEIST_FAMILY: &str = "Geist";

/// Family name of [`GEIST_MONO`] as the font declares it.
pub const GEIST_MONO_FAMILY: &str = "Geist Mono";

#[cfg(test)]
mod tests {
    use skrifa::MetadataProvider;
    use skrifa::string::StringId;

    use super::*;

    fn family_name(bytes: &[u8]) -> String {
        let font = skrifa::FontRef::new(bytes).expect("parses as a font");
        font.localized_strings(StringId::FAMILY_NAME)
            .english_or_first()
            .expect("has a family name")
            .to_string()
    }

    #[test]
    fn bundled_fonts_parse_and_declare_their_family_names() {
        // Given / When
        let geist = family_name(GEIST);
        let mono = family_name(GEIST_MONO);

        // Then
        assert_eq!(geist, GEIST_FAMILY);
        assert_eq!(mono, GEIST_MONO_FAMILY);
    }

    #[test]
    fn bundled_fonts_are_variable_in_weight_only() {
        // Given
        for bytes in [GEIST, GEIST_MONO] {
            let font = skrifa::FontRef::new(bytes).expect("parses as a font");

            // When
            let axes: Vec<String> = font
                .axes()
                .iter()
                .map(|axis| axis.tag().to_string())
                .collect();

            // Then
            assert_eq!(axes, ["wght"]);
        }
    }
}
