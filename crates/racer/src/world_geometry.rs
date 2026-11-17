//! The racer's solid scenery and car, as triangles tagged by the part
//! they belong to, so the Track renderer (`gpu/mesh.rs`, colours baked)
//! and the scene (`world.rs`, a mesh per part, its colour from CSS) draw
//! from one source. Positions are in the simulation's X, Y plane; Z is
//! height. Each triangle carries its face's light, which the Track bakes
//! into the vertex colour and the scene gives as the vertex shade.

/// A part of the racer's world drawn in one colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// The building's walls.
    Building,
    /// The building's roof.
    Roof,
    /// Every tree's trunk.
    Trunk,
    /// Every tree's foliage.
    Foliage,
    /// The car's four wheels.
    Wheels,
    /// The car's body.
    Body,
    /// The car's cabin.
    Cabin,
    /// The car's roof.
    CarRoof,
    /// The car's two stripes.
    Stripes,
}

impl Part {
    /// Every part, in the order the scene draws them.
    pub const ALL: [Part; 9] = [
        Part::Building,
        Part::Roof,
        Part::Trunk,
        Part::Foliage,
        Part::Wheels,
        Part::Body,
        Part::Cabin,
        Part::CarRoof,
        Part::Stripes,
    ];

    /// Its colour, sRGB-encoded, before a face's light.
    #[must_use]
    pub const fn colour(self) -> [f32; 3] {
        match self {
            Part::Building => [0.76, 0.73, 0.59],
            Part::Roof => [0.29, 0.40, 0.40],
            Part::Trunk => [0.34, 0.24, 0.15],
            Part::Foliage => [0.29, 0.48, 0.33],
            Part::Wheels => [0.07, 0.09, 0.10],
            Part::Body => [0.96, 0.58, 0.29],
            Part::Cabin => [0.19, 0.33, 0.38],
            Part::CarRoof => [0.96, 0.62, 0.32],
            Part::Stripes => [0.95, 0.88, 0.72],
        }
    }

    /// Whether it moves with the car.
    #[must_use]
    pub const fn on_car(self) -> bool {
        matches!(
            self,
            Part::Wheels | Part::Body | Part::Cabin | Part::CarRoof | Part::Stripes
        )
    }
}

/// Where triangles go.
pub trait Sink {
    /// A triangle of `part`, its face lit by `light`.
    fn triangle(&mut self, part: Part, points: [[f32; 3]; 3], light: f32);
}

/// A box from `origin`, `size` wide, deep and high: four sides and a top,
/// each face its own light.
fn cuboid(sink: &mut dyn Sink, part: Part, origin: [f32; 3], size: [f32; 3]) {
    let [left, near, low] = origin;
    let [right, far, high] = [0, 1, 2].map(|axis| origin[axis] + size[axis]);
    let corners = [
        [left, near, low],
        [right, near, low],
        [right, far, low],
        [left, far, low],
        [left, near, high],
        [right, near, high],
        [right, far, high],
        [left, far, high],
    ];
    for (face, light) in [
        ([0, 1, 5, 4], 0.65),
        ([1, 2, 6, 5], 0.8),
        ([2, 3, 7, 6], 0.7),
        ([3, 0, 4, 7], 0.85),
        ([4, 5, 6, 7], 1.0),
    ] {
        let [first, second, third, fourth] = face.map(|at| corners[at]);
        sink.triangle(part, [first, second, third], light);
        sink.triangle(part, [first, third, fourth], light);
    }
}

/// A tree at `(x, y)`: a trunk and eight foliage faces.
fn tree(sink: &mut dyn Sink, x: f32, y: f32) {
    cuboid(sink, Part::Trunk, [x - 2.0, y - 2.0, 0.0], [4.0, 4.0, 16.0]);
    for index in 0_i16..8 {
        let angle = f32::from(index) * std::f32::consts::TAU / 8.0;
        let next = angle + std::f32::consts::TAU / 8.0;
        let light = 0.8 + 0.2 * angle.cos();
        let rim = |at: f32| [x + at.cos() * 16.0, y + at.sin() * 16.0, 12.0];
        sink.triangle(Part::Foliage, [rim(angle), rim(next), [x, y, 43.0]], light);
    }
}

/// The building and every tree: the grid's cells clear of the road and
/// of the centre.
pub fn scenery(sink: &mut dyn Sink) {
    cuboid(
        sink,
        Part::Building,
        [-136.0, -6.0, 0.0],
        [188.0, 60.0, 24.0],
    );
    cuboid(sink, Part::Roof, [-140.0, -10.0, 24.0], [196.0, 68.0, 4.0]);
    for x in -12_i16..=12 {
        for y in -12_i16..=12 {
            let centre = [(f32::from(x) + 0.5) * 62.0, (f32::from(y) + 0.5) * 62.0];
            if crate::model::road_distance(centre) > crate::model::ROAD + 37.0
                && centre[0].hypot(centre[1]) > 250.0
            {
                tree(sink, centre[0], centre[1]);
            }
        }
    }
}

/// The car in its own frame: +X forward, centred on the origin.
pub fn car(sink: &mut dyn Sink) {
    for x in [-11.0, 8.0] {
        for y in [-10.0, 7.0] {
            cuboid(sink, Part::Wheels, [x, y, 1.0], [6.0, 3.0, 6.0]);
        }
    }
    cuboid(sink, Part::Body, [-16.0, -7.0, 3.0], [32.0, 14.0, 6.0]);
    cuboid(sink, Part::Cabin, [-5.0, -5.5, 9.0], [12.0, 11.0, 5.0]);
    cuboid(sink, Part::CarRoof, [-4.0, -5.0, 14.0], [7.0, 10.0, 1.0]);
    cuboid(sink, Part::Stripes, [-16.0, -1.0, 9.1], [10.0, 2.0, 0.2]);
    cuboid(sink, Part::Stripes, [8.0, -1.0, 9.1], [8.0, 2.0, 0.2]);
}
