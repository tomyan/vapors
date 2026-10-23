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

/// The page's own controls, drawn over the canvas. An app that draws
/// its own (an elements demo) hides them; see `hide_page_controls`.
pub const PAGE_CONTROL_IDS: [&str; 2] = [OPEN_ID, STATUS_ID];

/// What kind of line is being reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    /// A failure, from `report_error`.
    Error,
    /// A line that is not a failure, from `report_status`.
    Status,
}

/// The id of the element a `report` line is written to: an error goes
/// to [`ERROR_ID`] when the page has that element and to [`STATUS_ID`]
/// otherwise; a status line always goes to [`STATUS_ID`].
#[must_use]
pub fn element_for(report: Report, page_has_error_element: bool) -> &'static str {
    match report {
        Report::Error if page_has_error_element => ERROR_ID,
        Report::Error | Report::Status => STATUS_ID,
    }
}

/// Whether writing a `report` line shows the element it goes to. An
/// error is always shown, so a hidden status line that an error falls
/// back to (a page without [`ERROR_ID`]) cannot hide it; a status line
/// leaves its element as it is.
#[must_use]
pub fn shows_target(report: Report) -> bool {
    report == Report::Error
}

/// The value of `name` in a query string such as `?demo=hello&x=1`:
/// `None` when the name is absent, and the empty string for `?demo` or
/// `?demo=`. Values are taken as written; `+` and `%` escapes are not
/// decoded, so a value that needs them is refused by the caller's own
/// check rather than read wrongly here.
#[must_use]
pub fn query_value(search: &str, name: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .map(|pair| pair.split_once('=').unwrap_or((pair, "")))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.to_owned())
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
    fn an_error_goes_to_the_error_element_only_when_the_page_has_one() {
        // Given
        let cases = [
            (Report::Error, true, "error"),
            (Report::Error, false, "status"),
            (Report::Status, true, "status"),
            (Report::Status, false, "status"),
        ];

        for (report, has_error_element, expected) in cases {
            // When
            let id = element_for(report, has_error_element);

            // Then
            assert_eq!(
                id, expected,
                "{report:?} with #error present: {has_error_element}"
            );
        }
    }

    #[test]
    fn an_error_is_shown_wherever_it_is_written_and_a_status_line_is_not() {
        // Given: an error written to #status on a page without #error,
        // where the page controls (#status among them) are hidden
        let hidden = PAGE_CONTROL_IDS;

        // When
        let error_shown = shows_target(Report::Error);
        let status_shown = shows_target(Report::Status);

        // Then
        assert!(hidden.contains(&element_for(Report::Error, false)));
        assert!(error_shown);
        assert!(!status_shown);
    }
}
