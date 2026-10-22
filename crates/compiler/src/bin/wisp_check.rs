//! `wisp-check <file>`: parses a `.wisp` file and generates its Rust,
//! reporting a refusal in `file:line:col: message` form. Native only;
//! the web has no files.

#[cfg(not(target_family = "wasm"))]
fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: wisp-check <file.wisp>");
        std::process::exit(2);
    };
    let path = std::path::Path::new(&path);
    let checked = vapors_compiler::parse_file(path)
        .and_then(|component| vapors_compiler::generate(&component).map(|_| component.name));
    match checked {
        Ok(name) => println!("ok: {} compiles to component `{name}`", path.display()),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[cfg(target_family = "wasm")]
fn main() {}
