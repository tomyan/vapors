use super::*;

#[test]
fn halving_averages_each_two_by_two_block() {
    // Given: 4 x 2, black and white columns
    let black = [0, 0, 0, 255];
    let white = [255, 255, 255, 255];
    let rgba = [black, white, black, white, black, white, black, white].concat();

    // When
    let (size, half) = halved([4, 2], &rgba);

    // Then: 2 x 1 of the average, rounded
    assert_eq!(size, [2, 1]);
    assert_eq!(half, [128, 128, 128, 255, 128, 128, 128, 255]);
}

#[test]
fn an_odd_side_keeps_its_last_row_or_column_and_never_reaches_zero() {
    // Given: 3 x 1
    let rgba = [[10, 0, 0, 255], [20, 0, 0, 255], [90, 0, 0, 255]].concat();

    // When
    let (size, half) = halved([3, 1], &rgba);

    // Then: 1 x 1, the first two averaged (the last column is dropped)
    assert_eq!(size, [1, 1]);
    assert_eq!(half, [15, 0, 0, 255]);
}

#[test]
fn the_chain_runs_down_to_one_pixel() {
    // When
    let levels = chain([8, 2], &[255; 8 * 2 * 4]);

    // Then
    let sizes: Vec<[u32; 2]> = levels.iter().map(|(size, _)| *size).collect();
    assert_eq!(sizes, [[8, 2], [4, 1], [2, 1], [1, 1]]);
}
