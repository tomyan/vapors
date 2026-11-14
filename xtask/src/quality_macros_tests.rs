//! Reading a macro's arguments as expressions.

use super::*;

fn args_of(source: &str) -> Vec<String> {
    let mac: syn::Macro = syn::parse_str(source).expect("a macro call");
    macro_args(&mac)
        .iter()
        .map(|e| quote::ToTokens::to_token_stream(e).to_string())
        .collect()
}

#[test]
fn a_comma_separated_list_gives_every_expression() {
    // Given / When / Then
    assert_eq!(args_of("format!(\"{}\", a?.b)"), ["\"{}\"", "a ? . b"]);
}

#[test]
fn a_named_format_argument_is_an_expression_too() {
    assert_eq!(
        args_of("println!(\"{x}\", x = y.z)"),
        ["\"{x}\"", "x = y . z"]
    );
}

#[test]
fn a_macro_whose_later_argument_is_a_pattern_gives_the_leading_expressions() {
    assert_eq!(args_of("matches!(x.y, Some(ref v))"), ["x . y"]);
}

#[test]
fn a_macro_whose_tokens_start_with_no_expression_gives_nothing() {
    assert_eq!(args_of("m!(=> bad)"), Vec::<String>::new());
}
