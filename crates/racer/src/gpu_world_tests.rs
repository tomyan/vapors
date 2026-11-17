//! GPU tier: the racer's world in the scene (one-scene R3a1) draws what
//! the Track drew, recorded as golden pictures before R3a2 deleted it:
//! in the fixed and the following camera, on a landscape and a portrait
//! target, and close on the car. Ground and objects go through the
//! scene's one view-projection, so this is the ground and object check
//! (`docs/plans/3d-2.md`) on the racer's geometry: wherever the golden
//! picture is flat, the scene's pixel matches it (`gpu_world_compare.rs`).
//! The car's roof lands where `Projection::project` puts it; the scene's
//! draws are pinned.

use vapors_app::SurfaceSize;

use crate::gpu_world_compare::{agreement, roof_pixel};
use crate::gpu_world_golden::golden;
use crate::gpu_world_rig::{LANDSCAPE, PORTRAIT, Rig};
use crate::model::{Camera, RADIUS, Race};

/// The car on the bottom straight, heading along it.
fn race() -> Race {
    let mut race = Race::default();
    race.position = [60.0, -RADIUS];
    race.heading = 0.4;
    race
}

/// The fixed camera at rest (`Camera::blend`'s targets at speed 0).
fn fixed(_: &Race) -> Camera {
    Camera::default()
}

/// The following camera at rest behind the car: yawed to its heading,
/// tilted 0.62, looking 45 units ahead.
fn following(race: &Race) -> Camera {
    let ahead = [race.heading.cos() * 45.0, race.heading.sin() * 45.0];
    Camera {
        zoom: 1.25,
        follow: true,
        rotation: race.heading + std::f32::consts::FRAC_PI_2,
        tilt: 0.62,
        offset: ahead,
        moving: false,
    }
}

/// The fixed camera's tilt, centred on the car at 6 pixels a unit on
/// `LANDSCAPE` (zoom 10: 240 / 400 * 10), so the car covers some
/// hundreds of flat pixels and a placement off by a pixel shows.
fn close(_: &Race) -> Camera {
    Camera {
        zoom: 10.0,
        ..Camera::default()
    }
}

/// The car roof's colour: its top face, lit 1.
const ROOF: [u8; 4] = [245, 158, 82, 255];

/// The scene's draws: the composition, the ground and the nine parts.
const DRAWS: u64 = 11;

/// The fewest flat pixels each check compares, under what ANV (UHD 630)
/// and llvmpipe gave on 2026-11-17: fixed 19,012 and 18,774; following
/// 17,271 and 17,128; portrait 14,982 and 14,665; close 59,484 and
/// 59,705 (of 76,800; the ground's noise breaks up the rest).
const FEWEST_LANDSCAPE: usize = 15_000;
const FEWEST_PORTRAIT: usize = 12_000;
const FEWEST_CLOSE: usize = 50_000;

/// The most flat pixels each check lets differ by 3 to 6 (the ground's
/// noise, `gpu_world_compare.rs`), over what was measured against the
/// goldens (recorded on ANV) on 2026-11-17: on ANV 0 to 1 a check; on
/// llvmpipe, whose `sin` hash rounds otherwise, fixed 863, following 953,
/// portrait 1,036 and close 7,991. A pixel further apart than 6 fails
/// every check.
const NOISY_FIXED: usize = 1_100;
const NOISY_FOLLOWING: usize = 1_200;
const NOISY_PORTRAIT: usize = 1_300;
const NOISY_CLOSE: usize = 10_000;

/// The racer's world through `camera` at `size` against the golden
/// picture `name` (the Track's, `gpu_world_golden.rs`): at least
/// `fewest` flat pixels compared (fewer means the ground's noise or the
/// target changed, and the check would compare too little), at most
/// `noisy` of them 3 to 6 apart and none further; the roof where
/// `project` puts it; the draws pinned.
fn check(
    name: &str,
    camera: fn(&Race) -> Camera,
    size: SurfaceSize,
    (fewest, noisy): (usize, usize),
) {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let race = race();
    let camera = camera(&race);
    let reference = golden(name, size);
    let mut rig = Rig::new(&race, &camera, size);

    // When
    let (work, scene) = rig.frame(&gpu);

    // Then
    let found = agreement(size, &reference, &scene);
    assert!(
        found.flat >= fewest,
        "{name}: {} flat pixels, under the {fewest} this check compares: the ground's noise or the target changed, so it would compare too little; reproduce with `cargo nextest run -p vapors-racer gpu_the_racers_world`",
        found.flat
    );
    assert!(
        found.apart == 0 && found.noisy <= noisy,
        "{name}: {found:?}, at most {noisy} noisy"
    );
    let (x, y) = roof_pixel(&rig, &race);
    let at = ((y * size.width + x) * 4) as usize;
    let pixel = |pixels: &[u8]| -> [u8; 4] { pixels[at..at + 4].try_into().expect("four bytes") };
    assert_eq!(
        (pixel(&scene), pixel(&reference)),
        (ROOF, ROOF),
        "{name} at ({x}, {y})"
    );
    assert_eq!(work.draw_calls, DRAWS);
}

#[test]
fn gpu_the_racers_world_draws_the_golden_picture_in_the_fixed_camera() {
    check("fixed", fixed, LANDSCAPE, (FEWEST_LANDSCAPE, NOISY_FIXED));
}

#[test]
fn gpu_the_racers_world_draws_the_golden_picture_in_the_following_camera() {
    check(
        "following",
        following,
        LANDSCAPE,
        (FEWEST_LANDSCAPE, NOISY_FOLLOWING),
    );
}

#[test]
fn gpu_the_racers_world_draws_the_golden_picture_on_a_portrait_target() {
    check(
        "following-portrait",
        following,
        PORTRAIT,
        (FEWEST_PORTRAIT, NOISY_PORTRAIT),
    );
}

#[test]
fn gpu_the_racers_world_draws_the_golden_picture_close_on_the_car() {
    check("close", close, LANDSCAPE, (FEWEST_CLOSE, NOISY_CLOSE));
}
