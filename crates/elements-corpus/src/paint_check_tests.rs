use super::*;

/// A `w` x `h` image of one grey.
fn grey(w: u32, h: u32, value: u8) -> Vec<u8> {
    (0..w * h)
        .flat_map(|_| [value, value, value, 255])
        .collect()
}

fn set(rgba: &mut [u8], w: u32, x: u32, y: u32, value: u8) {
    let i = ((y * w + x) * 4) as usize;
    rgba[i..i + 3].copy_from_slice(&[value; 3]);
}

fn region(rect: [u32; 4]) -> Region {
    Region {
        label: "box".to_owned(),
        rect,
    }
}

#[test]
fn identical_images_pass() {
    // Given
    let a = grey(8, 8, 100);

    // When
    let found = compare(
        &Image {
            width: 8,
            height: 8,
            rgba: &a,
        },
        &Image {
            width: 8,
            height: 8,
            rgba: &a,
        },
        &[region([0, 0, 8, 8])],
    );

    // Then
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn a_pixel_off_in_a_flat_neighbourhood_fails() {
    // Given: ours is 3 off at (4, 4), where Chromium's is flat
    let theirs = grey(8, 8, 100);
    let mut ours = theirs.clone();
    set(&mut ours, 8, 4, 4, 103);

    // When
    let found = compare(
        &Image {
            width: 8,
            height: 8,
            rgba: &ours,
        },
        &Image {
            width: 8,
            height: 8,
            rgba: &theirs,
        },
        &[],
    );

    // Then
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn pixels_off_by_most_of_the_local_contrast_count_against_a_region() {
    // Given: Chromium has an edge between 100 and 120 at x 4 (contrast
    // 20); ours differs there by 15 on three pixels (most of the
    // contrast, as a misplaced edge does), and by 8 on three more (under
    // half of it, as anti-aliasing does)
    let mut theirs = grey(10, 10, 100);
    for y in 0..10 {
        for x in 4..10 {
            set(&mut theirs, 10, x, y, 120);
        }
    }
    let mut ours = theirs.clone();
    for y in 2..5 {
        set(&mut ours, 10, 4, y, 105);
    }
    for y in 6..9 {
        set(&mut ours, 10, 4, y, 112);
    }
    let (o, t) = (
        Image {
            width: 10,
            height: 10,
            rgba: &ours,
        },
        Image {
            width: 10,
            height: 10,
            rgba: &theirs,
        },
    );

    // When
    let found = compare(&o, &t, &[region([0, 0, 10, 10])]);

    // Then: three strongly off, over the budget of two
    assert_eq!(
        found,
        ["box: 3 pixels are off by more than half the local contrast; at most 2 may be"]
    );
}

#[test]
fn small_differences_at_an_edge_are_within_a_region() {
    // Given: the same edge, ours off by under half the contrast
    let mut theirs = grey(10, 10, 100);
    for y in 0..10 {
        for x in 4..10 {
            set(&mut theirs, 10, x, y, 120);
        }
    }
    let mut ours = theirs.clone();
    for y in 0..10 {
        set(&mut ours, 10, 4, y, 112);
    }

    // When
    let found = compare(
        &Image {
            width: 10,
            height: 10,
            rgba: &ours,
        },
        &Image {
            width: 10,
            height: 10,
            rgba: &theirs,
        },
        &[region([0, 0, 10, 10])],
    );

    // Then
    assert_eq!(found, Vec::<String>::new());
}
