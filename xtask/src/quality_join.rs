//! xtask's functions joined to clippy's counts by file and `fn` line
//! (both lints point at the function's signature). A function clippy
//! did not report has a count of "not over 1": the thresholds were 1.
//! A function allowed at its site (`#[allow(clippy::too_many_lines)]`,
//! or the complexity lint, on it or an enclosing item) is one clippy
//! says nothing about, so that count is not compared. A file clippy
//! never compiled (behind a cfg, a `#[cfg(test)]` module of an example)
//! is left out. The agreement is what holds xtask's band to clippy's
//! hard limit (`quality_clippy.rs` has the floors).

use std::collections::BTreeSet;

use crate::quality_clippy::ClippyCounts;
use crate::quality_collect::{Collected, Located, Split};
use crate::quality_stats::Sample;

/// How many functions were compared and how many agreed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Agreement {
    /// Functions compared: in files clippy compiled, not allowed at the site.
    pub compared: usize,
    /// Of those, with the same count.
    pub agreed: usize,
}

impl Agreement {
    /// Agreed functions in tenths of a percent of those compared; 1000 for none.
    pub fn per_mille(self) -> usize {
        if self.compared == 0 {
            return 1000;
        }
        self.agreed * 1000 / self.compared
    }
}

/// The join's result.
#[derive(Debug, Default)]
pub struct Joined {
    /// clippy's line counts, code and tests apart.
    pub lines: Split,
    /// clippy's complexity counts.
    pub complexity: Split,
    /// Agreement on lines.
    pub lines_agreement: Agreement,
    /// Agreement on complexity.
    pub complexity_agreement: Agreement,
    /// Each mismatch, its value the distance, labelled with both counts.
    pub mismatches: Vec<Sample>,
}

/// Join every function clippy compiled to its counts.
pub fn join(collected: &Collected, clippy: &ClippyCounts) -> Joined {
    let compiled: BTreeSet<&str> = clippy
        .lines
        .keys()
        .chain(clippy.complexity.keys())
        .map(|(path, _)| path.as_str())
        .collect();
    let mut joined = Joined::default();
    for located in &collected.functions {
        if compiled.contains(located.path.as_str()) {
            join_one(&mut joined, located, clippy);
        }
    }
    joined
}

fn join_one(joined: &mut Joined, located: &Located, clippy: &ClippyCounts) {
    let f = &located.function;
    let key = (located.path.clone(), f.line);
    let lines = Count {
        clippy: clippy.lines.get(&key).copied(),
        xtask: f.code_lines,
        allowed: f.allowed.lines,
    };
    let complexity = Count {
        clippy: clippy.complexity.get(&key).copied(),
        xtask: f.complexity,
        allowed: f.allowed.complexity,
    };
    let lines_agree = lines.count(&mut joined.lines_agreement);
    let complexity_agree = complexity.count(&mut joined.complexity_agreement);
    joined
        .lines
        .push(located.test, located.sample(lines.clippys()));
    joined
        .complexity
        .push(located.test, located.sample(complexity.clippys()));
    if !(lines_agree && complexity_agree) {
        joined.mismatches.push(Sample {
            value: lines.distance() + complexity.distance(),
            label: format!(
                "lines xtask {} clippy {}, complexity xtask {} clippy {}  {}",
                lines.xtask,
                lines.shown(),
                complexity.xtask,
                complexity.shown(),
                located.sample(0).label
            ),
        });
    }
}

/// One function's count from both sides.
struct Count {
    clippy: Option<usize>,
    xtask: usize,
    allowed: bool,
}

impl Count {
    /// clippy's count: as reported, or "not over 1", which is xtask's
    /// own count when that is 0 or 1 and 1 otherwise.
    fn clippys(&self) -> usize {
        self.clippy
            .unwrap_or(if self.xtask <= 1 { self.xtask } else { 1 })
    }

    /// Whether the two agree, counted into `agreement` unless allowed
    /// at the site; an allowed count agrees by not being compared.
    fn count(&self, agreement: &mut Agreement) -> bool {
        if self.allowed {
            return true;
        }
        agreement.compared += 1;
        let agree = self.clippys() == self.xtask;
        agreement.agreed += usize::from(agree);
        agree
    }

    fn distance(&self) -> usize {
        if self.allowed {
            0
        } else {
            self.clippys().abs_diff(self.xtask)
        }
    }

    fn shown(&self) -> String {
        if self.allowed {
            "allowed".to_owned()
        } else {
            self.clippys().to_string()
        }
    }
}

#[cfg(test)]
#[path = "quality_join_tests.rs"]
mod tests;
