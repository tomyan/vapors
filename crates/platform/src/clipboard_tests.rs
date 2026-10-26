use std::borrow::Cow;

use super::*;

fn image(width: usize, height: usize, bytes: usize) -> arboard::ImageData<'static> {
    arboard::ImageData {
        width,
        height,
        bytes: Cow::Owned(vec![7; bytes]),
    }
}

fn unreadable(read: &ClipboardImage) -> &str {
    let ClipboardImage::Unreadable(why) = read else {
        panic!("{read:?}")
    };
    why
}

#[test]
fn an_image_whose_bytes_match_its_size_is_handed_on() {
    // When
    let read = from_arboard(&image(2, 3, 24));

    // Then
    assert_eq!(
        read,
        ClipboardImage::Image {
            width: 2,
            height: 3,
            rgba: vec![7; 24]
        }
    );
}

#[test]
fn an_image_with_too_few_bytes_is_unreadable_naming_both_counts() {
    assert!(unreadable(&from_arboard(&image(2, 3, 20))).contains("20 bytes, not 24"));
}

#[test]
fn an_empty_image_is_unreadable_naming_its_size() {
    assert!(unreadable(&from_arboard(&image(0, 3, 0))).contains("0x3 pixels is empty"));
}

#[test]
fn an_image_over_the_limit_is_unreadable_before_its_bytes_are_copied() {
    assert!(
        unreadable(&from_arboard(&image(16385, 16384, 0)))
            .contains("16385x16384 pixels is over the 1024 MiB")
    );
}

#[test]
fn a_size_whose_byte_count_overflows_is_unreadable() {
    assert!(unreadable(&from_arboard(&image(usize::MAX, 2, 0))).contains("too large to hold"));
}

#[test]
fn only_no_content_and_xs_wrong_type_mean_no_image() {
    let wrong_type = arboard::Error::Unknown {
        description: "incorrect type received from clipboard".into(),
    };
    assert!(no_image(&arboard::Error::ContentNotAvailable));
    assert!(no_image(&wrong_type));
    assert!(!no_image(&arboard::Error::ConversionFailure));
    assert!(!no_image(&arboard::Error::Unknown {
        description: "X11 server connection timed out".into()
    }));
}

#[test]
fn a_read_past_its_deadline_is_unreadable_saying_so() {
    // Given
    fn hangs() -> ClipboardImage {
        std::thread::sleep(Duration::from_millis(500));
        ClipboardImage::NoImage
    }

    // When
    let read = within(Duration::from_millis(50), hangs);

    // Then
    assert_eq!(unreadable(&read), "timed out after 50 ms");
}
