//! `wisp-check [--global] <file>`: parses a `.wisp` file as its build
//! does (`compile_dir`, or `compile_dir_global` with `--global`) and
//! generates its Rust, reporting a refusal in `file:line:col: message`
//! form. Native only; the web has no files.

#[cfg(not(target_family = "wasm"))]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (global, path) = match args.as_slice() {
        [flag, path] if flag == "--global" => (true, path),
        [path] if path != "--global" => (false, path),
        _ => {
            eprintln!("usage: wisp-check [--global] <file.wisp>");
            std::process::exit(2);
        }
    };
    let path = std::path::Path::new(path);
    let (reading, parsed) = if global {
        ("global style", vapors_compiler::parse_file_global(path))
    } else {
        ("scoped style", vapors_compiler::parse_file(path))
    };
    let checked =
        parsed.and_then(|component| vapors_compiler::generate(&component).map(|_| component.name));
    match checked {
        Ok(name) => println!(
            "ok: {} compiles to component `{name}` ({reading})",
            path.display()
        ),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

// Relaxed guard: this bin is still built for wasm, because cargo cannot
// leave a bin out of one target and the workspace's wasm clippy builds
// every target. It refuses rather than exiting quietly. What would
// retire it: moving wisp-check into a native-only package, or cargo
// gaining per-target bins.
#[cfg(target_family = "wasm")]
fn main() {
    panic!(
        "wisp-check reads files and runs on native targets only: run it with `cargo run -p vapors-compiler --bin wisp-check -- [--global] <file>`"
    );
}
