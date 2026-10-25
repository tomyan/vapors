//! The system clipboard's image, for an app's paste on the desktop. The
//! web page reads its clipboard itself (a `paste` event or
//! `navigator.clipboard.read()`), so this is native only. An app depends
//! on [`Clipboard`]; [`SystemClipboard`] is the operating system's,
//! through `arboard`.

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
    /// The clipboard could not be read; why, from the system.
    Unreadable(String),
}

/// A clipboard an app can read an image from.
pub trait Clipboard {
    /// The image on the clipboard now.
    fn image(&mut self) -> ClipboardImage;
}

/// The operating system's clipboard. On Linux, X11 (and `XWayland`).
#[derive(Debug, Default)]
pub struct SystemClipboard;

impl Clipboard for SystemClipboard {
    fn image(&mut self) -> ClipboardImage {
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(clipboard) => clipboard,
            Err(error) => return ClipboardImage::Unreadable(error.to_string()),
        };
        match clipboard.get_image() {
            Ok(image) => from_arboard(&image),
            Err(arboard::Error::ContentNotAvailable) => ClipboardImage::NoImage,
            // On X11 a clipboard holding text answers an image request
            // with "incorrect type received" (arboard 3.7, checked by
            // examples/check_clipboard.rs); text that reads means no image.
            Err(error) if clipboard.get_text().is_ok() => {
                log::debug!("clipboard: no image ({error}); it holds text");
                ClipboardImage::NoImage
            }
            Err(error) => ClipboardImage::Unreadable(error.to_string()),
        }
    }
}

/// An image whose byte count disagrees with its size is refused rather
/// than handed on.
fn from_arboard(image: &arboard::ImageData<'_>) -> ClipboardImage {
    let (Ok(width), Ok(height)) = (u32::try_from(image.width), u32::try_from(image.height)) else {
        return ClipboardImage::Unreadable(format!(
            "an image of {}x{} pixels",
            image.width, image.height
        ));
    };
    if image.bytes.len() != image.width * image.height * 4 {
        return ClipboardImage::Unreadable(format!(
            "an image of {width}x{height} pixels with {} bytes, not {}",
            image.bytes.len(),
            image.width * image.height * 4
        ));
    }
    ClipboardImage::Image {
        width,
        height,
        rgba: image.bytes.to_vec(),
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
