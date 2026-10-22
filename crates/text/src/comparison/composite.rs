//! The side-by-side image: a label strip, then both renderings with a
//! gap between.

use super::measure::Rgba;

/// Pixels between the two renderings.
const GAP: u32 = 8;

/// `labels` across the top, then `left` and `right` side by side.
///
/// # Panics
///
/// If the strip is not as wide as both renderings and the gap, or the
/// two renderings differ in height; caller bugs with both sizes named.
#[must_use]
pub fn compose(labels: &Rgba, left: &Rgba, right: &Rgba) -> Rgba {
    let width = left.width + GAP + right.width;
    assert_eq!(labels.width, width, "label strip width");
    assert_eq!(left.height, right.height, "rendering heights");
    let height = labels.height + left.height;
    let mut out = Rgba {
        width,
        height,
        data: vec![0; (width * height * 4) as usize],
    };
    blit(&mut out, labels, [0, 0]);
    blit(&mut out, left, [0, labels.height]);
    blit(&mut out, right, [left.width + GAP, labels.height]);
    out
}

fn blit(out: &mut Rgba, source: &Rgba, at: [u32; 2]) {
    let row_bytes = (source.width * 4) as usize;
    for row in 0..source.height {
        let from = (row * source.width * 4) as usize;
        let to = (((at[1] + row) * out.width + at[0]) * 4) as usize;
        out.data[to..to + row_bytes].copy_from_slice(&source.data[from..from + row_bytes]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, value: u8) -> Rgba {
        Rgba {
            width,
            height,
            data: vec![value; (width * height * 4) as usize],
        }
    }

    #[test]
    fn the_composite_places_labels_above_and_the_renderings_either_side_of_the_gap() {
        // Given
        let labels = solid(2 + GAP + 2, 1, 9);
        let left = solid(2, 1, 1);
        let right = solid(2, 1, 2);

        // When
        let out = compose(&labels, &left, &right);

        // Then
        assert_eq!((out.width, out.height), (12, 2));
        assert_eq!(out.pixel(0, 0), [9; 4]);
        assert_eq!(out.pixel(1, 1), [1; 4]);
        assert_eq!(out.pixel(2, 1), [0; 4], "gap");
        assert_eq!(out.pixel(10, 1), [2; 4]);
    }
}
