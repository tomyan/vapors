//! Every Rust file under the workspace measured, sorted into the
//! samples the signals and the distributions print. A file is test code
//! by its path (`*_tests.rs`, `tests.rs`, under a `tests/` directory);
//! inside a code file, `#[cfg(test)]` items and `#[test]` functions are
//! test code too. Files under `[files.generated]` are left out.

use std::collections::BTreeMap;
use std::path::Path;

use crate::files_walk::{count_lines, workspace_files};
use crate::quality_measure::{Function, measure_source};
use crate::quality_stats::Sample;

/// The measurement, code and tests apart.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Collected {
    /// Lines per file.
    pub files: Split,
    /// Lines of code per function body.
    pub function_lines: Split,
    /// Complexity per function.
    pub complexity: Split,
    /// Hops per field chain, code only.
    pub chains: Vec<Sample>,
    /// Every function with where it is, for the join against clippy.
    pub functions: Vec<Located>,
}

/// A function and the file it is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    /// Path from the workspace root.
    pub path: String,
    /// The function.
    pub function: Function,
    /// In test code, by path or by attribute.
    pub test: bool,
}

impl Located {
    /// A sample of `value` labelled `path:line name`.
    pub fn sample(&self, value: usize) -> Sample {
        Sample {
            value,
            label: format!(
                "{}:{} {}",
                self.path, self.function.line, self.function.name
            ),
        }
    }
}

/// Samples from code and from tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Split {
    /// From code.
    pub code: Vec<Sample>,
    /// From tests.
    pub tests: Vec<Sample>,
}

impl Split {
    /// Add `sample` to tests or to code.
    pub fn push(&mut self, test: bool, sample: Sample) {
        if test {
            self.tests.push(sample);
        } else {
            self.code.push(sample);
        }
    }

    /// Code and tests together.
    pub fn all(&self) -> Vec<Sample> {
        self.code.iter().chain(&self.tests).cloned().collect()
    }
}

/// Measure every `.rs` file under `root` not in `generated`.
///
/// # Errors
///
/// A directory that cannot be read, or a file that cannot be read or
/// parsed, named with the fix.
pub fn collect(root: &Path, generated: &BTreeMap<String, String>) -> Result<Collected, String> {
    let mut collected = Collected::default();
    for path in workspace_files(root)? {
        let shown = path.display().to_string();
        let rust = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rs"));
        if !rust || generated.contains_key(&shown) {
            continue;
        }
        add_file(&mut collected, root, &shown)?;
    }
    Ok(collected)
}

fn add_file(collected: &mut Collected, root: &Path, path: &str) -> Result<(), String> {
    let full = root.join(path);
    let text = std::fs::read_to_string(&full).map_err(|e| format!("{path}: {e}"))?;
    let measure = measure_source(&text).map_err(|e| {
        format!("{path}: {e}; fix: make it parse; a hand-written file is never listed as generated")
    })?;
    let test_file = is_test_path(path);
    collected.files.push(
        test_file,
        Sample {
            value: count_lines(&full)?,
            label: path.to_owned(),
        },
    );
    for function in measure.functions {
        let located = Located {
            path: path.to_owned(),
            test: test_file || function.test,
            function,
        };
        collected
            .function_lines
            .push(located.test, located.sample(located.function.code_lines));
        collected
            .complexity
            .push(located.test, located.sample(located.function.complexity));
        collected.functions.push(located);
    }
    if !test_file {
        collected
            .chains
            .extend(measure.chains.into_iter().map(|chain| Sample {
                value: chain.hops,
                label: format!("{path}:{} {}", chain.line, chain.text),
            }));
    }
    Ok(())
}

/// Test code by path.
pub fn is_test_path(path: &str) -> bool {
    path.ends_with("_tests.rs") || path.ends_with("/tests.rs") || path.contains("/tests/")
}

#[cfg(test)]
#[path = "quality_collect_tests.rs"]
mod tests;
