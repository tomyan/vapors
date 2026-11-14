//! One file's functions: names, lines, test flags; a refusal with its line.

use super::*;

const SOURCE: &str = "\
/// Free.
pub fn free(x: i32) -> i32 {
    // a comment
    x + 1
}

impl Thing {
    fn method(&self) -> bool {
        if self.a { true } else { false }
    }
}

trait Tr {
    fn with_default(&self) {
        println!(\"d\");
    }
    fn without(&self);
}

#[cfg(test)]
mod tests {
    fn helper() {}

    #[test]
    fn a_test() {
        assert!(true);
    }
}
";

fn functions() -> Vec<Function> {
    measure_source(SOURCE).expect("parses").functions
}

#[test]
fn every_function_with_a_body_is_named_with_its_impl_type() {
    // Given / When
    let names: Vec<String> = functions().into_iter().map(|f| f.name).collect();

    // Then
    assert_eq!(
        names,
        [
            "free",
            "Thing::method",
            "Tr::with_default",
            "helper",
            "a_test"
        ]
    );
}

#[test]
fn lines_and_complexity_are_clippys_counts() {
    // Given / When
    let found = functions();

    // Then: the comment line is not code; the `if` is one branch
    assert_eq!(
        (found[0].line, found[0].code_lines, found[0].complexity),
        (2, 1, 1)
    );
    assert_eq!(
        (found[1].line, found[1].code_lines, found[1].complexity),
        (8, 1, 2)
    );
    assert_eq!(
        found[4].complexity, 2,
        "a #[test] is counted like any function"
    );
}

#[test]
fn test_code_is_flagged_under_cfg_test_and_on_test_functions() {
    let tests: Vec<bool> = functions().into_iter().map(|f| f.test).collect();
    assert_eq!(tests, [false, false, false, true, true]);
}

#[test]
fn allows_on_the_function_or_an_enclosing_item_are_read() {
    // Given
    let source = "\
#[allow(clippy::cognitive_complexity)]
mod m {
    #[allow(clippy::too_many_lines)] // one table
    pub fn both() {}
    pub fn complexity_only() {}
}
fn neither() {}
";

    // When
    let allowed: Vec<(String, bool, bool)> = measure_source(source)
        .expect("parses")
        .functions
        .into_iter()
        .map(|f| (f.name, f.allowed.lines, f.allowed.complexity))
        .collect();

    // Then
    assert_eq!(
        allowed,
        [
            ("both".to_owned(), true, true),
            ("complexity_only".to_owned(), false, true),
            ("neither".to_owned(), false, false),
        ]
    );
}

#[test]
fn a_cfg_other_than_test_gates_the_function_and_what_it_encloses() {
    // Given
    let source = "\
#[cfg(target_family = \"wasm\")]
mod web {
    pub fn inside() {}
}
#[cfg(not(feature = \"x\"))]
fn direct() {}
#[cfg(test)]
fn test_only() {}
fn plain() {}
";

    // When
    let gated: Vec<(String, bool)> = measure_source(source)
        .expect("parses")
        .functions
        .into_iter()
        .map(|f| (f.name, f.gated))
        .collect();

    // Then
    assert_eq!(
        gated,
        [
            ("inside".to_owned(), true),
            ("direct".to_owned(), true),
            ("test_only".to_owned(), false),
            ("plain".to_owned(), false),
        ]
    );
}

#[test]
fn a_file_that_is_not_rust_is_refused_naming_the_line() {
    // Given
    let source = "fn ok() {}\nfn broken( {\n";

    // When
    let error = measure_source(source).expect_err("refused");

    // Then
    assert!(error.starts_with("line 2:"), "{error}");
    assert!(error.contains("whole Rust files"), "{error}");
}
