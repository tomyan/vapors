//! Snippet refusals (1-R1, R1b), each at the `{snippet}` or `{render}`
//! it concerns: parameters that borrow, lack a type or a plain name,
//! are reactive types, or take a name the generated code or the script
//! has; renders of nothing declared, of a snippet out of scope, or with
//! the wrong number of arguments; two snippets of one name in a list;
//! and recursion.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{generated, refusal};

#[test]
fn a_snippet_parameter_that_borrows_or_has_no_type_is_refused() {
    // When
    let borrowed = refusal("<div>{snippet item(name: &str)}<b>{name}</b>{/snippet}</div>");
    let untyped = refusal("<div>{snippet item(name)}<b>{name}</b>{/snippet}</div>");

    // Then
    assert_eq!((borrowed.line, borrowed.col), (1, 6), "{borrowed}");
    assert!(
        borrowed.message.contains("name: &str") && borrowed.message.contains("owned"),
        "{borrowed}"
    );
    assert!(untyped.message.contains("needs a type"), "{untyped}");
}

#[test]
fn rendering_a_snippet_nobody_declared_is_refused_naming_the_declaration() {
    // When
    let err = refusal("<div>{render footer()}</div>");

    // Then
    assert_eq!((err.line, err.col), (1, 6), "{err}");
    assert!(
        err.message
            .contains("#[prop] let footer: Snippet = Snippet::empty();"),
        "{err}"
    );
}

#[test]
fn a_parameter_whose_type_returns_or_takes_a_borrow_in_a_function_type_is_kept() {
    // When: `->` and `&` inside function types are not the parameter's own
    let code = generated(
        "{snippet s(f: fn() -> i32, g: fn(&str) -> u32, n: i32)}<b>{n}</b>{/snippet}{render s(|| 1, |s: &str| 1, 2)}",
    );

    // Then
    assert!(
        code.contains("Snippet<(fn() -> i32, fn(&str) -> u32, i32)>"),
        "{code}"
    );
}

#[test]
fn a_parameter_borrowing_inside_a_generic_type_is_refused() {
    // When
    let err = refusal("{snippet s(names: Vec<&str>)}<b>x</b>{/snippet}");

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(
        err.message.contains("names: Vec<&str>") && err.message.contains("owned"),
        "{err}"
    );
}

#[test]
fn a_signal_parameter_is_refused_since_parameters_are_already_reactive() {
    // When
    let err = refusal("{snippet s(at: Signal<u32>)}<b>{at}</b>{/snippet}");

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(
        err.message.contains("already reactive") && err.message.contains("at: u32"),
        "{err}"
    );
}

#[test]
fn a_parameter_named_like_the_generated_code_is_refused() {
    // When
    let err = refusal("{snippet s(parent: String)}<b>{parent}</b>{/snippet}");

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(err.message.contains("`parent`"), "{err}");
}

#[test]
fn a_parameter_named_like_a_script_binding_is_refused_naming_both() {
    // When
    let err = refusal(
        "<script>\nlet amount: Signal<i32> = Signal::new(0);\n</script>\n{snippet s(amount: String)}<b>{amount}</b>{/snippet}",
    );

    // Then: at the snippet, naming the script's line
    assert_eq!((err.line, err.col), (4, 1), "{err}");
    assert!(
        err.message.contains("`amount`") && err.message.contains("line 2"),
        "{err}"
    );
}

#[test]
fn a_second_snippet_of_one_name_in_a_list_is_refused_naming_the_first() {
    // When
    let err = refusal("{snippet s()}first{/snippet}\n{snippet s()}second{/snippet}");

    // Then
    assert_eq!((err.line, err.col), (2, 1), "{err}");
    assert!(
        err.message.contains("`s`") && err.message.contains("line 1"),
        "{err}"
    );
}

#[test]
fn a_render_with_the_wrong_number_of_arguments_is_refused_naming_the_snippet() {
    // When
    let none = refusal("{snippet s(n: i32)}<b>{n}</b>{/snippet}\n<p>{render s()}</p>");
    let two = refusal("{snippet s(n: i32)}<b>{n}</b>{/snippet}\n<p>{render s(1, 2)}</p>");

    // Then
    assert_eq!((none.line, none.col), (2, 4), "{none}");
    assert!(
        none.message.contains("0 arguments")
            && none.message.contains("s(n: i32)")
            && none.message.contains("line 1"),
        "{none}"
    );
    assert!(two.message.contains("2 arguments"), "{two}");
}

#[test]
fn a_render_outside_the_list_its_snippet_is_declared_in_is_refused() {
    // When
    let err = refusal("<div>{snippet s()}x{/snippet}</div><p>{render s()}</p>");

    // Then
    assert_eq!((err.line, err.col), (1, 39), "{err}");
    assert!(err.message.contains("nothing named `s`"), "{err}");
}

#[test]
fn a_snippet_rendering_itself_is_refused() {
    // When: directly, and through another
    let direct = refusal("{snippet tree(n: i32)}{if n > 0}{render tree(n - 1)}{/if}{/snippet}");
    let through = refusal("{snippet a()}{render b()}{/snippet}{snippet b()}{render a()}{/snippet}");

    // Then
    assert_eq!((direct.line, direct.col), (1, 1), "{direct}");
    assert!(direct.message.contains("`tree` renders itself"), "{direct}");
    assert!(
        through.message.contains("renders itself") && through.message.contains("`b`"),
        "{through}"
    );
}
