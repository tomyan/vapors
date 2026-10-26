//! Compiling a scanned tree into memory, then writing the generated
//! files and `wisp.rs` only when every file compiled. Every file is
//! parsed first, so each component tag is checked against the props of
//! the component it names (`PropsIndex`).

use std::path::{Path, PathBuf};

use super::tree::{Dir, scan};
use super::{Compiled, modules, path_error};
use crate::ast::Component;
use crate::{CompileError, PropsIndex, generate_checked, parse_file_global, parse_file_under};

/// Whether each component's `<style>` is scoped to its own elements,
/// or added by the app as a global sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Styles {
    /// Scoped (decision D1): each mount adds its style under its scope.
    Scoped,
    /// Global: the app adds `STYLE` as a sheet; nothing is scoped.
    Global,
}

/// Compiles every `.wisp` under `src` into `out/wisp/` and writes
/// `out/wisp.rs`.
///
/// # Errors
/// Every refusal from every file, or the directory or output that
/// could not be read or written. Nothing is written after a refusal.
pub fn compile_dir_into(src: &Path, out: &Path) -> Result<Compiled, Vec<CompileError>> {
    compile_dir_into_styled(src, out, Styles::Scoped)
}

/// [`compile_dir_into`] with each component's style `styles`.
///
/// # Errors
/// As [`compile_dir_into`].
pub fn compile_dir_into_styled(
    src: &Path,
    out: &Path,
    styles: Styles,
) -> Result<Compiled, Vec<CompileError>> {
    let tree = scan_some(src)?;
    let generated = out.join("wisp");
    let mut errors = Vec::new();
    let parsed = parse_all(
        &tree,
        (src.parent().unwrap_or(src), styles),
        &generated,
        &mut errors,
    );
    let (index, indexed) = index_all(&parsed, &mut errors);
    let (mut outputs, unchecked) = generate_all(&indexed, &index, &mut errors);
    if !errors.is_empty() {
        errors.sort_by(|a, b| (&a.path, a.line, a.col).cmp(&(&b.path, b.line, b.col)));
        return Err(errors);
    }
    outputs.push((out.join("wisp.rs"), modules::render(&tree, &generated)));
    write_all(&outputs)?;
    Ok(Compiled {
        inputs: tree.inputs(),
        modules: out.join("wisp.rs"),
        unchecked,
    })
}

type Output = (PathBuf, String);

/// The tree under `src`, refused when it holds no `.wisp` file.
fn scan_some(src: &Path) -> Result<Dir, Vec<CompileError>> {
    let tree = scan(src)?;
    if tree.is_empty() {
        return Err(vec![path_error(
            src,
            "no .wisp files under this directory: add one, or remove the compile_dir call from build.rs",
        )]);
    }
    Ok(tree)
}

/// A parsed file: where its Rust goes, its module path, the component.
struct Parsed {
    target: PathBuf,
    module: Vec<String>,
    component: Component,
}

/// Every file parsed; a file that does not parse adds its refusal.
fn parse_all(
    tree: &Dir,
    root: (&Path, Styles),
    target: &Path,
    errors: &mut Vec<CompileError>,
) -> Vec<Parsed> {
    let mut parsed = Vec::new();
    parse_tree(tree, (root, &[]), target, &mut parsed, errors);
    parsed
}

/// The props index, and the files it holds: a file whose script does
/// not parse adds its refusal once, here, and is not generated.
fn index_all<'p>(
    parsed: &'p [Parsed],
    errors: &mut Vec<CompileError>,
) -> (PropsIndex, Vec<&'p Parsed>) {
    let mut index = PropsIndex::default();
    let mut indexed = Vec::new();
    for file in parsed {
        match index.add(&file.component, &file.module) {
            Ok(()) => indexed.push(file),
            Err(err) => errors.push(err),
        }
    }
    (index, indexed)
}

/// Each indexed file's Rust, and the tags left unchecked.
fn generate_all(
    indexed: &[&Parsed],
    index: &PropsIndex,
    errors: &mut Vec<CompileError>,
) -> (Vec<Output>, Vec<String>) {
    let (mut outputs, mut unchecked) = (Vec::new(), Vec::new());
    for file in indexed {
        match generate_checked(&file.component, index) {
            Ok(generated) => {
                outputs.push((file.target.clone(), generated.code));
                unchecked.extend(generated.unchecked);
            }
            Err(refused) => errors.extend(refused),
        }
    }
    (outputs, unchecked)
}

/// Every file under `dir`, parsed, with the path its Rust goes to and
/// its module path (`dir`'s is `module`).
fn parse_tree(
    dir: &Dir,
    (root, module): ((&Path, Styles), &[String]),
    target: &Path,
    parsed: &mut Vec<Parsed>,
    errors: &mut Vec<CompileError>,
) {
    for file in &dir.files {
        let component = match root {
            (root, Styles::Scoped) => parse_file_under(&file.path, root),
            (_, Styles::Global) => parse_file_global(&file.path),
        };
        let mut path = module.to_vec();
        path.push(file.module.clone());
        match component {
            Ok(component) => parsed.push(Parsed {
                target: target.join(format!("{}.rs", file.module)),
                module: path,
                component,
            }),
            Err(err) => errors.push(err),
        }
    }
    for sub in &dir.dirs {
        let mut path = module.to_vec();
        path.push(sub.module.clone());
        parse_tree(
            sub,
            (root, &path),
            &target.join(&sub.module),
            parsed,
            errors,
        );
    }
}

fn write_all(outputs: &[Output]) -> Result<(), Vec<CompileError>> {
    for (path, text) in outputs {
        let parent = path.parent().unwrap_or(path);
        std::fs::create_dir_all(parent)
            .and_then(|()| std::fs::write(path, text))
            .map_err(|err| {
                vec![path_error(
                    path,
                    format!("cannot write the generated file: {err}"),
                )]
            })?;
    }
    Ok(())
}
