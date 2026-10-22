//! What the comparison page shows: two panels (light on dark, dark on
//! light) of the same blocks at fixed positions, so the HTML and the
//! framework place text identically and a region means the same thing
//! in both renderings.

use vapors_scene::Color;

use crate::Family;

/// Width of each panel, pixels.
pub const PANEL_WIDTH: u32 = 460;
/// Inset of text from the panel edge.
pub const PADDING: u32 = 24;
/// The whole page: two panels side by side.
pub const PAGE_WIDTH: u32 = PANEL_WIDTH * 2;
/// Page height; the last block's bottom plus padding.
pub const PAGE_HEIGHT: u32 = 580;

/// Ink and background of one panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scheme {
    /// Name for labels and docs.
    pub name: &'static str,
    /// Text colour.
    pub ink: Color,
    /// Panel colour.
    pub background: Color,
}

/// Left panel light on dark (the touche bar colours), right panel dark on light.
pub const SCHEMES: [Scheme; 2] = [
    Scheme {
        name: "light on dark",
        ink: rgb(0xe4, 0xe8, 0xec),
        background: rgb(0x17, 0x1b, 0x1f),
    },
    Scheme {
        name: "dark on light",
        ink: rgb(0x1a, 0x1d, 0x20),
        background: rgb(0xf4, 0xf4, 0xf2),
    },
];

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

/// One block of text at a fixed place in every panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// The text.
    pub text: &'static str,
    /// Family.
    pub family: Family<'static>,
    /// Size in pixels.
    pub size: f32,
    /// Weight.
    pub weight: f32,
    /// `tnum`.
    pub tabular_figures: bool,
    /// Line height as a multiple of the size.
    pub line_height: f32,
    /// Top of the block from the page top, pixels.
    pub top: u32,
    /// Height reserved for the block, pixels.
    pub height: u32,
}

impl Block {
    /// Width the text wraps at.
    #[must_use]
    pub const fn width() -> u32 {
        PANEL_WIDTH - 2 * PADDING
    }
}

const BODY: &str = "Retouching is the slow part. The agent proposes a curve, the reviewer \
                    reads the readout, and the file keeps every step so a change can be \
                    undone a week later without guessing what moved.";

/// The blocks, top to bottom. The 13 px body is the one measured.
#[must_use]
pub fn blocks() -> Vec<Block> {
    let body = |size: f32, top: u32, height: u32| Block {
        text: BODY,
        family: Family::Geist,
        size,
        weight: 400.0,
        tabular_figures: false,
        line_height: 1.5,
        top,
        height,
    };
    vec![
        Block {
            text: "The quick brown fox jumps over the lazy dog",
            family: Family::Geist,
            size: 28.0,
            weight: 600.0,
            tabular_figures: false,
            line_height: 1.2,
            top: 24,
            height: 72,
        },
        Block {
            text: "R 238, G 231, B 222 → 245",
            family: Family::Geist,
            size: 13.0,
            weight: 400.0,
            tabular_figures: true,
            line_height: 1.5,
            top: 108,
            height: 24,
        },
        body(12.0, 144, 72),
        body(13.0, 228, 78),
        body(14.0, 318, 84),
        body(15.0, 414, 90),
        Block {
            text: "let alpha = adjust(coverage, luminance); // gamma 1.2, contrast 0.2",
            family: Family::GeistMono,
            size: 13.0,
            weight: 400.0,
            tabular_figures: false,
            line_height: 1.5,
            top: 516,
            height: 40,
        },
    ]
}

/// Index in [`blocks`] of the 13 px body: the block whose mean ink is
/// compared with Chromium's.
pub const MEASURED_BLOCK: usize = 3;

/// `[x, y, width, height]` of the measured block in the first panel,
/// in device pixels at `scale`.
#[must_use]
pub fn body_measure_rect(scale: u32) -> [u32; 4] {
    block_rect(0, MEASURED_BLOCK).map(|v| v * scale)
}

/// `[x, y, width, height]` of block `block` in panel `panel`.
///
/// # Panics
///
/// If either index is out of range; a caller bug.
#[must_use]
pub fn block_rect(panel: usize, block: usize) -> [u32; 4] {
    assert!(panel < SCHEMES.len(), "panel {panel} of {}", SCHEMES.len());
    let block = blocks()[block];
    let left = u32::try_from(panel).expect("two panels") * PANEL_WIDTH + PADDING;
    [left, block.top, Block::width(), block.height]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_stacked_without_overlap_inside_the_page() {
        // Given
        let blocks = blocks();

        // When / Then
        for pair in blocks.windows(2) {
            assert!(pair[0].top + pair[0].height <= pair[1].top, "{pair:?}");
        }
        let last = blocks.last().expect("blocks");
        assert_eq!(last.top + last.height + PADDING, PAGE_HEIGHT);
    }

    #[test]
    fn the_measured_region_is_the_13px_body() {
        // When
        let block = blocks()[MEASURED_BLOCK];
        let rect = body_measure_rect(1);

        // Then
        assert_eq!(block.size, 13.0);
        assert_eq!(block.text, BODY);
        assert_eq!(rect, [24, 228, 412, 78]);
        assert_eq!(body_measure_rect(2), [48, 456, 824, 156]);
    }
}
