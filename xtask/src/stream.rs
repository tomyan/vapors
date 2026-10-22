//! What of a long-running tool's output is shown: every line with
//! `--verbose`; otherwise the lines that mark a rebuild, and, once an
//! error has appeared, every line after it until the next build starts
//! or succeeds, so rustc's locations, excerpts and `help:` and trunk's
//! `Caused by:` chain are not hidden.

/// The filter's state: whether an error's lines are being passed through.
#[derive(Default)]
pub struct StreamFilter {
    after_error: bool,
}

impl StreamFilter {
    /// What to print for `line`, if anything, without trunk's timestamp.
    pub fn line<'a>(&mut self, verbose: bool, line: &'a str) -> Option<&'a str> {
        let text = without_timestamp(line);
        if is_build_boundary(text) {
            self.after_error = false;
        } else if is_error_line(text) {
            self.after_error = true;
        }
        (verbose || self.after_error || is_rebuild_line(text)).then_some(text)
    }
}

/// trunk prefixes each line with an RFC 3339 timestamp and a level;
/// the text after the level is what a reader needs.
pub fn without_timestamp(line: &str) -> &str {
    let trimmed = line.trim();
    match trimmed.split_once("  ") {
        Some((stamp, rest)) if stamp.ends_with('Z') && stamp.contains('T') => rest.trim(),
        _ => trimmed,
    }
}

/// A line that ends an error's pass-through.
fn is_build_boundary(text: &str) -> bool {
    text.contains("starting build") || text.contains("success")
}

fn is_error_line(text: &str) -> bool {
    text.contains("error")
}

fn is_rebuild_line(text: &str) -> bool {
    [
        "starting build",
        "success",
        "error",
        "warning",
        "server listening",
        "http://",
    ]
    .iter()
    .any(|mark| text.contains(mark))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filtered(lines: &[&str]) -> Vec<String> {
        let mut filter = StreamFilter::default();
        lines
            .iter()
            .filter_map(|line| filter.line(false, line).map(str::to_owned))
            .collect()
    }

    #[test]
    fn trunks_timestamp_and_level_are_cut_from_a_line() {
        // When / Then
        assert_eq!(
            without_timestamp("2026-10-22T10:28:39.440574Z  INFO ✅ success"),
            "INFO ✅ success"
        );
        assert_eq!(
            without_timestamp("   Compiling wgpu v30.0.1"),
            "Compiling wgpu v30.0.1"
        );
    }

    #[test]
    fn verbose_prints_every_line_and_quiet_prints_rebuild_lines() {
        // Given
        let compiling = "   Compiling wgpu v30.0.1";
        let success = "2026-10-22T10:28:39.440574Z  INFO ✅ success";
        let mut filter = StreamFilter::default();

        // When / Then
        assert_eq!(filter.line(false, compiling), None);
        assert_eq!(filter.line(false, success), Some("INFO ✅ success"));
        assert_eq!(filter.line(true, compiling), Some("Compiling wgpu v30.0.1"));
    }

    #[test]
    fn after_an_error_every_line_passes_until_the_next_build_starts() {
        // Given
        let lines = [
            "   Compiling demo v0.0.0",
            "error[E0425]: cannot find value `x`",
            "  --> src/main.rs:3:5",
            "   |",
            "help: a local variable with a similar name exists",
            "2026-10-22T10:28:39Z  ERROR ❌ error",
            "Caused by: cargo build failed",
            "2026-10-22T10:29:01Z  INFO 📦 starting build",
            "   Compiling demo v0.0.0",
            "2026-10-22T10:29:05Z  INFO ✅ success",
        ];

        // When
        let shown = filtered(&lines);

        // Then
        assert_eq!(
            shown,
            [
                "error[E0425]: cannot find value `x`",
                "--> src/main.rs:3:5",
                "|",
                "help: a local variable with a similar name exists",
                "ERROR ❌ error",
                "Caused by: cargo build failed",
                "INFO 📦 starting build",
                "INFO ✅ success",
            ]
        );
    }

    #[test]
    fn only_rebuild_lines_are_streamed_without_an_error() {
        // When / Then
        assert!(is_rebuild_line("INFO 📦 starting build"));
        assert!(is_rebuild_line("INFO ✅ success"));
        assert!(is_rebuild_line("ERROR error from build pipeline"));
        assert!(is_rebuild_line("INFO 📡 server listening at:"));
        assert!(!is_rebuild_line("Compiling wgpu v30.0.1"));
        assert!(!is_rebuild_line("INFO node packages []"));
    }
}
