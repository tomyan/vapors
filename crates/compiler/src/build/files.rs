//! Compiling a scanned tree into memory, then writing the generated
//! files and `wisp.rs` only when every file compiled.

use std::path::{Path, PathBuf};

use super::tree::{Dir, scan};
use super::{Compiled, modules, path_error};
use crate::{CompileError, compile_file_under};

/// Compiles every `.wisp` under `src` into `out/wisp/` and writes
/// `out/wisp.rs`.
///
/// # Errors
/// Every refusal from every file, or the directory or output that
/// could not be read or written. Nothing is written after a refusal.
pub fn compile_dir_into(src: &Path, out: &Path) -> Result<Compiled, Vec<CompileError>> {
    let tree = scan(src)?;
    if tree.is_empty() {
        return Err(vec![path_error(
            src,
            "no .wisp files under this directory: add one, or remove the compile_dir call from build.rs",
        )]);
    }
    let generated = out.join("wisp");
    let mut outputs = Vec::new();
    let mut errors = Vec::new();
    let root = src.parent().unwrap_or(src);
    compile_tree(&tree, root, &generated, &mut outputs, &mut errors);
    if !errors.is_empty() {
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

fn compile_tree(
    dir: &Dir,
    root: &Path,
    target: &Path,
    outputs: &mut Vec<Output>,
    errors: &mut Vec<CompileError>,
) {
    for file in &dir.files {
        match compile_file_under(&file.path, root) {
            Ok(code) => outputs.push((target.join(format!("{}.rs", file.module)), code)),
            Err(err) => errors.push(err),
        }
    }
    for sub in &dir.dirs {
        compile_tree(sub, root, &target.join(&sub.module), outputs, errors);
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
