//! rfd says why a native file dialog failed only through the `log`
//! crate, and a failure there looks exactly like a cancel otherwise. This
//! forwards rfd's warnings and errors to `report_error`. Only rfd's: the
//! other crates in the tree log for their own reasons and the app may
//! want its own logger for them, which `install` leaves in place.

use log::{Level, Log, Metadata, Record};

struct Forwarder;

/// Which records are forwarded: rfd's, at warning or above.
pub fn forwarded(target: &str, level: Level) -> bool {
    target.starts_with("rfd") && level <= Level::Warn
}

impl Log for Forwarder {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        forwarded(metadata.target(), metadata.level())
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            crate::report::report_error(&format!("file dialog: {}", record.args()));
        }
    }

    fn flush(&self) {}
}

/// Install the forwarder unless the app installed a logger first, in
/// which case rfd's messages go wherever that logger sends them.
pub fn install() {
    if log::set_logger(&Forwarder).is_ok() {
        log::set_max_level(log::LevelFilter::Warn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfd_warnings_and_errors_are_forwarded_and_nothing_else_is() {
        // Given
        let cases = [
            ("rfd::backend::xdg_desktop_portal", Level::Error, true),
            ("rfd", Level::Warn, true),
            ("rfd", Level::Info, false),
            ("wgpu_core::device", Level::Error, false),
        ];

        for (target, level, expected) in cases {
            // When
            let forwarded = forwarded(target, level);

            // Then
            assert_eq!(forwarded, expected, "{target} at {level}");
        }
    }
}
