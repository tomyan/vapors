//! Scanning a source directory: the `.wisp` files and the
//! subdirectories that hold some, each with its module name.

use std::path::{Path, PathBuf};

use super::path_error;
use crate::{CompileError, component_name};

/// A directory that holds `.wisp` files, directly or below.
pub(crate) struct Dir {
    pub path: PathBuf,
    pub module: String,
    pub files: Vec<WispFile>,
    pub dirs: Vec<Dir>,
}

/// A `.wisp` file and its module name (`text-input.wisp` is `text_input`).
pub(crate) struct WispFile {
    pub path: PathBuf,
    pub module: String,
}

impl Dir {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.dirs.iter().all(Dir::is_empty)
    }

    /// This directory, its subdirectories and its files, sorted.
    pub fn inputs(&self) -> Vec<PathBuf> {
        let mut inputs = vec![self.path.clone()];
        inputs.extend(self.files.iter().map(|file| file.path.clone()));
        inputs.extend(self.dirs.iter().flat_map(Dir::inputs));
        inputs.sort();
        inputs
    }
}

/// Scans `src`; the root has no module name of its own.
pub(crate) fn scan(src: &Path) -> Result<Dir, Vec<CompileError>> {
    scan_dir(src, String::new())
}

fn scan_dir(path: &Path, module: String) -> Result<Dir, Vec<CompileError>> {
    let mut dir = Dir {
        path: path.to_path_buf(),
        module,
        files: Vec::new(),
        dirs: Vec::new(),
    };
    let mut errors = Vec::new();
    for entry in entries(path)? {
        if let Err(mut refused) = add_entry(&mut dir, entry) {
            errors.append(&mut refused);
        }
    }
    errors.extend(collision(&dir));
    if errors.is_empty() {
        Ok(dir)
    } else {
        Err(errors)
    }
}

fn add_entry(dir: &mut Dir, entry: PathBuf) -> Result<(), Vec<CompileError>> {
    if entry.is_dir() {
        let sub = scan_dir(&entry, String::new())?;
        if !sub.is_empty() {
            let module = dir_module(&entry).map_err(|err| vec![err])?;
            dir.dirs.push(Dir { module, ..sub });
        }
    } else if entry.extension().is_some_and(|ext| ext == "wisp") {
        let module = snake_case(&component_name(&entry).map_err(|err| vec![err])?);
        dir.files.push(WispFile {
            path: entry,
            module,
        });
    }
    Ok(())
}

/// The directory's entries, sorted so output and errors are stable.
fn entries(path: &Path) -> Result<Vec<PathBuf>, Vec<CompileError>> {
    let read = std::fs::read_dir(path).map_err(|err| {
        vec![path_error(
            path,
            format!("cannot read the directory: {err}"),
        )]
    })?;
    let mut entries = Vec::new();
    for entry in read {
        let entry = entry.map_err(|err| {
            vec![path_error(
                path,
                format!("cannot read the directory: {err}"),
            )]
        })?;
        entries.push(entry.path());
    }
    entries.sort();
    Ok(entries)
}

fn dir_module(path: &Path) -> Result<String, CompileError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if syn::parse_str::<syn::Ident>(name).is_ok() {
        return Ok(name.to_string());
    }
    Err(path_error(
        path,
        format!(
            "the directory name `{name}` is not a Rust module name, and its .wisp files become module `{name}`: rename it with letters, digits and `_`"
        ),
    ))
}

/// Two entries that would make the same module.
fn collision(dir: &Dir) -> Option<CompileError> {
    let mut seen: Vec<(&str, &Path)> = Vec::new();
    let named = dir
        .files
        .iter()
        .map(|file| (file.module.as_str(), file.path.as_path()));
    let named = named.chain(
        dir.dirs
            .iter()
            .map(|sub| (sub.module.as_str(), sub.path.as_path())),
    );
    for (module, path) in named {
        if let Some((_, first)) = seen.iter().find(|(name, _)| *name == module) {
            return Some(path_error(
                path,
                format!(
                    "`{}` and `{}` both become module `{module}`: rename one",
                    first.display(),
                    path.display()
                ),
            ));
        }
        seen.push((module, path));
    }
    None
}

/// `TextInput` is `text_input`.
fn snake_case(name: &str) -> String {
    let mut snake = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.extend(ch.to_lowercase());
    }
    snake
}
