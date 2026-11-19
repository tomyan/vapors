//! What `for_options` asks winit for, as far as winit lets a test read it.
use crate::attributes::{class_names, for_options};
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

#[test]
fn a_borderless_option_asks_winit_for_no_decorations() {
    // When
    let plain = for_options(&options(true, None)).unwrap();
    let borderless = for_options(&options(false, None)).unwrap();

    // Then
    assert!(plain.decorations);
    assert!(!borderless.decorations);
}

#[test]
fn the_app_id_is_winits_general_name_and_its_instance() {
    assert_eq!(
        class_names(&options(true, Some("gui-bench.vapors"))),
        Some(("gui-bench.vapors".to_owned(), "gui-bench.vapors".to_owned()))
    );
    assert_eq!(class_names(&options(true, None)), None);
}
