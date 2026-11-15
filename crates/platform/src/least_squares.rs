//! Chromium's `SolveLeastSquares` (`ui/events/velocity_tracker/
//! velocity_tracker.cc`), unweighted: the polynomial with `count`
//! coefficients nearest the points in least squares, by a Gram-Schmidt
//! QR decomposition, in `f32` as Chromium computes it.

/// Most coefficients solved for (`Estimator::kMaxDegree`).
const MAX_COEFFICIENTS: usize = 4;

/// Columns of a matrix, each a vector over the points.
type Columns = Vec<Vec<f32>>;

/// The fitted polynomial's slope at 0 (its coefficient of the first
/// power); `None` when the columns are linearly dependent (a norm under
/// 1e-6, Chromium's test) or `count` is outside 2..=4.
pub(crate) fn slope_at_zero(times: &[f32], values: &[f32], count: usize) -> Option<f32> {
    solve(times, values, count).map(|coefficients| coefficients[1])
}

fn solve(times: &[f32], values: &[f32], count: usize) -> Option<[f32; MAX_COEFFICIENTS]> {
    if !(2..=MAX_COEFFICIENTS).contains(&count) || times.len() != values.len() {
        return None;
    }
    // column i holds each time to the power i
    let powers: Columns = (0..count)
        .map(|i| times.iter().map(|&t| power(t, i)).collect())
        .collect();
    let (basis, upper) = decompose(&powers)?;
    Some(back_substitute(&basis, &upper, values))
}

/// Solves upper × coefficients = basisᵀ × values, bottom row first.
fn back_substitute(
    basis: &[Vec<f32>],
    upper: &[Vec<f32>],
    values: &[f32],
) -> [f32; MAX_COEFFICIENTS] {
    let mut coefficients = [0.0; MAX_COEFFICIENTS];
    for i in (0..basis.len()).rev() {
        let mut value = dot(&basis[i], values);
        for j in (i + 1..basis.len()).rev() {
            value -= upper[i][j] * coefficients[j];
        }
        coefficients[i] = value / upper[i][i];
    }
    coefficients
}

/// The QR decomposition of `columns`: orthonormal columns and the upper
/// triangular matrix, by rows.
fn decompose(columns: &[Vec<f32>]) -> Option<(Columns, Columns)> {
    let count = columns.len();
    let mut basis: Columns = Vec::with_capacity(count);
    let mut upper = vec![vec![0.0; count]; count];
    for j in 0..count {
        let column = orthonormal(&columns[j], &basis)?;
        for (i, cell) in upper[j].iter_mut().enumerate().skip(j) {
            *cell = dot(&column, &columns[i]);
        }
        basis.push(column);
    }
    Some((basis, upper))
}

/// `column` less its projections on `basis`, at unit length (modified
/// Gram-Schmidt); `None` when what is left is under 1e-6 long.
fn orthonormal(column: &[f32], basis: &[Vec<f32>]) -> Option<Vec<f32>> {
    let mut column = column.to_vec();
    for earlier in basis {
        let projection = dot(earlier, &column);
        for (cell, along) in column.iter_mut().zip(earlier) {
            *cell -= projection * along;
        }
    }
    let norm = dot(&column, &column).sqrt();
    if norm < 0.000_001 {
        return None;
    }
    let inverse = 1.0 / norm;
    Some(column.into_iter().map(|cell| cell * inverse).collect())
}

fn power(value: f32, exponent: usize) -> f32 {
    (0..exponent).fold(1.0, |product, _| product * value)
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(l, r)| l * r).sum()
}
