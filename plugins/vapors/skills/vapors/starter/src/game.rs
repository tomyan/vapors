//! The game's rules, in plain Rust: no vapors types, so `cargo test`
//! checks them natively.

/// Columns on the board.
pub const WIDTH: i32 = 9;
/// Rows on the board; row 0 is the far side, the last row the start.
pub const HEIGHT: i32 = 7;

/// What one square shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Grass,
    Road,
    Car,
    Frog,
}

/// The whole state of a game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    /// The frog's column and row.
    pub frog: (i32, i32),
    /// Each car's column and row.
    pub cars: Vec<(i32, i32)>,
    pub crossings: u32,
    pub hits: u32,
}

impl World {
    #[must_use]
    pub fn new() -> Self {
        let cars = vec![
            (0, 1),
            (4, 1),
            (2, 2),
            (7, 3),
            (3, 3),
            (5, 4),
            (1, 5),
            (6, 5),
        ];
        Self {
            frog: start(),
            cars,
            crossings: 0,
            hits: 0,
        }
    }

    /// A key went down: the arrows move the frog one square.
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
        self.settle();
    }

    /// One step of time: odd rows drive right, even rows left.
    pub fn step(&mut self) {
        for (x, y) in &mut self.cars {
            let dx = if *y % 2 == 1 { 1 } else { -1 };
            *x = (*x + dx).rem_euclid(WIDTH);
        }
        self.settle();
    }

    /// A frog under a car goes back to the start; one on row 0 scores.
    fn settle(&mut self) {
        if self.cars.contains(&self.frog) {
            self.hits += 1;
            self.frog = start();
        } else if self.frog.1 == 0 {
            self.crossings += 1;
            self.frog = start();
        }
    }

    /// The board, row by row, for the template.
    #[must_use]
    pub fn tiles(&self) -> Vec<Vec<Tile>> {
        (0..HEIGHT)
            .map(|y| (0..WIDTH).map(|x| self.tile(x, y)).collect())
            .collect()
    }

    fn tile(&self, x: i32, y: i32) -> Tile {
        if self.frog == (x, y) {
            Tile::Frog
        } else if self.cars.contains(&(x, y)) {
            Tile::Car
        } else if y == 0 || y == HEIGHT - 1 {
            Tile::Grass
        } else {
            Tile::Road
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
        let mut world = World {
            cars: vec![],
            ..World::new()
        };

        // When
        world.key("ArrowLeft");

        // Then
        assert_eq!(world.frog, (WIDTH / 2 - 1, HEIGHT - 1));
    }

    #[test]
    fn a_car_driving_into_the_frog_sends_it_back_and_counts_a_hit() {
        // Given
        let mut world = World {
            cars: vec![(0, 1)],
            frog: (1, 1),
            ..World::new()
        };

        // When
        world.step();

        // Then
        assert_eq!((world.frog, world.hits), (start(), 1));
    }

    #[test]
    fn reaching_the_far_side_counts_a_crossing() {
        // Given
        let mut world = World {
            cars: vec![],
            frog: (0, 1),
            ..World::new()
        };

        // When
        world.key("ArrowUp");

        // Then
        assert_eq!((world.frog, world.crossings), (start(), 1));
    }
}
