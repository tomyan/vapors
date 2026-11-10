//! Compiled child controls route through document capture into the race.
use crate::app::Racer;
use vapors_elements::{Document, Viewport};
fn mounted() -> Racer {
    let racer = Racer::new();
    racer.hud.compact.set(true);
    racer
        .document
        .borrow_mut()
        .update(Viewport {
            width: 390.0,
            height: 700.0,
            scale: 1.0,
        })
        .unwrap();
    racer
}
fn centre(racer: &Racer, name: &str) -> [f32; 2] {
    let doc = racer.document.borrow();
    let node = doc
        .elements()
        .iter()
        .copied()
        .find(|n| doc.attribute(*n, "data-hold") == Some(name))
        .unwrap();
    let rect = doc.painted().unwrap().border_box(node).unwrap();
    [rect.x + rect.width / 2.0, rect.y + rect.height / 2.0]
}
#[test]
fn captured_fingers_leave_the_buttons_and_release_independently() {
    // Given
    let mut racer = mounted();
    let gas = centre(&racer, "gas");
    let right = centre(&racer, "right");
    // When
    Document::touch_pressed(&racer.document, 1, gas);
    Document::touch_pressed(&racer.document, 2, right);
    Document::touch_moved(&racer.document, 1, [190.0, 300.0]);
    racer.sync_driving_controls();
    // Then
    assert!(racer.document.borrow().touch_owned(1));
    assert!(racer.document.borrow().touch_owned(2));
    assert!(!racer.paused);
    assert_eq!(racer.driving.controls().throttle, 1.0);
    assert_eq!(racer.driving.controls().steer, 1.0);
    Document::touch_released(&racer.document, 2, [100.0, 300.0]);
    racer.sync_driving_controls();
    assert_eq!(racer.driving.controls().throttle, 1.0);
    assert_eq!(racer.driving.controls().steer, 0.0);
    Document::touch_cancelled(&racer.document, 1, [190.0, 300.0]);
    racer.sync_driving_controls();
    assert_eq!(racer.driving.controls().throttle, 0.0);
}
#[test]
fn pause_resets_held_component_state_before_resuming() {
    // Given
    let mut racer = mounted();
    Document::touch_pressed(&racer.document, 1, centre(&racer, "gas"));
    racer.sync_driving_controls();
    assert!(racer.hud.gas.get());
    // When
    racer.pause();
    racer.command("toggle").unwrap();
    racer.sync_driving_controls();
    // Then
    assert!(!racer.hud.gas.get());
    assert_eq!(racer.driving.controls().throttle, 0.0);
}

fn space() -> vapors_platform::Event {
    vapors_platform::Event::Key {
        key: " ".into(),
        code: "Space".into(),
        pressed: true,
        repeat: false,
        modifiers: vapors_platform::Modifiers::default(),
    }
}

#[test]
fn space_pauses_after_a_click_on_gas() {
    // Given
    let mut racer = mounted();
    racer.command("toggle").unwrap();
    let gas = centre(&racer, "gas");
    Document::pointer_pressed(&racer.document, gas);
    Document::pointer_released(&racer.document, gas);
    let focused = racer.document.borrow().active_element();
    assert_eq!(focused, None, "a click on a HoldButton leaves focus off it");
    // When: nothing focused uses Space, so the host has it
    racer.routed_key(&space(), vapors_app::Route::Host).unwrap();
    // Then
    assert!(racer.paused);
}
