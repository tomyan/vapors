//! The longest increasing subsequence: which rows of a reordered list
//! can stay where they are so the others move around them.

/// Indices into `positions` of one longest strictly increasing
/// subsequence. Patience sorting, O(n log n).
pub(crate) fn longest_increasing(positions: &[usize]) -> Vec<usize> {
    let mut tails: Vec<usize> = Vec::new();
    let mut predecessor = vec![usize::MAX; positions.len()];
    for (i, &value) in positions.iter().enumerate() {
        let at = tails.partition_point(|&tail| positions[tail] < value);
        if at > 0 {
            predecessor[i] = tails[at - 1];
        }
        if at == tails.len() {
            tails.push(i);
        } else {
            tails[at] = i;
        }
    }
    let mut result = Vec::with_capacity(tails.len());
    let mut current = tails.last().copied();
    while let Some(i) = current {
        result.push(i);
        current = (predecessor[i] != usize::MAX).then(|| predecessor[i]);
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::longest_increasing;

    #[test]
    fn keeps_the_longest_run_in_order() {
        // Given / When / Then
        assert_eq!(longest_increasing(&[2, 0, 1]), [1, 2]);
        assert_eq!(longest_increasing(&[1, 0]).len(), 1);
        assert_eq!(longest_increasing(&[0, 1, 2]), [0, 1, 2]);
        assert_eq!(longest_increasing(&[]), Vec::<usize>::new());
        assert_eq!(longest_increasing(&[3, 1, 2, 0]), [1, 2]);
    }
}
