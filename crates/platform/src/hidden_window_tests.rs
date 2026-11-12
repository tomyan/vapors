use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;

use super::*;

#[test]
fn a_hidden_window_waits_without_counting() {
    // Given
    let miss = Miss::Waiting("Occluded");

    // When
    let action = after_miss(miss, 59, 60);

    // Then
    assert_eq!(action, MissAction::Wait);
}

#[test]
fn a_failed_surface_counts_and_the_last_allowed_miss_fails() {
    // Given
    let miss = Miss::Failed("Lost");

    // When
    let counted = after_miss(miss, 3, 60);
    let stopped = after_miss(miss, 59, 60);

    // Then
    assert_eq!(counted, MissAction::Count(4));
    assert_eq!(stopped, MissAction::Fail("Lost"));
}

#[test]
fn events_that_can_show_the_window_end_the_wait() {
    // Given
    let showing = [
        WindowEvent::Occluded(false),
        WindowEvent::Focused(true),
        WindowEvent::Resized(PhysicalSize::new(10, 10)),
    ];
    let not_showing = [
        WindowEvent::Occluded(true),
        WindowEvent::Focused(false),
        WindowEvent::CloseRequested,
    ];

    // When / Then
    assert!(showing.iter().all(shows_window));
    assert!(!not_showing.iter().any(shows_window));
}
