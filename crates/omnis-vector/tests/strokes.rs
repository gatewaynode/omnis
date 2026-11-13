//! The view's lines as vertex buffers (rendering size 1): a quad a segment, each vertex with
//! both ends, its corner, the colour with the level's brightness, and the plain style; in the
//! scene's own order, since of two lines in one place the first in the mesh shows.

use crate::common;

use common::{SEED, levels_data, map};
use omnis_sim::Settings;
use omnis_sim::omnis_core::Level;
use omnis_sim::omnis_data::ColourRole;
use omnis_vector::geometry::Named;
use omnis_vector::geometry::{SegKind, Segment};
use omnis_vector::scene::{NEIGHBOUR_DIM, Scene};
use omnis_vector::sim::SimWorld;
use omnis_vector::strokes::{
    AIM, CORNERS, CREST, Look, QUAD, Strokes, channels, look, role, strength,
};
use omnis_vector::style::{Palette, Pattern, SLOTS, Weight, linear};

fn segment(kind: SegKind, role: Named) -> Segment {
    Segment {
        a: [1.0, 0.0, 2.0],
        b: [1.0, 1.0, 2.5],
        kind,
        role,
    }
}

#[test]
fn a_segment_is_a_quad_of_four_vertices_with_both_ends_and_two_triangles() {
    let wall = segment(SegKind::Wall, None);
    let door = Segment {
        a: [4.0, 0.0, 4.0],
        b: [5.0, 0.0, 4.0],
        ..segment(SegKind::Door, None)
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
    // The brightness rides in the alpha; the style names the role: structure, then a passage.
    let palette = Palette::default();
    let [r, g, b] = palette.ink(ColourRole::Structure);
    assert_eq!(strokes.colour[..4], [[r, g, b, 1.0]; 4]);
    let [r, g, b] = palette.ink(ColourRole::Passage);
    assert_eq!(strokes.colour[4..], [[r, g, b, 0.5]; 4]);
    assert_eq!(strokes.style[..4], [[1.0, 0.0, 0.0, 0.0]; 4]);
    assert_eq!(strokes.style[4..], [[1.0, 0.0, 2.0, 0.0]; 4]);
    assert!(Strokes::default().is_empty());
}

#[test]
fn every_kind_has_its_role_and_a_terrain_s_named_role_reaches_its_own_lines() {
    use ColourRole::{Danger, Ground, Hostile, Magic, Passage, Structure, Thing};
    use SegKind::{
        Activity, Barrier, Being, Block, Ceiling, Cliff, Door, Floor, Hole, Ladder, Object, Pile,
        Portal, Ramp, Stair, Wall,
    };
    // The owner's table (2026-11-13): the role a kind has, and whether a terrain that names a
    // role gives it to that kind.
    let table = [
        (vec![Wall, Cliff], Structure, false),
        (vec![Block, Barrier], Structure, true),
        (vec![Floor, Ceiling, Ramp], Ground, true),
        (vec![Door, Portal, Stair, Ladder], Passage, false),
        (vec![Object, Pile, Activity], Thing, false),
        (vec![Being], Hostile, false),
        (vec![Hole], Danger, false),
    ];
    let mut kinds = 0;
    for (row, own, named) in table {
        for kind in row {
            kinds += 1;
            assert_eq!(role(&segment(kind, None)), own, "{kind:?}");
            let on_magic = role(&segment(kind, Some(Magic)));
            assert_eq!(
                on_magic,
                if named { Magic } else { own },
                "{kind:?} on a ley pool"
            );
            // The role's number is what the shader reads, the same at each corner.
            let strokes = Strokes::of([(&segment(kind, Some(Magic)), 1.0)]);
            let third: Vec<f32> = strokes.style.iter().map(|s| s[2]).collect();
            #[allow(clippy::cast_precision_loss)]
            let number = on_magic.index() as f32;
            assert_eq!(third, [number; 4], "{kind:?}");
        }
    }
    assert_eq!(kinds, 16, "every kind of line has its row");
    // Strength: the ceiling grid is fainter than the floor's, a ramp's treads and a crest
    // brighter than the grid they lie on.
    assert!(strength(Ceiling) < strength(Floor));
    assert!(strength(Ramp) > strength(Floor) && CREST.strength > strength(Floor));
    assert_eq!(strength(Wall), 1.0);
    assert_eq!((CREST.role, AIM.role), (Ground, Thing));
}

#[test]
fn a_palette_is_the_biosphere_s_inks_over_the_default() {
    let default = Palette::default();
    assert_eq!(Palette::of(None), default);
    for role in ColourRole::ALL {
        let ink = default.ink(role);
        assert!(ink.iter().any(|c| *c > 0.0), "{role:?} is drawn");
        for other in ColourRole::ALL {
            assert!(
                other == role || default.ink(other) != ink,
                "{role:?} and {other:?}"
            );
        }
    }
    assert!(ColourRole::ALL.len() <= SLOTS);
    assert_eq!(default.inks[ColourRole::ALL.len()..], [[0.0; 3]; 7]);

    // sRGB as a pack writes it, to linear light.
    assert_eq!((linear(0), linear(255)), (0.0, 1.0));
    assert!((linear(128) - 0.2158).abs() < 1e-3);

    // Every role written: the yard's ember. Structure is (255, 120, 40) at 2800 per mille.
    let objects = common::objects_data();
    let ember = Palette::of(objects.biosphere_of(map(&objects, "objects:map:yard")));
    let [r, g, b] = ember.ink(ColourRole::Structure);
    assert!((r - 2.8).abs() < 1e-5 && (g - linear(120) * 2.8).abs() < 1e-5);
    assert!((b - linear(40) * 2.8).abs() < 1e-5);
    // A glow left out is the colour as written.
    assert_eq!(ember.ink(ColourRole::Interface)[0], 1.0);
    for role in ColourRole::ALL {
        assert_ne!(
            ember.ink(role),
            default.ink(role),
            "{role:?} is ember's own"
        );
    }

    // Two roles written: the tower's deep. The rest are the default's.
    let levels = levels_data();
    let deep = Palette::of(levels.biosphere_of(map(&levels, "levels:map:tower")));
    assert_ne!(
        deep.ink(ColourRole::Structure),
        default.ink(ColourRole::Structure)
    );
    assert_ne!(
        deep.ink(ColourRole::Passage),
        default.ink(ColourRole::Passage)
    );
    assert_eq!(
        deep.ink(ColourRole::Ground),
        default.ink(ColourRole::Ground)
    );
    assert_eq!(
        deep.ink(ColourRole::Danger),
        default.ink(ColourRole::Danger)
    );
}

#[test]
fn a_map_s_lines_carry_the_roles_its_terrains_name() {
    // The yard's loft floor names structure; its grass names nothing.
    let data = common::objects_data();
    let world = SimWorld::start(&data, SEED, Settings::default()).expect("an entry map");
    let scene = Scene::build(&data, map(&data, "objects:map:yard"), Level::GROUND, &world);
    let floors = |lines: &[Segment]| -> Vec<Named> {
        let mut roles: Vec<Named> = (lines.iter())
            .filter(|s| s.kind == SegKind::Floor)
            .map(|s| s.role)
            .collect();
        roles.sort();
        roles.dedup();
        roles
    };
    assert_eq!(
        floors(&scene.own),
        [None],
        "the ground is grass and a ladder"
    );
    assert!(
        floors(&scene.neighbours).contains(&Some(ColourRole::Structure)),
        "the loft's floor"
    );
    // The tower's hole names danger: its floor grid is drawn as one.
    let data = levels_data();
    let world = SimWorld::start(&data, SEED, Settings::default()).expect("an entry map");
    let scene = Scene::build(&data, map(&data, "levels:map:tower"), Level(1), &world);
    let danger = (scene.own.iter())
        .filter(|s| s.kind == SegKind::Floor && s.role == Some(ColourRole::Danger))
        .count();
    assert!(danger > 0, "the hole's cell on the upper floor");
    assert!(
        (scene.own.iter()).any(|s| s.kind == SegKind::Floor && s.role.is_none()),
        "and plain floor beside it"
    );
}

#[test]
fn a_kind_s_weight_and_pattern_ride_in_every_vertex_s_style() {
    use SegKind::{
        Activity, Barrier, Being, Block, Ceiling, Cliff, Door, Floor, Hole, Ladder, Object, Pile,
        Portal, Ramp, Stair, Wall,
    };
    // The provisional table (presentation-PRD 5.3, until M8.5 step 5): structure plain, the
    // grids hairline, what can be used heavy and dashed, beings heavy with two rails, a
    // portal's mark dotted.
    let table = [
        (
            vec![
                Wall, Door, Block, Barrier, Cliff, Ramp, Stair, Ladder, Hole, Activity,
            ],
            (Weight::Normal, Pattern::Solid),
        ),
        (vec![Floor, Ceiling], (Weight::Hairline, Pattern::Solid)),
        (vec![Object, Pile], (Weight::Heavy, Pattern::Dashed)),
        (vec![Being], (Weight::Heavy, Pattern::Double)),
        (vec![Portal], (Weight::Normal, Pattern::Dotted)),
    ];
    let mut kinds = 0;
    for (row, expected) in table {
        for kind in row {
            kinds += 1;
            assert_eq!(channels(kind), expected, "{kind:?}");
            let strokes = Strokes::of([(&segment(kind, None), 1.0)]);
            let (weight, pattern) = expected;
            let head: Vec<[f32; 2]> = strokes.style.iter().map(|s| [s[0], s[1]]).collect();
            assert_eq!(
                head,
                [[weight.factor(), pattern.code()]; 4],
                "{kind:?}: the same style at each corner"
            );
            assert!(
                strokes.style.iter().all(|s| s[3] == 0.0),
                "the party's level"
            );
        }
    }
    assert_eq!(kinds, 16, "every kind of line has its row");

    // What the shader reads: a factor on the width, and a code a pattern, solid being none.
    assert_eq!(
        [Weight::Hairline, Weight::Normal, Weight::Heavy].map(Weight::factor),
        [0.5, 1.0, 1.5]
    );
    assert_eq!(
        [
            Pattern::Solid,
            Pattern::Dashed,
            Pattern::Dotted,
            Pattern::Double
        ]
        .map(Pattern::code),
        [0.0, 1.0, 2.0, 3.0]
    );
    // A line pushed by hand carries the look it is given.
    let mut strokes = Strokes::default();
    let look = Look {
        role: ColourRole::Danger,
        strength: 1.0,
        weight: Weight::Heavy,
        pattern: Pattern::Dotted,
    };
    strokes.push([0.0; 3], [1.0; 3], look, 1.0);
    assert_eq!(strokes.style, [[1.5, 2.0, 7.0, 0.0]; 4]);
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
            let look = look(segment);
            let [r, g, b] = Palette::default().ink(look.role).map(|c| c * look.strength);
            assert_eq!(strokes.a[i * 4], segment.a, "line {i} in the scene's order");
            assert_eq!(strokes.b[i * 4 + 3], segment.b);
            assert_eq!(strokes.colour[i * 4], [r, g, b, *dim]);
            assert_eq!(strokes.style[i * 4], look.style());
        }
        let dims: Vec<f32> = strokes.colour.iter().map(|c| c[3]).collect();
        assert!(dims.contains(&1.0) && dims.contains(&NEIGHBOUR_DIM));
    }
    let shown = Strokes::of(scene.strokes(true)).len();
    let hidden = Strokes::of(scene.strokes(false)).len();
    assert!(hidden < shown, "the hidden grid leaves fewer lines");
}
