//! The lost-device state and what is said about it.

use super::*;

fn destroyed() -> Lost {
    Lost {
        reason: "Destroyed".to_owned(),
        message: "Device was destroyed.".to_owned(),
    }
}

#[test]
fn on_the_web_the_fix_is_to_reload_the_page() {
    // When
    let said = sentence(&destroyed(), Target::Web, "touche");

    // Then
    assert_eq!(
        said,
        "the GPU device was lost (Destroyed: Device was destroyed.); fix: reload the page"
    );
}

#[test]
fn natively_the_fix_is_to_restart_the_app_by_name() {
    // When
    let said = sentence(&destroyed(), Target::Native, "touche");

    // Then
    assert_eq!(
        said,
        "the GPU device was lost (Destroyed: Device was destroyed.); fix: restart touche"
    );
}

#[test]
fn a_loss_without_a_message_says_so() {
    // Given
    let lost = Lost {
        reason: "Unknown".to_owned(),
        message: String::new(),
    };

    // When / Then
    assert!(sentence(&lost, Target::Web, "touche").contains("(Unknown: no message)"));
}

#[test]
fn the_state_is_empty_until_a_loss_is_recorded_and_every_clone_sees_it() {
    // Given
    let state = DeviceLost::default();
    let seen_by_the_app = state.clone();
    assert_eq!(seen_by_the_app.get(), None);

    // When
    state.record(destroyed());

    // Then
    assert_eq!(seen_by_the_app.get(), Some(destroyed()));
}

#[test]
fn the_first_loss_is_kept_when_a_second_is_reported() {
    // Given
    let state = DeviceLost::default();
    state.record(destroyed());

    // When
    state.record(Lost {
        reason: "Unknown".to_owned(),
        message: "later".to_owned(),
    });

    // Then
    assert_eq!(state.get(), Some(destroyed()));
}

#[test]
fn a_lost_device_stops_the_frame_with_the_error_that_names_it() {
    // When / Then
    assert!(stop_for(None, Target::Native, "touche").is_none());
    let error = stop_for(Some(destroyed()), Target::Native, "touche").expect("stops");
    assert_eq!(
        error.to_string(),
        sentence(&destroyed(), Target::Native, "touche")
    );
}
