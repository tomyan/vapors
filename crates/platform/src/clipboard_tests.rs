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
    let read = from_arboard(image(2, 3, 24));

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
    assert!(unreadable(&from_arboard(image(2, 3, 20))).contains("20 bytes, not 24"));
}

#[test]
fn an_empty_image_is_unreadable_naming_its_size() {
    assert!(unreadable(&from_arboard(image(0, 3, 0))).contains("0x3 pixels is empty"));
}

#[test]
fn an_image_over_the_limit_is_unreadable_before_its_bytes_are_copied() {
    assert!(
        unreadable(&from_arboard(image(8193, 8192, 0)))
            .contains("8193x8192 pixels is over the 256 MiB")
    );
}

#[test]
fn a_size_whose_byte_count_overflows_is_unreadable() {
    assert!(unreadable(&from_arboard(image(usize::MAX, 2, 0))).contains("too large to hold"));
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

static HUNG_READS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn hangs() -> ClipboardImage {
    HUNG_READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(500));
    ClipboardImage::NoImage
}

#[test]
fn a_read_past_its_deadline_is_unreadable_saying_so_and_a_second_waits_for_it() {
    // Given
    let mut reader = Reader::default();

    // When
    let first = reader.read(Duration::from_millis(50), hangs);
    let second = reader.read(Duration::from_millis(50), hangs);

    // Then: one thread started; the second paste is refused, not queued
    assert_eq!(unreadable(&first), "timed out after 50 ms");
    assert_eq!(unreadable(&second), "a clipboard read is still in progress");
    assert_eq!(HUNG_READS.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn a_read_whose_thread_panics_says_so_not_a_timeout() {
    // Given
    fn panics() -> ClipboardImage {
        panic!("a reader that fails");
    }

    // When
    let read = Reader::default().read(Duration::from_secs(5), panics);

    // Then
    assert_eq!(
        unreadable(&read),
        "the clipboard read failed (its thread panicked)"
    );
}

#[test]
fn a_finished_read_does_not_hold_up_the_next() {
    // Given
    fn answers() -> ClipboardImage {
        ClipboardImage::NoImage
    }
    let mut reader = Reader::default();

    // When
    let first = reader.read(Duration::from_secs(5), answers);
    let second = reader.read(Duration::from_secs(5), answers);

    // Then
    assert_eq!(
        (first, second),
        (ClipboardImage::NoImage, ClipboardImage::NoImage)
    );
}
