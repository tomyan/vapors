//! The page contract on the web: the element ids the platform looks for,
//! and which element a reported line is written to. The routing is a
//! pure function so it is tested natively; `web.rs` applies it.

/// The id the canvas must have.
pub const CANVAS_ID: &str = "vapors";
/// The id of the button that opens the file picker.
pub const OPEN_ID: &str = "open";
/// The id status lines are written to, and errors when the page has no
/// [`ERROR_ID`] element.
pub const STATUS_ID: &str = "status";
/// The id errors are written to when the page has such an element; its
/// `hidden` attribute is removed when an error is written.
pub const ERROR_ID: &str = "error";
/// The id the GPU's start-up line is written to when the page has such
/// an element, so the status line stays the page's own.
pub const ADAPTER_ID: &str = "adapter";
/// The id a readout for a run by hand is written to when the page has
/// such an element: the demo bar's line, above the canvas, so it covers
/// no control of the app's.
pub const READOUT_ID: &str = "demo-line";

/// What kind of line is being reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    /// A failure, from `report_error`.
    Error,
    /// A line that is not a failure, from `report_status`.
    Status,
    /// A run by hand's readout, from `report_readout`: shown wherever it
    /// is written.
    Readout,
    /// The GPU's start-up line (adapter and limits), from `report_startup`.
    Startup,
}

/// The id of the element a `report` line is written to, given which ids
/// the page has: an error goes to [`ERROR_ID`] and the start-up line to
/// [`ADAPTER_ID`] when the page has that element, and each to
/// [`STATUS_ID`] otherwise; a status line always goes to [`STATUS_ID`].
#[must_use]
pub fn element_for(report: Report, page_has: impl Fn(&str) -> bool) -> &'static str {
    match report {
        Report::Error if page_has(ERROR_ID) => ERROR_ID,
        Report::Startup if page_has(ADAPTER_ID) => ADAPTER_ID,
        Report::Readout if page_has(READOUT_ID) => READOUT_ID,
        Report::Error | Report::Startup | Report::Status | Report::Readout => STATUS_ID,
    }
}

/// Whether writing a `report` line shows the element it goes to. An
/// error is always shown, so a hidden status line that an error falls
/// back to (a page without [`ERROR_ID`]) cannot hide it, and so is a
/// readout, which is asked for to be read; a status line leaves its
/// element as it is.
#[must_use]
pub fn shows_target(report: Report) -> bool {
    matches!(report, Report::Error | Report::Readout)
}

/// The value of `name` in a query string such as `?demo=hello&x=1`:
/// `None` when the name is absent, and the empty string for `?demo` or
/// `?demo=`. Values are taken as written; `+` and `%` escapes are not
/// decoded, so a value that needs them is refused by the caller's own
/// check rather than read wrongly here.
#[must_use]
pub fn query_value(search: &str, name: &str) -> Option<String> {
    query_values(search, name).into_iter().next()
}

/// Every value of `name` in a query string, in order, as
/// [`query_value`] reads the first: so a caller can refuse a name given
/// twice.
#[must_use]
pub fn query_values(search: &str, name: &str) -> Vec<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .map(|pair| pair.split_once('=').unwrap_or((pair, "")))
        .filter(|(key, _)| *key == name)
        .map(|(_, value)| value.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_value_is_found_by_name_among_others() {
        // Given
        let search = "?x=1&demo=hello&y";

        // When
        let (demo, y, absent) = (
            query_value(search, "demo"),
            query_value(search, "y"),
            query_value(search, "z"),
        );

        // Then
        assert_eq!(demo.as_deref(), Some("hello"));
        assert_eq!(y.as_deref(), Some(""));
        assert_eq!(absent, None);
    }

    #[test]
    fn every_value_of_a_name_given_twice_is_found_in_order() {
        assert_eq!(
            query_values("?samples=4&x=1&samples=1", "samples"),
            ["4", "1"]
        );
        assert_eq!(query_values("?x=1", "samples"), Vec::<String>::new());
    }

    #[test]
    fn each_line_goes_to_its_own_element_only_when_the_page_has_one() {
        // Given: the report, the one optional element the page has, and
        // where the line lands
        let cases = [
            (Report::Error, Some(ERROR_ID), "error"),
            (Report::Error, None, "status"),
            (Report::Error, Some(ADAPTER_ID), "status"),
            (Report::Status, Some(ERROR_ID), "status"),
            (Report::Status, Some(ADAPTER_ID), "status"),
            (Report::Status, None, "status"),
            (Report::Startup, Some(ADAPTER_ID), "adapter"),
            (Report::Startup, Some(ERROR_ID), "status"),
            (Report::Startup, None, "status"),
            (Report::Readout, Some(READOUT_ID), "demo-line"),
            (Report::Readout, Some(ERROR_ID), "status"),
            (Report::Readout, None, "status"),
        ];

        for (report, present, expected) in cases {
            // When
            let id = element_for(report, |id| Some(id) == present);

            // Then
            assert_eq!(id, expected, "{report:?} with {present:?} present");
        }
    }

    #[test]
    fn an_error_is_shown_wherever_it_is_written_and_a_status_line_is_not() {
        // Given: an error written to #status on a page without #error,
        // where the app hid the status line (`hide_status_line`)
        let hidden = [STATUS_ID];

        // When
        let error_shown = shows_target(Report::Error);
        let status_shown = shows_target(Report::Status);
        let startup_shown = shows_target(Report::Startup);

        // Then
        assert!(hidden.contains(&element_for(Report::Error, |_| false)));
        assert!(error_shown);
        assert!(!status_shown);
        assert!(!startup_shown);
    }

    #[test]
    fn a_readout_is_shown_wherever_it_is_written() {
        // Given: no demo bar, so the readout falls back to #status, which
        // the demos hide
        let to = element_for(Report::Readout, |_| false);

        // Then
        assert_eq!(to, STATUS_ID);
        assert!(shows_target(Report::Readout));
    }
}
