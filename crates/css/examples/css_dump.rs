//! Print the computed style of a small fixed tree for a stylesheet file.
//!
//! ```text
//! cargo run -p vapors-css --example css_dump -- path/to/sheet.css [--dark]
//! ```
//!
//! The tree is `html > body.app > (h1#title, p.hint:hover > span)` with
//! the span's text `"hi"`, which is enough to see inheritance, state
//! flags and `var()` at work. Native only: it reads a file.

#[cfg(not(target_family = "wasm"))]
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("css_dump: {message}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_family = "wasm")]
fn main() {
    panic!("css_dump reads a file; run it natively");
}

#[cfg(not(target_family = "wasm"))]
fn run() -> Result<(), String> {
    use vapors_css::cascade::{Cascade, Environment};
    use vapors_css::selector::Element;
    use vapors_css::tree::{States, Tree};
    use vapors_css::value::ColorScheme;

    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: css_dump <sheet.css> [--dark]")?;
    let dark = args.next().as_deref() == Some("--dark");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let sheet = vapors_css::parse(&text).map_err(|e| format!("{path}: {e}"))?;

    let mut tree = Tree::new("html");
    let body = tree.append(tree.root(), "body");
    tree.add_class(body, "app");
    let title = tree.append(body, "h1");
    tree.set_id(title, "title");
    let hint = tree.append(body, "p");
    tree.add_class(hint, "hint");
    tree.set_states(hint, States::HOVER);
    let span = tree.append(hint, "span");
    tree.append_text(span, "hi");

    let env = Environment {
        viewport_width: 1280.0,
        viewport_height: 800.0,
        color_scheme: if dark {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        },
        ..Environment::default()
    };
    let cascade = Cascade::new().author(sheet);
    let styles = cascade
        .compute_tree(&tree.node(tree.root()), &env)
        .map_err(|e| format!("{path}: {e}"))?;
    for (node, style) in &styles {
        let mut label = node.tag_name().to_owned();
        if let Some(id) = node.id() {
            label.push('#');
            label.push_str(id);
        }
        println!("{label}");
        for (id, value) in style.iter() {
            println!("  {}: {}", id.name(), value.to_css());
        }
        for (name, value) in style.custom_properties() {
            println!(
                "  {name}: {}",
                vapors_css::serialize::component_values(value)
            );
        }
    }
    Ok(())
}
