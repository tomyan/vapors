//! Ordered clean-lap checks, including reverse travel and infield shortcuts.
use vapors_racer::model::{Progress, RADIUS, STRAIGHT};
fn circuit() -> Vec<[f32; 2]> {
    (0_u16..=720)
        .map(|i| {
            let angle = f32::from(i) * std::f32::consts::TAU / 720.0 - std::f32::consts::FRAC_PI_2;
            let direction = [angle.cos(), angle.sin()];
            let mut low = 0.0;
            let mut high = 400.0;
            for _ in 0..20 {
                let mid = f32::midpoint(low, high);
                let x = direction[0] * mid;
                let y = direction[1] * mid;
                if (x - x.clamp(-STRAIGHT, STRAIGHT)).hypot(y) > RADIUS {
                    high = mid;
                } else {
                    low = mid;
                }
            }
            [direction[0] * low, direction[1] * low]
        })
        .collect()
}
#[test]
fn three_forward_laps_finish_with_a_best_time() {
    // Given
    let mut progress = Progress::default();
    let mut elapsed = 0.0;
    // When
    for _ in 0..3 {
        for point in circuit() {
            elapsed += 0.02;
            progress.observe(point, elapsed);
        }
    }
    // Then
    assert_eq!(progress.laps, 3);
    assert!(progress.best.unwrap() > 14.0);
}
#[test]
fn reverse_lap_does_not_count() {
    // Given
    let mut progress = Progress::default();
    // When
    for point in circuit().into_iter().rev() {
        progress.observe(point, 20.0);
    }
    // Then
    assert_eq!(progress.laps, 0);
}
#[test]
fn leaving_the_track_cannot_complete_a_shortcut_lap() {
    // Given
    let mut progress = Progress::default();
    let points = circuit();
    // When
    for point in &points[..400] {
        progress.observe(*point, 5.0);
    }
    progress.observe([0.0, 0.0], 6.0);
    for point in &points[600..] {
        progress.observe(*point, 10.0);
    }
    // Then
    assert_eq!(progress.laps, 0);
}
