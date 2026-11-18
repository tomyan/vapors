//! `<vapors:window onkeydown={…} onkeyup={…} />`: page-level key
//! listeners. Top level of a template only, once, no children, only
//! those two listeners; each refusal names its position and the fix.

use crate::common;

use common::{parse_err, parse_file, parse_ok};

fn refused(template: &str, line: usize, col: usize, fix: &str) {
    let err = parse_err(template);
    assert_eq!((err.line, err.col), (line, col), "{err}");
    assert!(err.message.contains(fix), "{err}");
}

#[test]
fn a_window_tag_at_the_top_level_keeps_its_listeners_apart_from_the_nodes() {
    // When
    let doc = parse_ok("<div>x</div>\n<vapors:window onkeydown={down} onkeyup={up} />");

    // Then
    let window = doc.window.expect("the window tag is kept");
    assert_eq!(doc.children.len(), 1);
    assert_eq!((window.offset, window.attributes.len()), (13, 2));
}

#[test]
fn a_window_tag_inside_an_element_is_refused_at_its_position() {
    refused(
        "<div>\n  <vapors:window onkeydown={down} />\n</div>",
        2,
        3,
        "top level of the template",
    );
}

#[test]
fn a_window_tag_inside_a_block_is_refused_at_its_position() {
    for block in [
        "{if a}\n<vapors:window onkeydown={d} />\n{/if}",
        "{for x in xs}\n<vapors:window onkeydown={d} />\n{/for}",
        "{snippet s()}\n<vapors:window onkeydown={d} />\n{/snippet}",
    ] {
        refused(block, 2, 1, "top level of the template");
    }
}

#[test]
fn a_second_window_tag_is_refused_naming_the_first() {
    refused(
        "<vapors:window onkeydown={a} />\n<vapors:window onkeyup={b} />",
        2,
        1,
        "already at 1:1",
    );
}

#[test]
fn a_window_tag_with_children_is_refused() {
    refused(
        "<vapors:window onkeydown={a}>x</vapors:window>",
        1,
        1,
        "no children",
    );
}

#[test]
fn an_attribute_other_than_a_key_listener_is_refused_at_the_attribute() {
    refused("<vapors:window onclick={a} />", 1, 16, "onkeydown");
    refused(
        "<vapors:window class=\"a\" onkeydown={a} />",
        1,
        16,
        "onkeydown",
    );
}

#[test]
fn a_listener_given_a_string_is_refused() {
    refused(
        "<vapors:window onkeydown=\"a\" />",
        1,
        16,
        "onkeydown={handler}",
    );
}

#[test]
fn a_window_tag_with_no_listener_is_refused() {
    refused("<vapors:window />", 1, 1, "onkeydown={handler}");
}

#[test]
fn any_other_vapors_tag_is_refused_at_the_top_and_inside() {
    refused("<vapors:body />", 1, 1, "the only `vapors:` tag");
    refused("<div><vapors:head /></div>", 1, 6, "the only `vapors:` tag");
}

#[test]
fn a_file_with_only_a_window_tag_parses() {
    // When
    let component = parse_file("<vapors:window onkeyup={up} />");

    // Then
    let template = component.expect("parses").template;
    assert_eq!(template.children, []);
}
