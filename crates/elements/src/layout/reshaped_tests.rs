use super::Shared;

/// `0..len` with `count` new slots (from 1000) put in at `at`.
fn inserted(len: usize, at: usize, count: usize) -> (Vec<usize>, Vec<usize>) {
    let old: Vec<usize> = (0..len).collect();
    let mut new = old.clone();
    new.splice(at..at, 1000..1000 + count);
    (old, new)
}

#[test]
fn the_shared_head_and_tail_are_found_around_an_insert_at_every_chunk_edge() {
    // Given: inserts at and around the chunk edges, and at both ends
    let cases = [
        (200, 0, 1),
        (200, 200, 1),
        (200, 130, 3),
        (130, 63, 1),
        (130, 64, 1),
        (130, 65, 1),
        (128, 64, 2),
        (64, 32, 1),
        (10, 5, 1),
        (0, 0, 2),
    ];
    for (len, at, count) in cases {
        let (old, new) = inserted(len, at, count);

        // When
        let shared = Shared::of(&old, &new);

        // Then
        let expected = Shared {
            head: at,
            tail: len - at,
        };
        assert_eq!(shared, expected, "{count} put in at {at} of {len}");
    }
}

#[test]
fn the_shared_head_and_tail_are_found_around_a_removal() {
    for (len, at, count) in [(200, 70, 1), (200, 0, 64), (200, 136, 64), (130, 64, 2)] {
        // Given
        let old: Vec<usize> = (0..len).collect();
        let mut new = old.clone();
        new.drain(at..at + count);

        // When
        let shared = Shared::of(&old, &new);

        // Then
        let expected = Shared {
            head: at,
            tail: len - at - count,
        };
        assert_eq!(shared, expected, "{count} taken out at {at} of {len}");
    }
}

#[test]
fn the_shared_head_and_tail_do_not_overlap_in_the_shorter_list() {
    // Given: one slot appended to 100
    let (old, new) = inserted(100, 100, 1);

    // When
    let shared = Shared::of(&old, &new);

    // Then: the head takes all 100, the tail none of them again
    assert_eq!(shared, Shared { head: 100, tail: 0 });
}

#[test]
fn the_shared_head_and_tail_end_at_two_children_swapped() {
    // Given: in 0..200, swaps on chunk edges, across one, and at the end
    for (a, b) in [(64, 127), (63, 128), (0, 199), (198, 199), (127, 128)] {
        let old: Vec<usize> = (0..200).collect();
        let mut new = old.clone();
        new.swap(a, b);

        // When
        let shared = Shared::of(&old, &new);

        // Then: the head stops at the first, the tail after the second
        let expected = Shared {
            head: a,
            tail: 199 - b,
        };
        assert_eq!(shared, expected, "{a} and {b} swapped");
    }
}
