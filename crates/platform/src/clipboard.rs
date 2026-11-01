//! The system clipboard's image and text, for a paste on the desktop. The
//! web page reads its clipboard itself (a `paste` event or
//! `navigator.clipboard.read()`), so this is native only. An app depends
//! on [`Clipboard`]; [`SystemClipboard`] is the operating system's,
//! through `arboard` (X11, and Wayland's data-control protocol).

use std::time::Duration;

use crate::clipboard_reader::{Answer, Reader};
pub use crate::clipboard_text::ClipboardText;

/// The largest image handed on: 256 MiB of RGBA (8192 px square). It
/// bounds what is handed on, not what is allocated: arboard decodes the
/// whole image first, so a far larger one can exhaust memory before
/// this refuses it. A real bound needs the image's size before the
/// decode, which arboard does not give.
pub const MAX_BYTES: usize = 1 << 28;

/// How long a read may take before it is refused, with [`MAX_BYTES`]
/// from the same measurement. Under Xvfb with xclip (dev box,
/// 2026-10-26, release build, load 27 to 38) an 8192x8192 PNG read in
/// 218-222 ms when compressible (4 MiB) and 415-467 ms as noise
/// (256 MiB moved). arboard 3.7 gives up on an X11 owner after 4 s
/// (`LONG_TIMEOUT_DUR`, src/platform/linux/x11.rs) and then answers
/// `ContentNotAvailable`, as for an empty clipboard, so this must come
/// first: 3 s, about 6 times the slowest. Not measured on Wayland,
/// macOS or Windows.
pub const DEADLINE: Duration = Duration::from_secs(3);

/// What the clipboard held when asked for an image: three answers, so a
/// clipboard that cannot be read is never taken for an empty one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardImage {
    /// An image, as 8-bit RGBA rows, top row first.
    Image {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// `width * height * 4` bytes.
        rgba: Vec<u8>,
    },
    /// The clipboard holds no image (empty, or text only).
    NoImage,
    /// The clipboard could not be read, or held an image that cannot be
    /// used; why.
    Unreadable(String),
}

/// A clipboard an app can read an image or text from.
pub trait Clipboard {
    /// The image on the clipboard now. May block for up to its deadline
    /// ([`DEADLINE`] for [`SystemClipboard`]); call it off the frame loop.
    fn image(&mut self) -> ClipboardImage;

    /// The text on the clipboard now, for a paste into a text field. May
    /// block for up to its deadline, as [`Clipboard::image`].
    fn text(&mut self) -> ClipboardText;
}

/// The operating system's clipboard.
#[derive(Debug, Default)]
pub struct SystemClipboard {
    reader: Reader<ClipboardImage>,
    text_reader: Reader<ClipboardText>,
}

impl Clipboard for SystemClipboard {
    fn image(&mut self) -> ClipboardImage {
        self.reader.read(DEADLINE, read)
    }

    fn text(&mut self) -> ClipboardText {
        self.text_reader.read(DEADLINE, crate::clipboard_text::read)
    }
}

impl Answer for ClipboardImage {
    fn unreadable(why: String) -> Self {
        ClipboardImage::Unreadable(why)
    }
}

fn read() -> ClipboardImage {
    let image = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_image());
    match image {
        Ok(image) => from_arboard(image),
        Err(error) if no_image(&error) => ClipboardImage::NoImage,
        Err(error) => ClipboardImage::Unreadable(error.to_string()),
    }
}

/// The clipboard holds no image: arboard says so, or (X11) the owner
/// answered the image request with another type, which arboard 3.7
/// reports only by this description (src/platform/linux/x11.rs:558;
/// `examples/check_clipboard` fails if it changes).
fn no_image(error: &arboard::Error) -> bool {
    match error {
        arboard::Error::ContentNotAvailable => true,
        arboard::Error::Unknown { description } => {
            description == "incorrect type received from clipboard"
        }
        _ => false,
    }
}

/// An image handed on only when its size is usable and its bytes match.
fn from_arboard(image: arboard::ImageData<'_>) -> ClipboardImage {
    let size = format!("{}x{}", image.width, image.height);
    let expected = image
        .width
        .checked_mul(image.height)
        .and_then(|pixels| pixels.checked_mul(4));
    let (Ok(width), Ok(height), Some(expected)) = (
        u32::try_from(image.width),
        u32::try_from(image.height),
        expected,
    ) else {
        return ClipboardImage::Unreadable(format!(
            "an image of {size} pixels is too large to hold"
        ));
    };
    if expected == 0 {
        return ClipboardImage::Unreadable(format!("an image of {size} pixels is empty"));
    }
    if expected > MAX_BYTES {
        return ClipboardImage::Unreadable(format!(
            "an image of {size} pixels is over the {} MiB read",
            MAX_BYTES >> 20
        ));
    }
    if image.bytes.len() != expected {
        return ClipboardImage::Unreadable(format!(
            "an image of {size} pixels with {} bytes, not {expected}",
            image.bytes.len()
        ));
    }
    ClipboardImage::Image {
        width,
        height,
        rgba: image.bytes.into_owned(),
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
