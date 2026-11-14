//! Field chains found, counted and cut at the right places.

use super::*;

fn hops_in(source: &str) -> Vec<(usize, String)> {
    let file = syn::parse_file(source).expect("a file");
    chains(&file)
        .into_iter()
        .map(|c| (c.hops, c.text))
        .collect()
}

#[test]
fn hops_are_the_field_steps_in_a_row() {
    // Given
    let source = "fn f(s: &S) { let _ = s.a.b.c; let _ = s.a; }";

    // When / Then
    assert_eq!(
        hops_in(source),
        [(3, "s.a.b.c".to_owned()), (1, "s.a".to_owned())]
    );
}

#[test]
fn a_method_call_ends_a_chain_and_what_follows_starts_another() {
    // Given: `y.z` is 2 hops on the call's result; `s.a` 1 hop below it
    let source = "fn f(s: &S) { let _ = s.a.x().y.z; }";

    // When / Then
    assert_eq!(
        hops_in(source),
        [(2, "s.a.x().y.z".to_owned()), (1, "s.a".to_owned())]
    );
}

#[test]
fn parentheses_and_deref_are_looked_through_and_tuple_fields_are_hops() {
    assert_eq!(
        hops_in("fn f(s: &S) { let _ = (*s.p).0.q; }"),
        [(3, "(*s.p).0.q".to_owned())]
    );
}

#[test]
fn chains_inside_macro_arguments_and_closures_are_seen() {
    let source = "fn f(s: &S) { println!(\"{}\", s.a.b); let _ = || s.c.d.e; }";
    assert_eq!(
        hops_in(source),
        [(2, "s.a.b".to_owned()), (3, "s.c.d.e".to_owned())]
    );
}

#[test]
fn test_items_are_left_out() {
    let source = "#[cfg(test)] mod t { fn f(s: &S) { let _ = s.a.b.c; } }\n#[test] fn g(s: &S) { let _ = s.a.b; }\nimpl X { #[test] fn h(&self) { let _ = self.a.b; } }";
    assert_eq!(hops_in(source), []);
}

#[test]
fn a_long_chain_text_is_cut_with_an_ellipsis() {
    let source = "fn f(s: &S) { let _ = s.aaaaaaaaaa.bbbbbbbbbb.cccccccccc.dddddddddd.eeeeeeeeee.ffffffffff; }";
    let found = hops_in(source);
    assert_eq!(found[0].0, 6);
    assert_eq!(found[0].1.chars().count(), 60);
    assert!(found[0].1.ends_with("..."), "{}", found[0].1);
}
