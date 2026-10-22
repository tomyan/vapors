//! The side-by-side image: a label strip, then rows of two renderings
//! (ours, Chromium's) with a gap between and below each.

use super::measure::Rgba;

/// Pixels between renderings, sideways and downwards.
pub const GAP: u32 = 8;

/// Width of a row of `pair`, gap included.
#[must_use]
pub fn row_width(pair: [&Rgba; 2]) -> u32 {
    pair[0].width + GAP + pair[1].width
}

/// `labels` across the top, then each row's two renderings side by side,
/// rows top to bottom.
///
/// # Panics
///
/// If the strip is narrower than the widest row, or a row's renderings
/// differ in height; caller bugs with the sizes named.
#[must_use]
pub fn compose(labels: &Rgba, rows: &[[&Rgba; 2]]) -> Rgba {
    let width = rows.iter().map(|&row| row_width(row)).max().unwrap_or(0);
    assert!(
        labels.width >= width,
        "label strip {} px wide, rows {width}",
        labels.width
    );
    let height = labels.height + rows.iter().map(|row| row[0].height + GAP).sum::<u32>();
    let mut out = Rgba {
        width: labels.width,
        height,
        data: vec![0; (labels.width * height * 4) as usize],
    };
    blit(&mut out, labels, [0, 0]);
    let mut top = labels.height;
    for [left, right] in rows {
        assert_eq!(left.height, right.height, "rendering heights");
        blit(&mut out, left, [0, top]);
        blit(&mut out, right, [left.width + GAP, top]);
        top += left.height + GAP;
    }
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
    fn the_composite_places_labels_above_and_rows_either_side_of_the_gap() {
        // Given: a 1x row of 2 px renderings and a 2x row of 4 px ones
        let labels = solid(4 + GAP + 4, 1, 9);
        let (left, right) = (solid(2, 1, 1), solid(2, 1, 2));
        let (left_2x, right_2x) = (solid(4, 2, 3), solid(4, 2, 4));

        // When
        let out = compose(&labels, &[[&left, &right], [&left_2x, &right_2x]]);

        // Then
        assert_eq!((out.width, out.height), (16, 1 + 1 + GAP + 2 + GAP));
        assert_eq!(out.pixel(0, 0), [9; 4]);
        assert_eq!(out.pixel(1, 1), [1; 4]);
        assert_eq!(out.pixel(2, 1), [0; 4], "gap");
        assert_eq!(out.pixel(10, 1), [2; 4]);
        assert_eq!(out.pixel(0, 2), [0; 4], "gap below the first row");
        assert_eq!(out.pixel(3, 2 + GAP), [3; 4]);
        assert_eq!(out.pixel(12, 2 + GAP + 1), [4; 4]);
    }
}
