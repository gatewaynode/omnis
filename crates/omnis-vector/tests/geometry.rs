//! The geometry against a copy of the flat extractor it replaced (M8.5 step 4): on every
//! shipped map, all flat, `extract` on the ground gives exactly the old segments in the old order;
//! on the elevation fixture, cliffs have faces, ramps have treads, and walls stand on the ground.

use crate::common;

use common::{data, heights_data, map};
use omnis_sim::Settings;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_vector::geometry::{SegKind, Segment, extract};
use omnis_vector::ground::Ground;
use omnis_vector::sim::SimWorld;

/// `geometry.rs` as it stood before height was drawn (commit `5c0a015`), kept as the reference.
#[allow(clippy::too_many_arguments)]
mod flat {
    use omnis_sim::omnis_core::{Facing, MapId};
    use omnis_sim::omnis_data::MapData;
    use omnis_vector::geom::wall_height;
    use omnis_vector::geometry::{SegKind, Segment};
    use omnis_vector::sim::SimWorld;
    use std::collections::BTreeMap;

    /// A canonical edge: `(horizontal, column, row)` of its lower-left corner.
    type EdgeKey = (bool, u16, u16);

    /// What stands on an edge: its kind, whether a door is open, the terrain colour, the height.
    type EdgeLook = (SegKind, bool, (u8, u8, u8), f32);

    /// The segments for `map`, with doors drawn open or closed as `world` has them.
    #[must_use]
    pub fn extract_flat(map: &MapData, map_id: MapId, world: &SimWorld) -> Vec<Segment> {
        let mut out = Vec::new();
        let mut edges: BTreeMap<EdgeKey, EdgeLook> = BTreeMap::new();
        for y in 0..map.def.height {
            for x in 0..map.def.width {
                let Some(cell) = map.cell(omnis_sim::omnis_core::Level::GROUND, x, y) else {
                    continue;
                };
                let terrain = map.terrain(cell);
                let h = wall_height(terrain);
                cells(
                    &mut out,
                    x,
                    y,
                    terrain.color,
                    terrain.passable,
                    terrain.block.is_some(),
                    terrain.ceiling.is_some(),
                    h,
                );
                for facing in Facing::ALL {
                    let kind = if cell.doors.has(facing) {
                        SegKind::Door
                    } else if cell.walls.has(facing) {
                        SegKind::Wall
                    } else {
                        continue;
                    };
                    let open =
                        world.door_open(map_id, omnis_sim::omnis_core::Level::GROUND, x, y, facing);
                    let entry =
                        edges
                            .entry(key(x, y, facing))
                            .or_insert((kind, open, terrain.color, h));
                    // A door on either side wins over a wall on the other.
                    if kind == SegKind::Door {
                        *entry = (kind, open, terrain.color, h);
                    }
                }
            }
        }
        for (&(horizontal, col, row), &(kind, open, rgb, h)) in &edges {
            let (x0, z0) = (f32::from(col), f32::from(row));
            let (x1, z1) = if horizontal {
                (x0 + 1.0, z0)
            } else {
                (x0, z0 + 1.0)
            };
            edge(&mut out, [x0, z0], [x1, z1], kind, open, rgb, h);
        }
        out
    }

    fn key(x: u16, y: u16, facing: Facing) -> EdgeKey {
        match facing {
            Facing::North => (true, x, y),
            Facing::South => (true, x, y + 1),
            Facing::West => (false, x, y),
            Facing::East => (false, x + 1, y),
        }
    }

    fn line(out: &mut Vec<Segment>, a: [f32; 3], b: [f32; 3], kind: SegKind, rgb: (u8, u8, u8)) {
        out.push(Segment { a, b, kind, rgb });
    }

    /// The per-cell lines: the floor grid's north and west sides (the far sides come from the
    /// neighbours, or from the walls that close the map), a block's box, a barrier's cross, and the
    /// ceiling grid.
    #[allow(clippy::too_many_arguments)]
    fn cells(
        out: &mut Vec<Segment>,
        x: u16,
        y: u16,
        rgb: (u8, u8, u8),
        passable: bool,
        block: bool,
        ceiling: bool,
        h: f32,
    ) {
        let (x0, z0) = (f32::from(x), f32::from(y));
        let (x1, z1) = (x0 + 1.0, z0 + 1.0);
        line(out, [x0, 0.0, z0], [x1, 0.0, z0], SegKind::Floor, rgb);
        line(out, [x0, 0.0, z0], [x0, 0.0, z1], SegKind::Floor, rgb);
        if ceiling && !block {
            line(out, [x0, h, z0], [x1, h, z0], SegKind::Ceiling, rgb);
            line(out, [x0, h, z0], [x0, h, z1], SegKind::Ceiling, rgb);
        }
        if block {
            let i = 0.1;
            boxed(out, [x0 + i, z0 + i], [x1 - i, z1 - i], h, rgb);
        } else if !passable {
            line(out, [x0, 0.0, z0], [x1, 0.0, z1], SegKind::Barrier, rgb);
            line(out, [x1, 0.0, z0], [x0, 0.0, z1], SegKind::Barrier, rgb);
        }
    }

    /// The twelve edges of a box over the ground rectangle `lo..hi`.
    fn boxed(out: &mut Vec<Segment>, lo: [f32; 2], hi: [f32; 2], h: f32, rgb: (u8, u8, u8)) {
        let corners = [
            [lo[0], lo[1]],
            [hi[0], lo[1]],
            [hi[0], hi[1]],
            [lo[0], hi[1]],
        ];
        for i in 0..4 {
            let (p, q) = (corners[i], corners[(i + 1) % 4]);
            line(
                out,
                [p[0], 0.0, p[1]],
                [q[0], 0.0, q[1]],
                SegKind::Block,
                rgb,
            );
            line(out, [p[0], h, p[1]], [q[0], h, q[1]], SegKind::Block, rgb);
            line(out, [p[0], 0.0, p[1]], [p[0], h, p[1]], SegKind::Block, rgb);
        }
    }

    /// A wall's rectangle, or a door: the lintel over a frame, crossed while closed.
    fn edge(
        out: &mut Vec<Segment>,
        a: [f32; 2],
        b: [f32; 2],
        kind: SegKind,
        open: bool,
        rgb: (u8, u8, u8),
        h: f32,
    ) {
        let at = |t: f32, y: f32| [a[0] + (b[0] - a[0]) * t, y, a[1] + (b[1] - a[1]) * t];
        match kind {
            SegKind::Door => {
                let (l, r, top) = (0.2, 0.8, h * 0.85);
                line(out, at(0.0, h), at(1.0, h), SegKind::Wall, rgb);
                line(out, at(0.0, 0.0), at(0.0, h), SegKind::Wall, rgb);
                line(out, at(1.0, 0.0), at(1.0, h), SegKind::Wall, rgb);
                line(out, at(l, 0.0), at(l, top), kind, rgb);
                line(out, at(r, 0.0), at(r, top), kind, rgb);
                line(out, at(l, top), at(r, top), kind, rgb);
                if !open {
                    line(out, at(l, 0.0), at(r, top), kind, rgb);
                    line(out, at(r, 0.0), at(l, top), kind, rgb);
                }
            }
            _ => {
                line(out, at(0.0, 0.0), at(1.0, 0.0), kind, rgb);
                line(out, at(0.0, h), at(1.0, h), kind, rgb);
                line(out, at(0.0, 0.0), at(0.0, h), kind, rgb);
                line(out, at(1.0, 0.0), at(1.0, h), kind, rgb);
            }
        }
    }
}

fn world(data: &Data) -> SimWorld {
    SimWorld::start(data, common::SEED, Settings::default()).expect("an entry map")
}

#[test]
fn on_the_flat_maps_the_lines_are_what_they_were() {
    let data = data();
    let world = world(&data);
    for (&id, m) in &data.maps {
        let now = extract(
            m,
            id,
            omnis_sim::omnis_core::Level::GROUND,
            &world,
            &Ground::from_map(m, omnis_sim::omnis_core::Level::GROUND),
        );
        assert_eq!(now, flat::extract_flat(m, id, &world), "{}", m.def.id);
    }
}

fn hills() -> (Data, Vec<Segment>) {
    let data = heights_data();
    let id = map(&data, "heights:map:hills");
    let m = &data.maps[&id];
    let lines = extract(
        m,
        id,
        omnis_sim::omnis_core::Level::GROUND,
        &world(&data),
        &Ground::from_map(m, omnis_sim::omnis_core::Level::GROUND),
    );
    (data, lines)
}

/// The cliff edges of the fixture as `(horizontal, column, row)`, from either side.
fn cliff_edges(data: &Data) -> Vec<(bool, u16, u16)> {
    let m = &data.maps[&map(data, "heights:map:hills")];
    let mut out = Vec::new();
    for y in 0..m.def.height {
        for x in 0..m.def.width {
            for (facing, key) in [
                (Facing::East, (false, x + 1, y)),
                (Facing::South, (true, x, y + 1)),
            ] {
                let cell = m
                    .cell(omnis_sim::omnis_core::Level::GROUND, x, y)
                    .expect("a cell");
                let Some(next) = (if facing == Facing::East {
                    m.cell(omnis_sim::omnis_core::Level::GROUND, x + 1, y)
                } else {
                    m.cell(omnis_sim::omnis_core::Level::GROUND, x, y + 1)
                }) else {
                    continue;
                };
                if cell.cliffs.has(facing) || next.cliffs.has(facing.opposite()) {
                    out.push(key);
                }
            }
        }
    }
    out
}

/// The canonical edge a vertical cliff line stands on.
fn edge_of(s: &Segment) -> (bool, u16, u16) {
    let (x, z) = (s.a[0], s.a[2]);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    if x.fract() == 0.0 && z.fract() != 0.0 {
        (false, x as u16, z.floor() as u16)
    } else if z.fract() == 0.0 && x.fract() != 0.0 {
        (true, x.floor() as u16, z as u16)
    } else {
        // At a corner: the end of an edge; matched by its other lines.
        (false, x as u16, z as u16)
    }
}

#[test]
fn every_cliff_has_a_hatched_face_and_every_face_is_on_a_cliff() {
    let (data, lines) = hills();
    let cliffs = cliff_edges(&data);
    assert!(cliffs.len() > 10, "{} cliffs", cliffs.len());
    let faces: Vec<_> = lines.iter().filter(|s| s.kind == SegKind::Cliff).collect();
    for s in &faces {
        assert_eq!(
            (s.a[0], s.a[2]),
            (s.b[0], s.b[2]),
            "a face line is vertical: {s:?}"
        );
        let on = cliffs.iter().any(|&(h, c, r)| {
            let (c, r) = (f32::from(c), f32::from(r));
            if h {
                s.a[2] == r && (c..=c + 1.0).contains(&s.a[0])
            } else {
                s.a[0] == c && (r..=r + 1.0).contains(&s.a[2])
            }
        });
        assert!(on, "on a cliff edge: {s:?} ({:?})", edge_of(s));
    }
    // The slope's top: the edge at x 3 on row 0 rises from the slope to 12, hatched every
    // quarter (the corner at z 1 is shared with the cliff along row line 1).
    let mut at: Vec<f32> = faces
        .iter()
        .filter(|s| s.a[0] == 3.0 && (0.0..=1.0).contains(&s.a[2]))
        .map(|s| s.a[2])
        .collect();
    at.dedup();
    assert_eq!(at, [0.0, 0.25, 0.5, 0.75, 1.0]);
    for &(h, c, r) in &cliffs {
        let (c, r) = (f32::from(c), f32::from(r));
        let hatched = faces.iter().any(|s| {
            if h {
                s.a[2] == r && s.a[0] > c && s.a[0] < c + 1.0
            } else {
                s.a[0] == c && s.a[2] > r && s.a[2] < r + 1.0
            }
        });
        assert!(hatched, "a face on {h} {c} {r}");
    }
}

#[test]
fn the_ramp_has_treads_climbing_with_the_ground() {
    let (_, lines) = hills();
    let treads: Vec<_> = lines.iter().filter(|s| s.kind == SegKind::Ramp).collect();
    assert_eq!(treads.len(), 16, "four treads of four pieces");
    // The ramp joins (0, 2) at 0 and (1, 2) at 16, along x = 1: treads run north to south.
    assert!(
        treads
            .iter()
            .all(|s| s.a[0] == s.b[0] && (2.0..=3.0).contains(&s.a[2]))
    );
    let rise = |x: f32| {
        treads
            .iter()
            .find(|s| (s.a[0] - x).abs() < 1e-6 && (s.a[2] - 2.5).abs() < 1e-6)
            .map(|s| s.a[1])
            .expect("a tread piece at the middle")
    };
    assert!(rise(0.625) < rise(0.875) && rise(0.875) < rise(1.125) && rise(1.125) < rise(1.375));
}

#[test]
fn a_wall_stands_on_the_ground_and_tops_the_higher_side() {
    let (_, lines) = hills();
    // The map's north wall over (3, 0), at 12: its foot at 1.5, its top a wall above.
    let wall: Vec<_> = lines
        .iter()
        .filter(|s| {
            s.kind == SegKind::Wall
                && s.a[2] == 0.0
                && s.b[2] == 0.0
                && s.a[0] == 3.0
                && s.b[0] == 4.0
        })
        .collect();
    let ys: Vec<_> = wall.iter().map(|s| (s.a[1], s.b[1])).collect();
    assert_eq!(ys, [(1.5, 1.5), (2.5, 2.5)], "foot, then top");
}
