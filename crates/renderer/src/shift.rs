//! A scroll frame's shift as the pipelines take it (IU4 4c,
//! `docs/plans/iu4.md` section 3): whole device pixels, so a kept item
//! snapped at offset zero and moved by it lands on the edges a snap of
//! the moved item gives (`half_up(v) + n == half_up(v + n)` only for a
//! whole `n`). Anything else is refused where the shift is made.

/// Added to every position a kept draw records, device pixels, whole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shift([f32; 2]);

impl Shift {
    /// No shift: items on the page.
    pub const NONE: Self = Self([0.0; 2]);

    /// `[dx, dy]` device pixels.
    ///
    /// # Panics
    ///
    /// When either is not a whole, finite number: a kept item moved by
    /// it would land off the pixel grid its edges were snapped to.
    #[must_use]
    pub fn whole(shift: [f32; 2]) -> Self {
        assert!(
            shift.iter().all(|v| v.is_finite() && v.fract() == 0.0),
            "a shift of {shift:?} device pixels is not whole: kept items are snapped at offset \
             zero and moved by the shift, which keeps their edges on the pixel grid only for whole \
             pixels (docs/plans/iu4.md section 3); round the scroll offset in device pixels \
             (scroll::frame_shift) before drawing; reproduce with `cargo xtask test`"
        );
        Self(shift)
    }

    /// The shift, `[dx, dy]` device pixels.
    #[must_use]
    pub fn pixels(self) -> [f32; 2] {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Shift;

    #[test]
    fn a_whole_shift_is_kept() {
        assert_eq!(Shift::whole([3.0, -8.0]).pixels(), [3.0, -8.0]);
    }

    #[test]
    #[should_panic(expected = "a shift of [0.0, 37.5] device pixels is not whole")]
    fn a_fractional_shift_is_refused() {
        let _ = Shift::whole([0.0, 37.5]);
    }

    #[test]
    #[should_panic(expected = "is not whole")]
    fn a_shift_that_is_not_a_number_is_refused() {
        let _ = Shift::whole([f32::NAN, 0.0]);
    }

    #[test]
    #[should_panic(expected = "is not whole")]
    fn an_infinite_shift_is_refused() {
        let _ = Shift::whole([0.0, f32::INFINITY]);
    }
}
