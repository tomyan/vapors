//! Compiling a scanned tree into memory, then writing the generated
//! files and `wisp.rs` only when every file compiled. Every file is
//! parsed first, so each component tag is checked against the props of
//! the component it names (`PropsIndex`).

use std::path::{Path, PathBuf};

use super::tree::{Dir, scan};
use super::{Compiled, modules, path_error};
use crate::ast::Component;
use crate::{CompileError, PropsIndex, generate_with, parse_file_global, parse_file_under};

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
    let tree = scan(src)?;
    if tree.is_empty() {
        return Err(vec![path_error(
            src,
            "no .wisp files under this directory: add one, or remove the compile_dir call from build.rs",
        )]);
    }
    let generated = out.join("wisp");
    let root = (src.parent().unwrap_or(src), styles);
    let mut errors = Vec::new();
    let mut parsed = Vec::new();
    parse_tree(&tree, root, &generated, &mut parsed, &mut errors);
    let mut index = PropsIndex::default();
    errors.extend(
        parsed
            .iter()
            .filter_map(|(_, component)| index.add(component).err()),
    );
    let mut outputs = Vec::new();
    for (target, component) in &parsed {
        match generate_with(component, &index) {
            Ok(code) => outputs.push((target.clone(), code)),
            Err(err) => errors.push(err),
        }
    }
    if !errors.is_empty() {
        // parse refusals come before generation refusals; report them in
        // file order, as each file's are
        errors.sort_by(|a, b| (&a.path, a.line, a.col).cmp(&(&b.path, b.line, b.col)));
        return Err(errors);
    }
    outputs.push((out.join("wisp.rs"), modules::render(&tree, &generated)));
    write_all(&outputs)?;
    Ok(Compiled {
        inputs: tree.inputs(),
        modules: out.join("wisp.rs"),
    })
}

type Output = (PathBuf, String);

/// Every file under `dir`, parsed, beside the path its Rust goes to.
fn parse_tree(
    dir: &Dir,
    root: (&Path, Styles),
    target: &Path,
    parsed: &mut Vec<(PathBuf, Component)>,
    errors: &mut Vec<CompileError>,
) {
    for file in &dir.files {
        let component = match root {
            (root, Styles::Scoped) => parse_file_under(&file.path, root),
            (_, Styles::Global) => parse_file_global(&file.path),
        };
        match component {
            Ok(component) => parsed.push((target.join(format!("{}.rs", file.module)), component)),
            Err(err) => errors.push(err),
        }
    }
    for sub in &dir.dirs {
        parse_tree(sub, root, &target.join(&sub.module), parsed, errors);
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
