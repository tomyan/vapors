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

#[cfg(test)]
mod tests {
    use super::*;

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
}
