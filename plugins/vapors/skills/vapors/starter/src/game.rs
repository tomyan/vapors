//! The game's rules, in plain Rust with no vapors types, so `cargo
//! test` checks them natively.

/// Columns on the board.
pub const WIDTH: i32 = 9;
/// Rows: row 0 is the far bank, the last row the start.
pub const HEIGHT: i32 = 9;
/// Lives at the start of a game.
pub const LIVES: u32 = 3;

/// What a row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ground {
    Grass,
    Road,
    Water,
}

/// What moves along a lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Car,
    Truck,
    Log,
}

/// One lane: its row, what moves on it, its speed in squares a second
/// (negative moves left), and where each piece starts.
struct Lane {
    row: i32,
    kind: Kind,
    speed: f32,
    starts: &'static [f32],
}

const LANES: [Lane; 6] = [
    Lane {
        row: 1,
        kind: Kind::Log,
        speed: 1.2,
        starts: &[0.0, 5.0],
    },
    Lane {
        row: 2,
        kind: Kind::Log,
        speed: -1.6,
        starts: &[1.0, 6.5],
    },
    Lane {
        row: 4,
        kind: Kind::Car,
        speed: -2.0,
        starts: &[0.0, 4.5],
    },
    Lane {
        row: 5,
        kind: Kind::Truck,
        speed: 1.0,
        starts: &[1.0, 7.0],
    },
    Lane {
        row: 6,
        kind: Kind::Car,
        speed: -1.4,
        starts: &[2.0, 6.0],
    },
    Lane {
        row: 7,
        kind: Kind::Car,
        speed: 2.4,
        starts: &[0.0, 4.0],
    },
];

/// Pieces leave one side and come back on the other, off the board.
const MARGIN: f32 = 2.0;
/// Half the frog's width, for what it touches.
const FROG: f32 = 0.3;

impl Kind {
    /// Its length along the lane, in squares.
    #[must_use]
    pub fn length(self) -> f32 {
        match self {
            Kind::Car => 1.3,
            Kind::Truck => 2.2,
            Kind::Log => 2.8,
        }
    }
}

/// What each row is.
#[must_use]
pub fn ground(row: i32) -> Ground {
    match row {
        1 | 2 => Ground::Water,
        4..=7 => Ground::Road,
        _ => Ground::Grass,
    }
}

/// One piece at one moment: its centre's column, its row, its kind and
/// its lane's speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub x: f32,
    pub row: i32,
    pub kind: Kind,
    pub speed: f32,
}

impl Piece {
    fn covers(&self, x: f32, reach: f32) -> bool {
        (self.x - x).abs() < self.kind.length() / 2.0 + reach
    }
}

/// Every piece after `seconds`.
#[must_use]
pub fn pieces_at(seconds: f32) -> Vec<Piece> {
    let span = WIDTH as f32 + 2.0 * MARGIN;
    LANES
        .iter()
        .flat_map(|lane| {
            lane.starts.iter().map(move |start| Piece {
                x: (start + MARGIN + lane.speed * seconds).rem_euclid(span) - MARGIN,
                row: lane.row,
                kind: lane.kind,
                speed: lane.speed,
            })
        })
        .collect()
}

/// The frog, the score and the lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct World {
    /// The frog's column (fractional while a log carries it) and row.
    pub frog: (f32, i32),
    pub score: u32,
    pub lives: u32,
}

impl World {
    #[must_use]
    pub fn new() -> Self {
        Self {
            frog: start(),
            score: 0,
            lives: LIVES,
        }
    }

    /// A key went down: the arrows move the frog one square; any key
    /// starts a new game once the lives are gone. Whether it used the key.
    pub fn key(&mut self, key: &str) -> bool {
        if self.lives == 0 {
            *self = Self::new();
            return true;
        }
        let (dx, dy) = match key {
            "ArrowLeft" => (-1.0, 0),
            "ArrowRight" => (1.0, 0),
            "ArrowUp" => (0.0, -1),
            "ArrowDown" => (0.0, 1),
            _ => return false,
        };
        let (x, row) = self.frog;
        let x = (x.round() + dx).clamp(0.0, (WIDTH - 1) as f32);
        self.frog = (x, (row + dy).clamp(0, HEIGHT - 1));
        if self.frog.1 == 0 {
            self.score += 1;
            self.frog = start();
        }
        true
    }

    /// `dt` seconds passed and the pieces are where `pieces` says: a log
    /// carries the frog, a car or open water costs a life.
    pub fn tick(&mut self, pieces: &[Piece], dt: f32) {
        if self.lives == 0 {
            return;
        }
        let (x, row) = self.frog;
        let mut under = pieces.iter().filter(|piece| piece.row == row);
        let safe = match ground(row) {
            Ground::Grass => true,
            Ground::Road => !under.any(|car| car.covers(x, FROG)),
            Ground::Water => match under.find(|log| log.covers(x, 0.0)) {
                Some(log) => {
                    self.frog.0 += log.speed * dt;
                    (0.0..=(WIDTH - 1) as f32).contains(&self.frog.0)
                }
                None => false,
            },
        };
        if !safe {
            self.lives -= 1;
            self.frog = start();
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

fn start() -> (f32, i32) {
    ((WIDTH / 2) as f32, HEIGHT - 1)
}

#[cfg(test)]
#[path = "game_tests.rs"]
mod tests;
