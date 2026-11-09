//! Compiled HUD layout and state transitions.
use super::app::Racer;
use vapors_elements::Viewport;
#[test]
fn compiled_hud_accepts_desktop_and_phone_layouts() {
    // Given
    let racer = Racer::new();
    // When / Then
    for (width, height) in [(1100.0, 720.0), (390.0, 700.0), (320.0, 568.0)] {
        racer.hud.compact.set(width < 760.0);
        for settings in [false, true] {
            racer.hud.settings.set(settings);
            let mut doc = racer.document.borrow_mut();
            doc.update(Viewport {
                width,
                height,
                scale: 1.0,
            })
            .unwrap();
            for node in doc.elements() {
                if doc.attribute(node, "data-drive").is_some() {
                    let rect = doc.painted().unwrap().border_box(node).unwrap();
                    assert!(rect.width >= 44.0 && rect.height >= 44.0);
                    assert!(rect.x >= 0.0 && rect.x + rect.width <= width);
                }
            }
        }
    }
}
#[test]
fn pause_clears_held_inputs_and_restart_resets_camera() {
    // Given
    let mut racer = Racer::new();
    racer.command("toggle").unwrap();
    racer.driving.key("KeyW", true);
    racer.camera.zoom = 0.7;
    // When
    racer.pause();
    // Then
    assert!(racer.paused);
    assert_eq!(racer.driving.controls().throttle, 0.0);
    racer.command("restart").unwrap();
    assert_eq!(racer.camera.zoom, 1.0);
    assert!(racer.command("unknown").is_err());
}
#[test]
fn simultaneous_touch_controls_release_independently() {
    // Given
    let mut driving = super::driving::Driving::default();
    // When
    driving.hold("touch-1".into(), Some("gas".into()));
    driving.hold("touch-2".into(), Some("left".into()));
    // Then
    assert_eq!(driving.controls().throttle, 1.0);
    assert_eq!(driving.controls().steer, -1.0);
    driving.hold("touch-2".into(), None);
    assert_eq!(driving.controls().steer, 0.0);
    assert_eq!(driving.controls().throttle, 1.0);
}

#[test]
fn invalid_tuning_keeps_the_run_paused_and_can_be_corrected() {
    // Given
    let mut racer = Racer::new();
    racer.hud.steering.set(11.25);
    // When
    racer.command("toggle").unwrap();
    // Then
    assert!(racer.paused);
    assert!(racer.hud.settings.get());
    racer.hud.steering.set(1.25);
    racer.command("toggle").unwrap();
    assert!(!racer.paused);
}

#[test]
fn a_slow_frame_does_not_pause_or_drop_held_throttle() {
    // Given
    let mut racer = Racer::new();
    racer.command("toggle").unwrap();
    racer.driving.key("KeyW", true);
    // When
    racer.advance_elapsed(0.5).unwrap();
    // Then
    assert!(!racer.paused);
    assert_eq!(racer.driving.controls().throttle, 1.0);
    assert!(racer.race.speed > 0.0);
    assert!(racer.race.elapsed <= 0.051);
}

#[test]
fn fps_uses_wall_time_including_slow_frames_and_resets_on_pause() {
    // Given
    let mut racer = Racer::new();
    racer.command("toggle").unwrap();
    // When
    for _ in 0..4 {
        racer.advance_elapsed(0.125).unwrap();
    }
    // Then
    assert_eq!(racer.hud.fps.get(), "8");
    racer.pause();
    assert_eq!(racer.hud.fps.get(), "—");
}

#[test]
fn landscape_stats_leave_space_for_driving() {
    // Given
    let racer = Racer::new();
    racer.hud.short.set(true);
    racer.hud.paused.set(false);
    let mut doc = racer.document.borrow_mut();
    // When
    doc.update(Viewport {
        width: 740.0,
        height: 250.0,
        scale: 1.0,
    })
    .unwrap();
    // Then
    let top = doc
        .elements()
        .iter()
        .copied()
        .find(|n| doc.attribute(*n, "class") == Some("top"))
        .unwrap();
    let bounds = doc.painted().unwrap().border_box(top).unwrap();
    assert!(
        bounds.y + bounds.height <= 52.0,
        "stats must leave the track visible: {bounds:?}"
    );
}
