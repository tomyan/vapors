//! Solid scenery and car meshes. Positions use the simulation XY plane; Z is height.
#[derive(Default)]
pub(super) struct Mesh {
    pub vertices: Vec<f32>,
}
impl Mesh {
    fn triangle(&mut self, points: [[f32; 3]; 3], color: [f32; 3], car: bool) {
        for point in points {
            self.vertices.extend(point);
            self.vertices.extend(color);
            self.vertices.push(if car { 1.0 } else { 0.0 });
        }
    }
    fn cuboid(&mut self, origin: [f32; 3], size: [f32; 3], color: [f32; 3], car: bool) {
        let [x, y, z] = origin;
        let [width, depth, height] = size;
        let corners = [
            [x, y, z],
            [x + width, y, z],
            [x + width, y + depth, z],
            [x, y + depth, z],
            [x, y, z + height],
            [x + width, y, z + height],
            [x + width, y + depth, z + height],
            [x, y + depth, z + height],
        ];
        for (face, light) in [
            ([0, 1, 5, 4], 0.65),
            ([1, 2, 6, 5], 0.8),
            ([2, 3, 7, 6], 0.7),
            ([3, 0, 4, 7], 0.85),
            ([4, 5, 6, 7], 1.0),
        ] {
            let shade = color.map(|c| c * light);
            self.triangle(
                [corners[face[0]], corners[face[1]], corners[face[2]]],
                shade,
                car,
            );
            self.triangle(
                [corners[face[0]], corners[face[2]], corners[face[3]]],
                shade,
                car,
            );
        }
    }
    fn tree(&mut self, x: f32, y: f32) {
        self.cuboid(
            [x - 2.0, y - 2.0, 0.0],
            [4.0, 4.0, 16.0],
            [0.34, 0.24, 0.15],
            false,
        );
        for index in 0_i16..8 {
            let angle = f32::from(index) * std::f32::consts::TAU / 8.0;
            let next = angle + std::f32::consts::TAU / 8.0;
            let light = 0.8 + 0.2 * angle.cos();
            self.triangle(
                [
                    [x + angle.cos() * 16.0, y + angle.sin() * 16.0, 12.0],
                    [x + next.cos() * 16.0, y + next.sin() * 16.0, 12.0],
                    [x, y, 43.0],
                ],
                [0.29 * light, 0.48 * light, 0.33 * light],
                false,
            );
        }
    }
}
pub(super) fn scenery() -> Mesh {
    let mut mesh = Mesh::default();
    mesh.cuboid(
        [-136.0, -6.0, 0.0],
        [188.0, 60.0, 24.0],
        [0.76, 0.73, 0.59],
        false,
    );
    mesh.cuboid(
        [-140.0, -10.0, 24.0],
        [196.0, 68.0, 4.0],
        [0.29, 0.40, 0.40],
        false,
    );
    for x in -12_i16..=12 {
        for y in -12_i16..=12 {
            let centre = [(f32::from(x) + 0.5) * 62.0, (f32::from(y) + 0.5) * 62.0];
            if crate::model::road_distance(centre) > crate::model::ROAD + 37.0
                && centre[0].hypot(centre[1]) > 250.0
            {
                mesh.tree(centre[0], centre[1]);
            }
        }
    }
    mesh
}
pub(super) fn car() -> Mesh {
    let mut mesh = Mesh::default();
    for x in [-11.0, 8.0] {
        for y in [-10.0, 7.0] {
            mesh.cuboid([x, y, 1.0], [6.0, 3.0, 6.0], [0.07, 0.09, 0.10], true);
        }
    }
    mesh.cuboid(
        [-16.0, -7.0, 3.0],
        [32.0, 14.0, 6.0],
        [0.96, 0.58, 0.29],
        true,
    );
    mesh.cuboid(
        [-5.0, -5.5, 9.0],
        [12.0, 11.0, 5.0],
        [0.19, 0.33, 0.38],
        true,
    );
    mesh.cuboid(
        [-4.0, -5.0, 14.0],
        [7.0, 10.0, 1.0],
        [0.96, 0.62, 0.32],
        true,
    );
    mesh.cuboid(
        [-16.0, -1.0, 9.1],
        [10.0, 2.0, 0.2],
        [0.95, 0.88, 0.72],
        true,
    );
    mesh.cuboid([8.0, -1.0, 9.1], [8.0, 2.0, 0.2], [0.95, 0.88, 0.72], true);
    mesh
}
#[cfg(test)]
#[path = "mesh_tests.rs"]
mod tests;
