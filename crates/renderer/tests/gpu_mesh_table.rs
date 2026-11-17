//! GPU tier: kept mesh items placed through the transform table
//! (`docs/plans/flat-scene-f2.md`, F2.3): an item in a frame's entry
//! lands moved by the frame's shift, and an item in a lane's row lands
//! moved by its row's offset, as the same item drawn at the moved place
//! does; a pipeline draws from one table.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/meshes.rs"]
mod meshes;
#[path = "support/table.rs"]
mod table;

use frame::{square, target};
use meshes::mesh;
use table::{Table, table};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, MeshEntries, MeshEntry, MeshPipeline, MeshShape, Target};
use vapors_scene::Color;
use vapors_shapes::MeshTriangle;

const SIDE: u32 = 32;

/// A filled 6 x 4 triangle with a curved side, at the origin.
fn shape() -> Vec<MeshTriangle> {
    mesh("M0 0 L6 0 Q6 4 0 4 Z", None)
}

/// A red item translated to `at`.
fn item(at: [f32; 2]) -> MeshShape {
    let transform = [1.0, 0.0, 0.0, 1.0, at[0], at[1]];
    let red = Color::from_rgba8(255, 0, 0, 255);
    MeshShape::new(transform, red)
        .expect("drawable")
        .expect("has area")
}

/// The pixels of what `draw` draws with a mesh pipeline and an entry
/// of `shape`, on a square `SIDE` across.
fn on_square(draw: impl FnOnce(&Target<'_>, &mut MeshPipeline, &mut Drawing)) -> Vec<u8> {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let mut pipeline = MeshPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let at = target(&gpu, &encoder, &view, SIDE);
    let mut entries = MeshEntries::new(&gpu.device);
    let entry = entries.write(&at, &shape()).expect("room");
    draw(&at, &mut pipeline, &mut Drawing { entries, entry });
    encoder.submit(&gpu.queue).expect("no refusal");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

/// The mesh entries and the entry of `shape` in them.
struct Drawing {
    entries: MeshEntries,
    entry: MeshEntry,
}

impl Drawing {
    /// Keeps `bytes`, of one item for each place, and draws them through
    /// `table` from its rows' first.
    fn kept(
        &mut self,
        (at, pipeline): (&Target<'_>, &mut MeshPipeline),
        bytes: &[u8],
        table: &mut Table,
    ) {
        let count = bytes.len() / MeshPipeline::INSTANCE_BYTES;
        let drawn = vec![&self.entry; count];
        let range = pipeline.keep(at, bytes).expect("room");
        let place = (&mut table.transforms, table.rows_base);
        pipeline.draw_kept(at, &range, place, (&mut self.entries, &drawn));
        pipeline.release(range);
    }
}

/// The pixels of `shape` drawn at each of `places` from item bytes made
/// by `bytes` once the table `setup` makes is made, through that table.
fn drawn(places: &[[f32; 2]], bytes: Bytes, setup: Setup) -> Vec<u8> {
    on_square(|at, pipeline, drawing| {
        let items: Vec<_> = places.iter().map(|&place| item(place)).collect();
        let mut table = setup(at);
        let bytes = bytes(&items, &table);
        drawing.kept((at, pipeline), &bytes, &mut table);
    })
}

/// Makes a draw's item bytes, which may name the table's entries.
type Bytes = fn(&[MeshShape], &Table) -> Vec<u8>;

/// The items' bytes on the page.
fn on_page(items: &[MeshShape], _: &Table) -> Vec<u8> {
    MeshPipeline::bytes(items)
}

/// Makes the table a draw reads.
type Setup = fn(&Target<'_>) -> Table;
#[test]
fn gpu_a_kept_mesh_in_a_frame_entry_matches_the_mesh_drawn_at_the_moved_place() {
    // Given
    let moved = drawn(&[[7.0, 9.0]], on_page, |at| table(at, [0.0; 2], &[]));

    // When: kept at 4, 4 in a frame shifted by 3, 5
    let shifted = drawn(
        &[[4.0, 4.0]],
        |items, table| MeshPipeline::bytes_in(items, table.frame, &[]),
        |at| table(at, [3.0, 5.0], &[]),
    );

    // Then
    assert!(moved.contains(&255), "the mesh drew nothing");
    assert!(
        shifted == moved,
        "the shifted mesh differs from the moved one"
    );
}

#[test]
fn gpu_kept_meshes_in_a_lanes_rows_match_the_meshes_drawn_at_their_rows_places() {
    // Given
    let places = [[4.0, 3.0], [4.0, 13.0], [4.0, 23.0]];
    let moved = drawn(&places, on_page, |at| table(at, [0.0; 2], &[]));

    // When: each kept at 4, 3, in rows 0, 2 and 1 of rows 0, 20 and 10 down
    let rowed = drawn(
        &[[4.0, 3.0]; 3],
        |items, table| MeshPipeline::bytes_in(items, table.frame, &[0, 2, 1]),
        |at| table(at, [0.0; 2], &[[0.0, 0.0], [0.0, 20.0], [0.0, 10.0]]),
    );

    // Then
    assert!(
        rowed == moved,
        "the rowed meshes differ from the moved ones"
    );
}

#[test]
#[should_panic(expected = "a pipeline draws from one table")]
fn gpu_a_mesh_pipeline_given_a_second_transform_table_refuses_it() {
    let _ = on_square(|at, pipeline, drawing| {
        let bytes = MeshPipeline::bytes(&[item([4.0, 4.0])]);
        drawing.kept((at, pipeline), &bytes, &mut table(at, [0.0; 2], &[]));
        drawing.kept((at, pipeline), &bytes, &mut table(at, [0.0; 2], &[]));
    });
}
