//! Reading `cfg` and `allow` attributes: test-only code wherever `test`
//! is named inside a `cfg`, `cfg_attr` left alone, gates and allows.

use super::*;

fn attr(text: &str) -> syn::Attribute {
    let item: syn::ItemFn = syn::parse_str(&format!("{text} fn f() {{}}")).expect("parses");
    item.attrs.into_iter().next().expect("one attribute")
}

#[test]
fn a_cfg_naming_test_anywhere_inside_is_test_code() {
    // Given / When / Then
    assert!(is_test_attr(&attr("#[cfg(test)]")));
    assert!(is_test_attr(&attr("#[cfg(all(test, feature = \"x\"))]")));
    assert!(is_test_attr(&attr("#[cfg(any(feature = \"x\", test))]")));
    assert!(is_test_attr(&attr("#[test]")));
}

#[test]
fn a_cfg_attr_or_a_cfg_without_test_is_not_test_code() {
    assert!(!is_test_attr(&attr("#[cfg_attr(test, allow(dead_code))]")));
    assert!(!is_test_attr(&attr("#[cfg(feature = \"testing\")]")));
    assert!(!is_test_attr(&attr("#[cfg(target_family = \"wasm\")]")));
}

#[test]
fn a_cfg_other_than_test_gates_and_one_naming_test_does_not() {
    assert!(is_gated(&[attr("#[cfg(target_family = \"wasm\")]")]));
    assert!(is_gated(&[attr("#[cfg(not(feature = \"x\"))]")]));
    assert!(!is_gated(&[attr("#[cfg(all(test, feature = \"x\"))]")]));
    assert!(!is_gated(&[attr("#[cfg_attr(test, allow(dead_code))]")]));
}

#[test]
fn allows_and_expects_of_either_lint_are_read_on_top_of_the_outer() {
    // Given
    let outer = Allowed {
        lines: false,
        complexity: true,
    };

    // When
    let read = Allowed::with(
        outer,
        &[attr(
            "#[expect(clippy::too_many_lines, reason = \"one table\")]",
        )],
    );

    // Then
    assert_eq!(
        read,
        Allowed {
            lines: true,
            complexity: true
        }
    );
    assert_eq!(
        Allowed::with(Allowed::default(), &[attr("#[allow(dead_code)]")]),
        Allowed::default()
    );
}
