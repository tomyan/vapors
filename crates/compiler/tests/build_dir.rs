//! `build::compile_dir_into`: every `.wisp` under a directory becomes
//! a generated file and a module in `wisp.rs`; every refusal in every
//! file is reported, and nothing is half-written.

#[path = "common/build.rs"]
mod build;

use build::{COUNTER, dirs, read, write};
use vapors_compiler::build::compile_dir_into;

#[test]
fn each_wisp_file_becomes_a_generated_file_and_a_module() {
    // Given
    let (src, out) = dirs("each_wisp_file");
    write(&src, "counter.wisp", COUNTER);
    write(&src, "text-input.wisp", COUNTER);
    write(&src, "lib.rs", "// not a component");

    // When
    let compiled = compile_dir_into(&src, &out).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then
    assert!(read(&out.join("wisp/counter.rs")).contains("pub struct Counter {"));
    assert!(read(&out.join("wisp/text_input.rs")).contains("pub struct TextInput {"));
    let modules = read(&compiled.modules);
    assert!(modules.contains("pub mod counter {"), "{modules}");
    assert!(modules.contains("pub mod text_input {"), "{modules}");
    assert!(!modules.contains("lib"), "{modules}");
}

#[test]
fn each_module_is_documented_with_its_source_file() {
    // Given
    let (src, out) = dirs("documented");
    write(&src, "counter.wisp", COUNTER);

    // When
    let compiled = compile_dir_into(&src, &out).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then
    let modules = read(&compiled.modules);
    let expected = format!(
        "/// The component compiled from `{}`.",
        src.join("counter.wisp").display()
    );
    assert!(modules.contains(&expected), "{modules}");
}

#[test]
fn a_subdirectory_becomes_a_nested_module() {
    // Given
    let (src, out) = dirs("nested");
    write(&src, "forms/text_input.wisp", COUNTER);

    // When
    let compiled = compile_dir_into(&src, &out).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then
    let modules = read(&compiled.modules);
    assert!(modules.contains("pub mod forms {"), "{modules}");
    assert!(modules.contains("pub mod text_input {"), "{modules}");
    assert!(read(&out.join("wisp/forms/text_input.rs")).contains("pub struct TextInput {"));
}

#[test]
fn inputs_list_every_file_and_directory_in_order() {
    // Given
    let (src, out) = dirs("inputs");
    write(&src, "b.wisp", COUNTER);
    write(&src, "a.wisp", COUNTER);
    write(&src, "forms/c.wisp", COUNTER);

    // When
    let compiled = compile_dir_into(&src, &out).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then
    let expected = vec![
        src.clone(),
        src.join("a.wisp"),
        src.join("b.wisp"),
        src.join("forms"),
        src.join("forms/c.wisp"),
    ];
    assert_eq!(compiled.inputs, expected);
}

#[test]
fn every_refusal_in_every_file_is_reported() {
    // Given
    let (src, out) = dirs("every_refusal");
    write(&src, "a.wisp", "<span>{1 +}</span>");
    write(&src, "b.wisp", "<blink></blink>");
    write(&src, "c.wisp", COUNTER);

    // When
    let errors = compile_dir_into(&src, &out).expect_err("two files are broken");

    // Then
    let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(errors.len(), 2, "{lines:#?}");
    assert!(
        lines[0].starts_with(&format!("{}:1:", src.join("a.wisp").display())),
        "{lines:#?}"
    );
    assert!(
        lines[1].starts_with(&format!("{}:1:", src.join("b.wisp").display())),
        "{lines:#?}"
    );
    assert!(
        !out.join("wisp.rs").exists(),
        "no modules file after a refusal"
    );
}

#[test]
fn a_directory_without_wisp_files_is_refused() {
    // Given
    let (src, out) = dirs("empty");
    write(&src, "lib.rs", "");

    // When
    let errors = compile_dir_into(&src, &out).expect_err("nothing to compile");

    // Then
    assert!(errors[0].message.contains("no .wisp files"), "{errors:?}");
}

#[test]
fn a_missing_directory_is_refused_by_name() {
    // Given
    let (src, out) = dirs("missing");

    // When
    let errors = compile_dir_into(&src.join("nope"), &out).expect_err("no such directory");

    // Then
    assert_eq!(errors[0].path, src.join("nope").display().to_string());
    assert!(
        errors[0].message.contains("cannot read the directory"),
        "{errors:?}"
    );
}

#[test]
fn two_files_making_the_same_module_are_refused() {
    // Given
    let (src, out) = dirs("same_module");
    write(&src, "text-input.wisp", COUNTER);
    write(&src, "text_input.wisp", COUNTER);

    // When
    let errors = compile_dir_into(&src, &out).expect_err("both are text_input");

    // Then
    assert!(errors[0].message.contains("text_input"), "{errors:?}");
    assert!(errors[0].message.contains("text-input.wisp"), "{errors:?}");
}

#[test]
fn a_directory_name_that_is_not_a_module_name_is_refused() {
    // Given
    let (src, out) = dirs("bad_dir");
    write(&src, "my forms/a.wisp", COUNTER);

    // When
    let errors = compile_dir_into(&src, &out).expect_err("a space is not a module name");

    // Then
    assert_eq!(errors[0].path, src.join("my forms").display().to_string());
}
