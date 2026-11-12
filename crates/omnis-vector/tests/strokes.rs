//! The view's lines as vertex buffers (rendering size 1): a quad a segment, each vertex with
//! both ends, its corner, the colour with the level's brightness, and the plain style; in the
//! scene's own order, since of two lines in one place the first in the mesh shows.

use crate::common;

use common::{SEED, levels_data, map};
use omnis_sim::Settings;
use omnis_sim::omnis_core::Level;
use omnis_vector::geometry::{SegKind, Segment};
use omnis_vector::scene::{NEIGHBOUR_DIM, Scene};
use omnis_vector::sim::SimWorld;
use omnis_vector::strokes::{CORNERS, PLAIN, QUAD, Strokes, colour};

fn segment(kind: SegKind, rgb: (u8, u8, u8)) -> Segment {
    Segment {
        a: [1.0, 0.0, 2.0],
        b: [1.0, 1.0, 2.5],
        kind,
        rgb,
    }
}

#[test]
fn a_segment_is_a_quad_of_four_vertices_with_both_ends_and_two_triangles() {
    let wall = segment(SegKind::Wall, (0, 0, 0));
    let door = Segment {
        a: [4.0, 0.0, 4.0],
        b: [5.0, 0.0, 4.0],
        ..segment(SegKind::Door, (0, 0, 0))
    };
    let strokes = Strokes::of([(&wall, 1.0), (&door, 0.5)]);
    assert_eq!(strokes.len(), 2);
    assert!(!strokes.is_empty());
    assert_eq!(strokes.a.len(), 8);
    for list in [strokes.b.len(), strokes.corner.len()] {
        assert_eq!(list, 8);
    }
    assert_eq!((strokes.colour.len(), strokes.style.len()), (8, 8));
    // Every vertex of a quad knows both ends; the corners are the four of the quad, in order.
    assert_eq!(strokes.a[..4], [wall.a; 4]);
    assert_eq!(strokes.b[..4], [wall.b; 4]);
    assert_eq!(strokes.a[4..], [door.a; 4]);
    assert_eq!(strokes.b[4..], [door.b; 4]);
    assert_eq!(strokes.corner[..4], CORNERS);
    assert_eq!(strokes.corner[4..], CORNERS);
    assert_eq!(
        CORNERS,
        [[0.0, -1.0], [0.0, 1.0], [1.0, 1.0], [1.0, -1.0]],
        "both sides at each end"
    );
    // Two triangles a quad, the second quad's over its own four vertices.
    assert_eq!(strokes.indices, [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    assert_eq!(QUAD, [0, 1, 2, 0, 2, 3]);
    // The colour is the kind's, the brightness rides in the alpha; the style is plain.
    let (r, g, b) = colour(&wall);
    assert_eq!(strokes.colour[..4], [[r, g, b, 1.0]; 4]);
    let (r, g, b) = colour(&door);
    assert_eq!(strokes.colour[4..], [[r, g, b, 0.5]; 4]);
    assert_eq!(strokes.style, [PLAIN; 8]);
    assert_eq!(PLAIN, [1.0, 0.0, 0.0, 0.0], "weight one, the rest kept");
    assert!(Strokes::default().is_empty());
}

#[test]
fn terrain_kinds_take_the_terrain_s_colour_and_the_rest_their_own() {
    let rgb = (255, 51, 0);
    assert_eq!(colour(&segment(SegKind::Wall, rgb)), (0.15, 3.0, 0.5));
    assert_eq!(colour(&segment(SegKind::Block, rgb)), (3.0, 0.6, 0.0));
    let (r, g, b) = colour(&segment(SegKind::Floor, rgb));
    assert!((r - 0.6).abs() < 1e-6 && (g - 0.12).abs() < 1e-6 && b == 0.0);
    let (r, _, _) = colour(&segment(SegKind::Ceiling, rgb));
    assert!(
        (r - 0.3).abs() < 1e-6,
        "the ceiling grid is dimmer than the floor's"
    );
}

#[test]
fn a_scene_s_buffers_keep_its_order_its_dims_and_the_grid_setting() {
    let data = levels_data();
    let world = SimWorld::start(&data, SEED, Settings::default()).expect("an entry map");
    let scene = Scene::build(&data, map(&data, "levels:map:tower"), Level::GROUND, &world);
    assert!(
        !scene.neighbours.is_empty(),
        "the tower has levels beside the ground"
    );
    for grid in [true, false] {
        let lines: Vec<(&Segment, f32)> = scene.strokes(grid).collect();
        let strokes = Strokes::of(scene.strokes(grid));
        assert_eq!(strokes.len(), lines.len());
        for (i, (segment, dim)) in lines.iter().enumerate() {
            let (r, g, b) = colour(segment);
            assert_eq!(strokes.a[i * 4], segment.a, "line {i} in the scene's order");
            assert_eq!(strokes.b[i * 4 + 3], segment.b);
            assert_eq!(strokes.colour[i * 4], [r, g, b, *dim]);
        }
        let dims: Vec<f32> = strokes.colour.iter().map(|c| c[3]).collect();
        assert!(dims.contains(&1.0) && dims.contains(&NEIGHBOUR_DIM));
    }
    let shown = Strokes::of(scene.strokes(true)).len();
    let hidden = Strokes::of(scene.strokes(false)).len();
    assert!(hidden < shown, "the hidden grid leaves fewer lines");
}
