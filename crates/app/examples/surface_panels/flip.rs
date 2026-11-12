//! When the right surface's animation starts and stops.

use std::time::Duration;

use vapors_platform::Instant;

/// How long the right surface animates, then stays stopped.
pub const PHASE: Duration = Duration::from_secs(2);

/// What the right surface's animation should do at a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flip {
    Start,
    Stop,
    Keep,
}

/// Starts on the first frame, then flips each time `PHASE` has passed
/// since the last flip.
#[derive(Default)]
pub struct Flipper {
    phase_began: Option<Instant>,
    animating: bool,
}

impl Flipper {
    pub fn at(&mut self, now: Instant) -> Flip {
        let over = self
            .phase_began
            .is_none_or(|began| now.duration_since(began) >= PHASE);
        if !over {
            return Flip::Keep;
        }
        self.phase_began = Some(now);
        self.animating = !self.animating;
        if self.animating {
            Flip::Start
        } else {
            Flip::Stop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flips_at(offsets_ms: &[u64]) -> Vec<Flip> {
        let start = Instant::now();
        let mut flipper = Flipper::default();
        offsets_ms
            .iter()
            .map(|ms| flipper.at(start + Duration::from_millis(*ms)))
            .collect()
    }

    #[test]
    fn starts_on_the_first_frame() {
        // Given / When
        let flips = flips_at(&[0]);

        // Then
        assert_eq!(flips, [Flip::Start]);
    }

    #[test]
    fn stops_at_two_seconds_and_starts_again_at_four() {
        // Given / When
        let flips = flips_at(&[0, 2000, 4000]);

        // Then
        assert_eq!(flips, [Flip::Start, Flip::Stop, Flip::Start]);
    }

    #[test]
    fn keeps_its_state_between_flips() {
        // Given / When
        let flips = flips_at(&[0, 1, 1999, 2000, 2001, 3999, 4000]);

        // Then
        use Flip::{Keep, Start, Stop};
        assert_eq!(flips, [Start, Keep, Keep, Stop, Keep, Keep, Start]);
    }
}
