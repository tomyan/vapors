use super::*;

fn car(x: f32, row: i32) -> Piece {
    Piece {
        x,
        row,
        kind: Kind::Car,
        speed: 1.0,
    }
}

fn log(x: f32, row: i32, speed: f32) -> Piece {
    Piece {
        x,
        row,
        kind: Kind::Log,
        speed,
    }
}

#[test]
fn an_arrow_moves_the_frog_one_square() {
    // Given
    let mut world = World::new();

    // When
    world.key("ArrowLeft");

    // Then
    assert_eq!(world.frog, ((WIDTH / 2 - 1) as f32, HEIGHT - 1));
}

#[test]
fn reaching_the_far_bank_scores_and_starts_again() {
    // Given
    let mut world = World {
        frog: (3.0, 1),
        ..World::new()
    };

    // When
    world.key("ArrowUp");

    // Then
    assert_eq!((world.frog, world.score), (start(), 1));
}

#[test]
fn a_car_over_the_frog_costs_a_life() {
    // Given
    let mut world = World {
        frog: (3.0, 5),
        ..World::new()
    };

    // When
    world.tick(&[car(3.5, 5)], 0.016);

    // Then
    assert_eq!((world.frog, world.lives), (start(), LIVES - 1));
}

#[test]
fn a_car_in_another_row_does_nothing() {
    // Given
    let mut world = World {
        frog: (3.0, 5),
        ..World::new()
    };

    // When
    world.tick(&[car(3.0, 6)], 0.016);

    // Then
    assert_eq!(world.lives, LIVES);
}

#[test]
fn a_log_carries_the_frog_at_its_speed() {
    // Given
    let mut world = World {
        frog: (3.0, 1),
        ..World::new()
    };

    // When
    world.tick(&[log(3.5, 1, 2.0)], 0.5);

    // Then
    assert_eq!((world.frog, world.lives), ((4.0, 1), LIVES));
}

#[test]
fn open_water_costs_a_life() {
    // Given
    let mut world = World {
        frog: (3.0, 2),
        ..World::new()
    };

    // When
    world.tick(&[log(8.0, 2, 1.0)], 0.016);

    // Then
    assert_eq!(world.lives, LIVES - 1);
}

#[test]
fn any_key_after_the_last_life_starts_a_new_game() {
    // Given
    let mut world = World {
        lives: 0,
        score: 4,
        ..World::new()
    };

    // When
    world.key("x");

    // Then
    assert_eq!(world, World::new());
}

#[test]
fn pieces_wrap_around_beyond_the_edges() {
    // When: the first log, from column 0 at 1.2 a second, after 10 s
    let first = pieces_at(10.0)[0];

    // Then: 12 squares on, around a 13-square loop
    assert!((first.x - -1.0).abs() < 1e-4, "{first:?}");
}
