//! The uncaptured-error state and what is said about it; wgpu's handler
//! reaching it is `tests/gpu_uncaptured_error.rs`.

use super::*;

fn validation() -> Uncaptured {
    Uncaptured {
        kind: "Validation",
        message: "Dimension X value 16385 exceeds the limit of 16384".to_owned(),
    }
}

#[test]
fn on_the_web_the_sentence_names_the_error_and_says_reload() {
    // When
    let said = sentence(&validation(), Target::Web, "touche");

    // Then
    assert_eq!(
        said,
        "a GPU call failed with no error scope to catch it (Validation: Dimension X value 16385 \
         exceeds the limit of 16384); the frame it belonged to was not drawn. This is a bug in \
         touche or vapors, so report this message; fix: reload the page"
    );
}

#[test]
fn natively_the_fix_is_to_restart_the_app_by_name() {
    // When
    let said = sentence(&validation(), Target::Native, "touche");

    // Then
    assert!(said.ends_with("fix: restart touche"), "{said}");
}

#[test]
fn the_state_keeps_the_first_error_and_every_clone_sees_it() {
    // Given
    let state = UncapturedErrors::default();
    let seen_by_the_loop = state.clone();
    assert_eq!(seen_by_the_loop.get(), None);

    // When
    state.record(validation());
    state.record(Uncaptured {
        kind: "Internal",
        message: "later".to_owned(),
    });

    // Then
    assert_eq!(seen_by_the_loop.get(), Some(validation()));
}

#[test]
fn no_error_no_stop() {
    assert!(stop_for(None, Target::Native, "touche").is_none());
}
