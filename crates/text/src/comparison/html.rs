//! The same page as HTML: absolute positions from the spec, the bundled
//! fonts through `@font-face`, so Chromium renders exactly these blocks.

use std::fmt::Write as _;

use vapors_scene::Color;

use crate::Family;
use crate::fonts::{GEIST, GEIST_MONO};

use super::spec::{Block, PADDING, PAGE_HEIGHT, PAGE_WIDTH, PANEL_WIDTH, SCHEMES, blocks};

/// Font files the HTML refers to, to write beside it.
pub const FONT_FILES: [(&str, &[u8]); 2] = [
    ("Geist-wght.ttf", GEIST),
    ("GeistMono-wght.ttf", GEIST_MONO),
];

/// The page.
#[must_use]
pub fn html() -> String {
    let mut out = String::new();
    out.push_str(&head());
    for (index, scheme) in SCHEMES.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)] // two panels
        let left = PANEL_WIDTH * index as u32;
        let _ = writeln!(
            out,
            "<div class=\"panel\" style=\"left:{left}px;background:{};color:{}\">",
            css(scheme.background),
            css(scheme.ink)
        );
        for block in blocks() {
            out.push_str(&block_html(&block));
        }
        out.push_str("</div>\n");
    }
    out.push_str("</body></html>\n");
    out
}

fn head() -> String {
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>text comparison</title>\n\
         <style>\n\
         @font-face{{font-family:\"Geist\";src:url(\"Geist-wght.ttf\");font-weight:100 900}}\n\
         @font-face{{font-family:\"Geist Mono\";src:url(\"GeistMono-wght.ttf\");font-weight:100 900}}\n\
         html,body{{margin:0;width:{PAGE_WIDTH}px;height:{PAGE_HEIGHT}px;overflow:hidden}}\n\
         .panel{{position:absolute;top:0;width:{PANEL_WIDTH}px;height:{PAGE_HEIGHT}px}}\n\
         .block{{position:absolute;left:{PADDING}px;width:{}px;margin:0;font-kerning:normal}}\n\
         </style></head><body>\n",
        Block::width()
    )
}

fn block_html(block: &Block) -> String {
    let numeric = if block.tabular_figures {
        "font-variant-numeric:tabular-nums;"
    } else {
        ""
    };
    format!(
        "<p class=\"block\" style=\"top:{}px;height:{}px;font:{} {}px/{} {};{numeric}\">{}</p>\n",
        block.top,
        block.height,
        block.weight,
        block.size,
        block.line_height,
        family_css(block.family),
        escape(block.text)
    )
}

/// Single quotes: these land inside a double-quoted `style` attribute.
fn family_css(family: Family<'_>) -> String {
    match family {
        Family::Geist => "'Geist'".to_owned(),
        Family::GeistMono => "'Geist Mono'".to_owned(),
        Family::Named(name) => format!("'{name}'"),
    }
}

fn css(color: Color) -> String {
    let [r, g, b, _] = color.to_rgba8();
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_html_places_every_block_in_both_panels_with_the_spec_values() {
        // When
        let page = html();

        // Then
        assert_eq!(page.matches("class=\"panel\"").count(), 2);
        assert_eq!(page.matches("class=\"block\"").count(), 2 * blocks().len());
        assert!(page.contains("top:228px;height:78px;font:400 13px/1.5 'Geist'"));
        assert!(page.contains("font-variant-numeric:tabular-nums"));
        assert!(page.contains("B 222 → 245"));
        assert!(page.contains("background:#171b1f;color:#e4e8ec"));
    }

    #[test]
    fn no_double_quote_appears_inside_a_style_attribute() {
        // Given
        let page = html();

        // When: every style attribute's value
        let values = page
            .split("style=\"")
            .skip(1)
            .map(|rest| rest.split('"').next().unwrap_or(""));

        // Then: none of them is cut short at a family name
        for value in values {
            assert!(
                !value.contains("font:") || value.contains("'Geist"),
                "{value}"
            );
        }
    }

    #[test]
    fn markup_characters_in_the_text_are_escaped() {
        assert_eq!(escape("a < b & c"), "a &lt; b &amp; c");
    }
}
