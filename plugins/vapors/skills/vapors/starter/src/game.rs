//! The game's rules, in plain Rust with no vapors types, so `cargo
//! test` checks them natively.

/// Columns on the board.
pub const WIDTH: i32 = 9;
/// Rows on the board: row 0 is the far side, the last row the start.
pub const HEIGHT: i32 = 7;

/// Each road row: its row, its speed in squares a second (negative
/// drives left), and its cars' columns at time zero.
pub const LANES: [(i32, f32, &[f32]); 5] = [
    (1, 1.5, &[0.0, 4.5]),
    (2, -2.0, &[2.0, 6.0]),
    (3, 1.0, &[1.0, 4.0, 7.0]),
    (4, -1.2, &[3.0, 7.5]),
    (5, 2.5, &[0.0, 5.0]),
];

/// How close a car's centre may come to the frog's before it hits:
/// half a car (1.4 wide) plus half the frog (0.7), rounded down.
const REACH: f32 = 1.0;

/// Where every car is after `seconds`: its column (fractional, so it
/// moves smoothly) and its row.
#[must_use]
pub fn cars_at(seconds: f32) -> Vec<(f32, i32)> {
    let width = WIDTH as f32;
    LANES
        .iter()
        .flat_map(|&(row, speed, starts)| {
            starts
                .iter()
                .map(move |start| ((start + speed * seconds).rem_euclid(width), row))
        })
        .collect()
}

/// The frog and the score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct World {
    /// The frog's column and row.
    pub frog: (i32, i32),
    pub crossings: u32,
    pub hits: u32,
}

impl World {
    #[must_use]
    pub fn new() -> Self {
        Self {
            frog: start(),
            crossings: 0,
            hits: 0,
        }
    }

    /// A key went down: the arrows move the frog one square; reaching
    /// row 0 scores a crossing and starts again.
    pub fn key(&mut self, key: &str) {
        let (dx, dy) = match key {
            "ArrowLeft" => (-1, 0),
            "ArrowRight" => (1, 0),
            "ArrowUp" => (0, -1),
            "ArrowDown" => (0, 1),
            _ => return,
        };
        let (x, y) = self.frog;
        self.frog = ((x + dx).clamp(0, WIDTH - 1), (y + dy).clamp(0, HEIGHT - 1));
        if self.frog.1 == 0 {
            self.crossings += 1;
            self.frog = start();
        }
    }

    /// A car within reach of the frog sends it back and counts a hit.
    pub fn run_over(&mut self, cars: &[(f32, i32)]) {
        let (x, y) = self.frog;
        if cars
            .iter()
            .any(|&(cx, row)| row == y && (cx - x as f32).abs() < REACH)
        {
            self.hits += 1;
            self.frog = start();
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

fn start() -> (i32, i32) {
    (WIDTH / 2, HEIGHT - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arrow_moves_the_frog_one_square() {
        // Given
        let mut world = World::new();

        // When
        world.key("ArrowLeft");

        // Then
        assert_eq!(world.frog, (WIDTH / 2 - 1, HEIGHT - 1));
    }

    #[test]
    fn reaching_the_far_side_counts_a_crossing() {
        // Given
        let mut world = World {
            frog: (0, 1),
            ..World::new()
        };

        // When
        world.key("ArrowUp");

        // Then
        assert_eq!((world.frog, world.crossings), (start(), 1));
    }

    #[test]
    fn a_car_within_reach_sends_the_frog_back_and_counts_a_hit() {
        // Given
        let mut world = World {
            frog: (3, 2),
            ..World::new()
        };

        // When
        world.run_over(&[(3.5, 2)]);

        // Then
        assert_eq!((world.frog, world.hits), (start(), 1));
    }

    #[test]
    fn cars_move_by_their_lane_speed_and_wrap_around() {
        // When
        let cars = cars_at(2.0);

        // Then: lane 2 drives left at 2 a second from column 2
        assert!(cars.contains(&(7.0, 2)));
    }
}
