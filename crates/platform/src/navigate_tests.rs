use super::{NewContext, go};
use crate::Platform;

#[test]
fn a_native_window_refuses_a_link_naming_its_href() {
    for context in [None, Some(NewContext::Tab), Some(NewContext::Window)] {
        // When
        let refused = go("?demo=counter", context).expect_err("no system browser hook");

        // Then
        assert!(refused.contains("<a href=\"?demo=counter\">"), "{refused}");
    }
}

#[test]
fn a_fragment_is_refused_saying_ids_are_not_scrolled_to() {
    // When
    let refused = Platform::detached()
        .navigate("#section")
        .expect_err("no scroll to id");

    // Then
    let message = refused.to_string();
    assert!(
        message.contains("#section") && message.contains("scroll to ids"),
        "{message}"
    );
}

#[test]
fn a_detached_platform_records_each_link_and_refuses_it() {
    // Given
    let platform = Platform::detached();

    // When
    let here = platform.navigate("?demo=a");
    let tab = platform.open_new("?demo=b", NewContext::Tab);

    // Then
    assert!(here.is_err() && tab.is_err());
    assert_eq!(
        platform.take_followed(),
        [
            ("?demo=a".to_owned(), None),
            ("?demo=b".to_owned(), Some(NewContext::Tab))
        ]
    );
}
