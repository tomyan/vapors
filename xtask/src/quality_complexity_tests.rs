//! clippy's count, reproduced. Each expected value was read from clippy
//! itself (`cognitive-complexity-threshold = 1` on a probe crate,
//! 2026-11-13); see the module doc for the rules.

use super::*;

fn count(source: &str) -> usize {
    let item: syn::ItemFn = syn::parse_str(source).expect("a function");
    complexity(&item.block, &item.sig.output)
}

#[test]
fn a_straight_line_body_is_one() {
    assert_eq!(count("fn f(x: i32) -> i32 { x + 1 }"), 1);
}

#[test]
fn each_if_match_with_arms_guard_while_and_for_adds_one() {
    // Given / When / Then
    let cases = [
        ("fn f(x: i32) -> i32 { if x > 0 { 1 } else { 2 } }", 2),
        (
            "fn f(x: Option<i32>) -> i32 { match x { Some(v) => v, None => 0 } }",
            2,
        ),
        (
            "fn f(x: (i32, i32)) -> i32 { match x { (a, b) => a + b } }",
            1,
        ),
        (
            "fn f(x: Option<i32>) -> i32 { match x { Some(v) if v > 0 => v, _ => 0 } }",
            3,
        ),
        ("fn f(mut x: i32) { while x > 0 { x -= 1; } }", 2),
        (
            "fn f(v: &[i32]) -> i32 { let mut s = 0; for x in v { s += x; } s }",
            2,
        ),
        (
            "fn f(mut x: i32) { loop { x -= 1; if x < 0 { break; } } }",
            2,
        ),
        ("fn f(a: bool, b: bool) -> bool { a && b }", 1),
    ];
    for (source, expected) in cases {
        assert_eq!(count(source), expected, "{source}");
    }
}

#[test]
fn a_question_mark_nets_nothing_in_a_result_function_and_half_otherwise() {
    // Given / When / Then
    assert_eq!(
        count("fn f(x: Result<i32, E>) -> Result<i32, E> { let y = x?; Ok(y) }"),
        1
    );
    assert_eq!(
        count("fn f(x: Option<i32>) -> Option<i32> { let y = x?; Some(y) }"),
        2
    );
    assert_eq!(
        count("fn f(x: Option<i32>) -> Option<i32> { let y = x?; let z = x?; Some(y + z) }"),
        2
    );
    assert_eq!(
        count(
            "fn f(x: io::Result<i32>) -> io::Result<i32> { let y = x?; if y > 0 { return Ok(1); } Ok(0) }"
        ),
        1
    );
}

#[test]
fn returns_come_off_the_count_all_for_result_and_half_otherwise() {
    assert_eq!(count("fn f(x: i32) -> i32 { if x > 0 { return 1; } 2 }"), 2);
    assert_eq!(
        count("fn f(x: i32) -> bool { if x > 0 { return true; } if x < 0 { return false; } true }"),
        2
    );
    assert_eq!(
        count(
            "fn f(x: i32) -> Result<(), ()> { if x > 0 { return Err(()); } if x < 0 { return Err(()); } Ok(()) }"
        ),
        1
    );
}

#[test]
fn let_else_is_no_branch_but_its_block_is_walked() {
    assert_eq!(
        count("fn f(x: Option<i32>) -> bool { let Some(_) = x else { return false; }; true }"),
        1
    );
    assert_eq!(
        count(
            "fn f(x: Option<i32>, y: bool, z: bool) -> bool { let Some(_) = x else { if y { return false; } if z { return false; } return true; }; true }"
        ),
        2
    );
}

#[test]
fn closures_and_nested_items_are_not_entered() {
    assert_eq!(
        count(
            "fn f(v: &[i32]) -> i32 { let g = |x: i32| if x > 0 { 1 } else if x < 0 { -1 } else { 0 }; g(1) }"
        ),
        1
    );
    assert_eq!(
        count(
            "fn f(x: i32) -> i32 { fn inner(y: i32) -> i32 { if y > 0 { 1 } else { 0 } } inner(x) }"
        ),
        1
    );
}

#[test]
fn standard_macros_add_the_branches_their_expansions_hold() {
    assert_eq!(count("fn f(x: i32) { assert!(x > 0); }"), 2);
    assert_eq!(count("fn f(x: i32) { assert_eq!(x, 1); }"), 2);
    assert_eq!(count("fn f(x: i32) { debug_assert!(x > 0); }"), 3);
    assert_eq!(
        count("fn f(x: Option<i32>) -> bool { matches!(x, Some(_)) }"),
        2
    );
    assert_eq!(count("fn f(x: i32) -> i32 { dbg!(x) }"), 1);
    assert_eq!(
        count("fn f(x: i32) -> String { format!(\"{}\", if x > 0 { 1 } else { 0 }) }"),
        2
    );
}
