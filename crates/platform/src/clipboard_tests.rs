use std::borrow::Cow;

use super::*;

fn image(width: usize, height: usize, bytes: usize) -> arboard::ImageData<'static> {
    arboard::ImageData {
        width,
        height,
        bytes: Cow::Owned(vec![7; bytes]),
    }
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
    // When
    let read = from_arboard(&image(2, 3, 20));

    // Then
    let ClipboardImage::Unreadable(why) = read else {
        panic!("{read:?}")
    };
    assert!(why.contains("20 bytes, not 24"), "{why}");
}
