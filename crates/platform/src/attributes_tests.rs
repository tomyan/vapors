//! What `for_options` asks winit for, read back from winit's own attributes
//! (its `Debug` shows the app id, which has no getter), and what it refuses.
use crate::attributes::for_options;
use crate::session::Session;
use crate::window::WindowOptions;

fn options(decorations: bool, app_id: Option<&str>) -> WindowOptions {
    WindowOptions {
        app_name: String::new(),
        title: "t".into(),
        width: 10,
        height: 10,
        decorations,
        app_id: app_id.map(str::to_owned),
    }
}

fn refusal(options: &WindowOptions, session: Session) -> String {
    for_options(options, session).unwrap_err().to_string()
}

#[test]
fn a_borderless_option_asks_winit_for_no_decorations() {
    // When
    let plain = for_options(&options(true, None), Session::Wayland).unwrap();
    let borderless = for_options(&options(false, None), Session::Wayland).unwrap();

    // Then
    assert!(plain.decorations);
    assert!(!borderless.decorations);
}

#[cfg(target_os = "linux")]
#[test]
fn the_app_id_reaches_winits_attributes_as_name_and_instance() {
    // When
    let named = for_options(&options(true, Some("gui-bench.vapors")), Session::Wayland).unwrap();
    let plain = for_options(&options(true, None), Session::Wayland).unwrap();

    // Then
    let shown = format!("{named:?}");
    assert!(shown.contains(r#"general: "gui-bench.vapors""#), "{shown}");
    assert!(shown.contains(r#"instance: "gui-bench.vapors""#), "{shown}");
    assert!(!format!("{plain:?}").contains("gui-bench"));
}

#[test]
fn x11_refuses_an_app_id_naming_the_session_the_option_and_the_fix() {
    // When
    let message = refusal(&options(true, Some("gui-bench.vapors")), Session::X11);

    // Then
    assert!(
        message.contains("X11") && message.contains("`app_id`"),
        "{message}"
    );
    assert!(
        message.contains("Fix:") && message.contains("Reproduce:"),
        "{message}"
    );
}

#[test]
fn x11_refuses_borderless_naming_the_session_the_option_and_the_fix() {
    // When
    let message = refusal(&options(false, None), Session::X11);

    // Then
    assert!(
        message.contains("X11") && message.contains("`decorations: false`"),
        "{message}"
    );
    assert!(message.contains("Fix:"), "{message}");
}

#[test]
fn desktop_platforms_refuse_an_app_id_but_honour_borderless() {
    assert!(for_options(&options(false, None), Session::Desktop).is_ok());
    let message = refusal(&options(true, Some("id")), Session::Desktop);
    assert!(message.contains("`app_id`"), "{message}");
}

#[test]
fn x11_accepts_the_defaults() {
    assert!(for_options(&options(true, None), Session::X11).is_ok());
}
