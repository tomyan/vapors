//! The phase times a per-frame hook is shown (`ElementsApp::with_per_frame`):
//! the phases that do work today, by name, from the last frame.

use std::time::Duration;

use crate::frame::{FrameTimes, Phase};

/// The phases that do work today, shown by a per-frame hook.
const SHOWN: [Phase; 4] = [
    Phase::Input,
    Phase::Update,
    Phase::Accessibility,
    Phase::Draw,
];

/// `times` for the shown phases, by name; empty before the first frame.
pub(super) fn shown_times(times: Option<&FrameTimes>) -> Vec<(&'static str, Duration)> {
    times
        .map(|times| {
            SHOWN
                .iter()
                .map(|phase| (phase.name(), times.of(*phase)))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::shown_times;
    use crate::frame::{FrameTimes, Phase};

    #[test]
    fn the_shown_times_are_input_update_accessibility_and_draw_once_there_is_a_frame() {
        // Given
        let times = FrameTimes::default();

        // Then
        assert_eq!(shown_times(None), Vec::new());
        let names: Vec<&str> = shown_times(Some(&times))
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(
            names,
            [
                Phase::Input,
                Phase::Update,
                Phase::Accessibility,
                Phase::Draw
            ]
            .map(Phase::name)
        );
        assert!(
            shown_times(Some(&times))
                .iter()
                .all(|(_, time)| *time == Duration::ZERO)
        );
    }
}
