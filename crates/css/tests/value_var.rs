//! `var()` substitution. Translated from sumi's `runtime/css/vars_test.go`;
//! sumi expanded strings and collapsed cycles and unresolved references to
//! "" (dropping the property), vapors substitutes tokens and refuses both,
//! naming the variable.

use std::collections::BTreeMap;

use vapors_css::ast::ComponentValue;
use vapors_css::serialize::component_values;
use vapors_css::value::{VarError, substitute_vars};

fn tokens(text: &str) -> Vec<ComponentValue> {
    let sheet = vapors_css::parse(&format!("a {{ --v: {text}; }}")).expect("parses");
    sheet.rules[0].declarations[0].value.clone()
}

fn expand(value: &str, vars: &[(&str, &str)]) -> Result<String, VarError> {
    let table: BTreeMap<String, Vec<ComponentValue>> = vars
        .iter()
        .map(|(k, v)| ((*k).to_owned(), tokens(v)))
        .collect();
    let lookup = |name: &str| table.get(name).map(Vec::as_slice);
    substitute_vars(&tokens(value), &lookup).map(|v| component_values(&v))
}

fn ok(value: &str, vars: &[(&str, &str)]) -> String {
    expand(value, vars).unwrap_or_else(|e| panic!("{value}: {e}"))
}

#[test]
fn expand_var_simple() {
    assert_eq!(ok("var(--accent)", &[("--accent", "cyan")]), "cyan");
}

#[test]
fn expand_var_inside_value() {
    assert_eq!(ok("1px var(--w) 2px", &[("--w", "20px")]), "1px 20px 2px");
}

#[test]
fn expand_var_fallback() {
    assert_eq!(ok("var(--missing, red)", &[]), "red");
}

#[test]
fn expand_var_fallback_with_function() {
    assert_eq!(ok("var(--missing, rgb(1, 2, 3))", &[]), "rgb(1, 2, 3)");
}

#[test]
fn expand_var_nested_in_fallback() {
    assert_eq!(ok("var(--a, var(--b))", &[("--b", "blue")]), "blue");
}

#[test]
fn expand_var_value_references_var() {
    assert_eq!(
        ok("var(--a)", &[("--a", "var(--b)"), ("--b", "green")]),
        "green"
    );
}

#[test]
fn expand_var_cycle_is_refused_naming_the_variables() {
    // When
    let err = expand("var(--a)", &[("--a", "var(--b)"), ("--b", "var(--a)")]).unwrap_err();

    // Then
    assert_eq!(
        err,
        VarError::Cycle(vec!["--a".to_owned(), "--b".to_owned(), "--a".to_owned()])
    );
    assert!(err.to_string().contains("--a -> --b -> --a"), "{err}");
}

#[test]
fn expand_var_unresolved_without_fallback_is_refused() {
    // When
    let err = expand("var(--nope)", &[]).unwrap_err();

    // Then
    assert_eq!(err, VarError::Unresolved("--nope".to_owned()));
}

#[test]
fn expand_var_plain_value_untouched() {
    assert_eq!(ok("space-between", &[("--x", "1")]), "space-between");
}

#[test]
fn var_inside_a_function_argument_is_substituted() {
    assert_eq!(ok("rgb(var(--r) 0 0)", &[("--r", "255")]), "rgb(255 0 0)");
}

#[test]
fn var_with_a_malformed_name_is_refused() {
    assert!(matches!(
        expand("var(accent)", &[]).unwrap_err(),
        VarError::BadName(_)
    ));
}

#[test]
fn empty_fallback_substitutes_nothing() {
    assert_eq!(ok("var(--missing,)", &[]), "");
}
