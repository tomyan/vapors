//! `wisp-check <file>`: parses a `.wisp` file and reports refusals in
//! `file:line:col: message` form. Native only; the web has no files.

#[cfg(not(target_family = "wasm"))]
fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: wisp-check <file.wisp>");
        std::process::exit(2);
    };
    match vapors_compiler::parse_file(std::path::Path::new(&path)) {
        Ok(component) => println!(
            "ok: {path} parses as component `{}` with {} top-level nodes",
            component.name,
            component.template.children.len()
        ),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[cfg(target_family = "wasm")]
fn main() {}
