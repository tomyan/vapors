//! Corner radii as CSS Backgrounds 3 uses them: scaled down together
//! when adjacent radii would overlap (§5.5), and grown with a shadow's
//! spread (§7.1). Radii are circular, in device pixels, in the order
//! top-left, top-right, bottom-right, bottom-left.

use vapors_scene::Rect;

/// `radii` reduced so no two on one side overlap: every radius times
/// f = min(Lᵢ / Sᵢ, 1), where Lᵢ is a side's length and Sᵢ the sum of
/// the two radii on it. One factor for all four corners keeps their
/// proportions, which clamping each corner on its own does not.
#[must_use]
pub(crate) fn fitted(radii: [f32; 4], rect: Rect) -> [f32; 4] {
    let [tl, tr, br, bl] = radii;
    let sides = [
        (rect.width(), tl + tr),
        (rect.height(), tr + br),
        (rect.width(), br + bl),
        (rect.height(), bl + tl),
    ];
    let factor = sides
        .iter()
        .filter(|(_, sum)| *sum > 0.0)
        .map(|(length, sum)| (length.max(0.0) / sum).min(1.0))
        .fold(1.0_f32, f32::min);
    radii.map(|r| r * factor)
}

/// A corner radius grown by a shadow's `spread`: r + spread, except
/// that a radius smaller than a positive spread grows less, by
/// spread × (1 + (r / spread − 1)³), so a square corner stays square;
/// a negative spread shrinks it, not below zero.
#[must_use]
pub(crate) fn spread(radius: f32, spread: f32) -> f32 {
    if spread <= 0.0 || radius >= spread {
        return (radius + spread).max(0.0);
    }
    let ratio = radius / spread - 1.0;
    radius + spread * (1.0 + ratio * ratio * ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radii_that_fit_are_kept() {
        // Given: 40 on one corner of a 60x60 box
        let rect = Rect::new(0.0, 0.0, 60.0, 60.0);

        // When
        let radii = fitted([40.0, 0.0, 0.0, 0.0], rect);

        // Then
        assert_eq!(radii, [40.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn overlapping_radii_shrink_by_one_factor_for_all_corners() {
        // Given: 20 and 9999 on a 100x40 box; the right side's sum,
        // 9999 + 9999, gives the smallest factor, 40 / 19998
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);

        // When
        let radii = fitted([20.0, 9999.0, 9999.0, 20.0], rect);

        // Then: the left corners shrink with them, to almost nothing
        let factor = 40.0 / 19998.0;
        assert_eq!(
            radii,
            [
                20.0 * factor,
                9999.0 * factor,
                9999.0 * factor,
                20.0 * factor
            ]
        );
    }

    #[test]
    fn a_large_radius_on_every_corner_makes_a_pill() {
        // Given
        let rect = Rect::new(0.0, 0.0, 120.0, 32.0);

        // When
        let radii = fitted([9999.0; 4], rect);

        // Then: half the height
        assert!(radii.iter().all(|r| (r - 16.0).abs() < 1e-3), "{radii:?}");
    }

    #[test]
    fn spread_grows_a_radius_and_keeps_a_square_corner_square() {
        // Given / When / Then
        assert_eq!(spread(8.0, 4.0), 12.0, "r >= spread: r + spread");
        assert_eq!(spread(0.0, 4.0), 0.0, "a square corner stays square");
        assert_eq!(
            spread(2.0, 4.0),
            2.0 + 4.0 * (1.0 - 0.125),
            "r < spread: the cubic"
        );
        assert_eq!(spread(3.0, -5.0), 0.0, "a negative spread stops at zero");
    }
}
