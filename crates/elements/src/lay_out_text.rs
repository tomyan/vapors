//! Text laid out outside the document's boxes, with the document's own
//! fonts: for what an app draws over the scene (the tooltip).

use vapors_text::{Layout, LayoutOptions, Style};

use crate::document::Document;

impl Document {
    /// `text` in `style`, laid out with this document's fonts: the
    /// bundle, the system's on native, and every script fallback
    /// registered on its `FontSystem` (the web registers its own), so
    /// a character the document can show this can show too.
    #[must_use]
    pub fn lay_out_text(
        &mut self,
        text: &str,
        style: &Style<'_>,
        options: &LayoutOptions,
    ) -> Layout {
        Layout::new(&mut self.fonts, text, style, options)
    }
}

#[cfg(test)]
mod tests {
    use vapors_text::{Family, FontSystem, LayoutOptions, Style};

    use crate::document::Document;

    #[test]
    fn text_laid_out_by_the_document_wraps_at_the_width_given() {
        // Given
        let mut doc = Document::new(FontSystem::bundle_only());
        let style = Style::new(Family::Geist, 12.0);
        let text = "Undo the last change (Ctrl+Z)";

        // When
        let one_line = doc.lay_out_text(text, &style, &LayoutOptions::default());
        let options = LayoutOptions {
            width: Some(60.0),
            ..LayoutOptions::default()
        };
        let wrapped = doc.lay_out_text(text, &style, &options);

        // Then
        assert_eq!(one_line.lines().count(), 1);
        assert!(wrapped.lines().count() > 1);
        assert!(wrapped.width() <= 60.0);
    }
}
