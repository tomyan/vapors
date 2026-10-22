//! The file picker. Native: the system dialog through `rfd` on its own
//! thread, so the window keeps drawing. Web: a hidden `<input type=file>`
//! clicked from the user's own event, read with `File.arrayBuffer()`.
//! Either way the file comes back as [`Message::FileChosen`].
//!
//! On Linux rfd asks the XDG desktop portal over D-Bus. The only wait
//! that is not the user's is the portal's reply to the `OpenFile` call,
//! which libdbus bounds at its default 25 s (rfd passes -1, meaning
//! `DBUS_TIMEOUT_USE_DEFAULT`); measured here with no portal: the dialog
//! returns None after 25.1 s with the cause logged, and the log forwarder
//! turns that into a report. The wait for the user's choice is unbounded
//! on purpose.

use winit::event_loop::EventLoopProxy;

use crate::window::Message;

/// Open the picker; the result, if any, is sent to the event loop.
#[cfg(not(target_family = "wasm"))]
pub fn open(proxy: EventLoopProxy<Message>) {
    std::thread::spawn(move || {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .pick_file()
        else {
            return;
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match std::fs::read(&path) {
            Ok(bytes) => crate::tasks::send(&proxy, Message::FileChosen { name, bytes }),
            Err(error) => {
                crate::report::report_error(&format!("could not read {}: {error}", path.display()));
            }
        }
    });
}

/// Open the picker; the result, if any, is sent to the event loop.
#[cfg(target_family = "wasm")]
pub fn open(proxy: EventLoopProxy<Message>) {
    if let Err(message) = crate::picker_web::open(proxy) {
        crate::report::report_error(&format!("could not open the file picker: {message}"));
    }
}
