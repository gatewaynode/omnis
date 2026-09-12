//! Draw planning: from a `ViewportModel` (or the automap) to an ordered list of sprites and
//! fills in canvas pixels. Pure functions, no Bevy, so the composition rules are unit-tested
//! against the test pack; the renderer only spawns what the plan says.
//!
//! Order within the viewport: rows far to near; within a row floors and ceilings, then side
//! walls from the outer offsets inward, then front walls and doors. Each shared wall plane is
//! drawn once: a tile's left edge at offsets `<= 0`, its right edge at offsets `>= 0`.

use crate::layout::Camera;
use omnis_sim::World;
use omnis_sim::omnis_core::{Facing, MapId};
use omnis_sim::omnis_data::{Data, MapKind, Tileset};
use omnis_sim::query::{EdgeView, ViewportModel};
use omnis_sim::world::layer;

/// What to paint.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// A pack-relative image, drawn at its natural size.
    Sprite(String),
    /// A solid rectangle.
    Fill {
        /// RGB.
        color: (u8, u8, u8),
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
}

/// One draw, top-left in canvas pixels. Later ops paint over earlier ones.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawOp {
    /// What.
    pub paint: Paint,
    /// Canvas x of the top-left corner.
    pub x: i32,
    /// Canvas y of the top-left corner.
    pub y: i32,
}

impl DrawOp {
    fn sprite(path: &str, x: i16, y: i16) -> DrawOp {
        DrawOp {
            paint: Paint::Sprite(path.to_owned()),
            x: i32::from(x),
            y: i32::from(y),
        }
    }

    fn fill(color: (u8, u8, u8), x: i32, y: i32, width: u32, height: u32) -> DrawOp {
        DrawOp {
            paint: Paint::Fill {
                color,
                width,
                height,
            },
            x,
            y,
        }
    }
}

/// The background behind the viewport: sky outdoors, dark underground.
#[must_use]
pub fn backdrop(data: &Data, map: MapId) -> (u8, u8, u8) {
    match data.maps.get(&map).map(|m| m.def.kind) {
        Some(MapKind::Outdoor) | Some(MapKind::Town) => (96, 150, 220),
        _ => (8, 6, 12),
    }
}

/// The viewport draw list.
#[must_use]
pub fn viewport(view: &ViewportModel, data: &Data) -> Vec<DrawOp> {
    let mut ops = Vec::new();
    let (Some(map), Some(tileset)) = (data.maps.get(&view.map), data.tilesets.get(&view.tileset))
    else {
        return ops;
    };
    let camera = Camera::new(tileset.viewport);
    let width = i8::try_from(tileset.width).unwrap_or(i8::MAX);
    let mut tiles: Vec<_> = view
        .tiles
        .iter()
        .filter(|t| t.offset.abs() <= width)
        .collect();
    // Far to near, outer offsets first.
    tiles.sort_by_key(|t| {
        (
            std::cmp::Reverse(t.depth),
            std::cmp::Reverse(t.offset.abs()),
        )
    });

    // Horizon band: everything beyond detail depth as flat colour, far to near.
    for t in tiles.iter().filter(|t| t.depth >= view.detail_depth) {
        let terrain = &map.def.terrains[usize::from(t.terrain)];
        let z_far = f32::from(t.depth) + 1.0;
        let z_near = f32::from(t.depth);
        let x0 = camera.sx(f32::from(t.offset) - 0.5, z_far);
        let x1 = camera.sx(f32::from(t.offset) + 0.5, z_far);
        let fade = shade(
            terrain.color,
            t.depth,
            view.detail_depth,
            view.visibility_depth,
        );
        if terrain.opaque {
            // A silhouette the height of a wall at that distance.
            let y0 = camera.sy(1.0, z_far);
            let y1 = camera.sy(0.0, z_far);
            push_rect(&mut ops, fade, x0, y0, x1, y1);
        } else {
            // The ground between the far and near edges.
            let y0 = camera.sy(0.0, z_far);
            let y1 = camera.sy(0.0, z_near);
            push_rect(&mut ops, fade, x0, y0, x1, y1);
        }
    }

    // Detail rows from tileset slots.
    let mut depth = view.detail_depth;
    while depth > 0 {
        depth -= 1;
        let row: Vec<_> = tiles.iter().filter(|t| t.depth == depth).collect();
        for t in &row {
            let terrain = &map.def.terrains[usize::from(t.terrain)];
            if let Some(path) = tileset.slot(&terrain.floor, depth, t.offset) {
                ops.push(slot_op(tileset, &terrain.floor, depth, t.offset, path));
            }
            if let Some(ceiling) = &terrain.ceiling
                && let Some(path) = tileset.slot(ceiling, depth, t.offset)
            {
                ops.push(slot_op(tileset, ceiling, depth, t.offset, path));
            }
        }
        for t in &row {
            if t.offset <= 0 && t.left != EdgeView::Open {
                push_slot(&mut ops, tileset, &map.def.wall.left, depth, t.offset);
            }
            if t.offset >= 0 && t.right != EdgeView::Open {
                push_slot(&mut ops, tileset, &map.def.wall.right, depth, t.offset);
            }
        }
        for t in &row {
            match t.front {
                EdgeView::Wall => {
                    push_slot(&mut ops, tileset, &map.def.wall.front, depth, t.offset)
                }
                EdgeView::Door { open: false } => {
                    push_slot(&mut ops, tileset, &map.def.door, depth, t.offset)
                }
                EdgeView::Door { open: true } | EdgeView::Open => {}
            }
        }
    }
    ops
}

fn slot_op(tileset: &Tileset, surface: &str, depth: u8, offset: i8, path: &str) -> DrawOp {
    let slot = tileset.surfaces[surface]
        .slots
        .iter()
        .find(|s| s.depth == depth && s.offset == offset);
    let (x, y) = slot.map_or((0, 0), |s| (s.x, s.y));
    DrawOp::sprite(path, x, y)
}

fn push_slot(ops: &mut Vec<DrawOp>, tileset: &Tileset, surface: &str, depth: u8, offset: i8) {
    if let Some(path) = tileset.slot(surface, depth, offset) {
        ops.push(slot_op(tileset, surface, depth, offset, path));
    }
}

fn push_rect(ops: &mut Vec<DrawOp>, color: (u8, u8, u8), x0: f32, y0: f32, x1: f32, y1: f32) {
    let (x0, x1) = (x0.min(x1).round() as i32, x0.max(x1).round() as i32);
    let (y0, y1) = (y0.min(y1).round() as i32, y0.max(y1).round() as i32);
    if x1 > x0 && y1 > y0 {
        ops.push(DrawOp::fill(
            color,
            x0,
            y0,
            (x1 - x0) as u32,
            (y1 - y0) as u32,
        ));
    }
}

/// Darken a colour with distance beyond detail depth: full at the first horizon row, half at
/// the visibility limit.
fn shade(color: (u8, u8, u8), depth: u8, detail: u8, visibility: u8) -> (u8, u8, u8) {
    let span = u32::from(visibility.saturating_sub(detail)).max(1);
    let step = u32::from(depth.saturating_sub(detail)).min(span);
    let scale = |c: u8| ((u32::from(c) * (2 * span - step)) / (2 * span)) as u8;
    (scale(color.0), scale(color.1), scale(color.2))
}

/// The automap for the party's current map at `scale` pixels per tile, top-left at `origin`:
/// known tiles as terrain colour, dimmed unless visited, walls and doors as one-pixel edges,
/// the party as a white mark with a red pixel on its facing edge.
#[must_use]
pub fn automap(world: &World, data: &Data, origin: (i32, i32), scale: i32) -> Vec<DrawOp> {
    let mut ops = Vec::new();
    let map_id = world.position.map;
    let Some(map) = data.maps.get(&map_id) else {
        return ops;
    };
    let s = scale.max(1);
    ops.push(DrawOp::fill(
        (20, 20, 28),
        origin.0 - 1,
        origin.1 - 1,
        (i32::from(map.def.width) * s + 2) as u32,
        (i32::from(map.def.height) * s + 2) as u32,
    ));
    if let Some(known) = world.automap.map(map_id) {
        for (&(x, y), tile) in known {
            let terrain = &map.def.terrains[usize::from(tile.terrain)];
            let color = if tile.layers & layer::VISITED != 0 {
                terrain.color
            } else {
                dim(terrain.color)
            };
            let (px, py) = (origin.0 + i32::from(x) * s, origin.1 + i32::from(y) * s);
            ops.push(DrawOp::fill(color, px, py, s as u32, s as u32));
            for (facing, x0, y0, w, h) in [
                (Facing::North, px, py, s as u32, 1),
                (Facing::South, px, py + s - 1, s as u32, 1),
                (Facing::West, px, py, 1, s as u32),
                (Facing::East, px + s - 1, py, 1, s as u32),
            ] {
                if tile.walls.has(facing) {
                    ops.push(DrawOp::fill((0, 0, 0), x0, y0, w, h));
                } else if tile.doors.has(facing) {
                    ops.push(DrawOp::fill((200, 140, 40), x0, y0, w, h));
                }
            }
        }
    }
    let p = world.position;
    let (cx, cy) = (origin.0 + i32::from(p.x) * s, origin.1 + i32::from(p.y) * s);
    if s >= 4 {
        ops.push(DrawOp::fill(
            (255, 255, 255),
            cx + 1,
            cy + 1,
            (s - 2) as u32,
            (s - 2) as u32,
        ));
    } else {
        ops.push(DrawOp::fill((255, 255, 255), cx, cy, s as u32, s as u32));
    }
    // A red pixel on the facing edge of the party's cell.
    let (dx, dy) = p.facing.delta();
    let along = |d: i32| if d == 0 { s / 2 } else { (d + 1) / 2 * (s - 1) };
    ops.push(DrawOp::fill(
        (255, 80, 80),
        cx + along(dx),
        cy + along(dy),
        1,
        1,
    ));
    ops
}

/// The automap fitted into `rect` (x, y, width, height): centred when the map fits, otherwise
/// scrolled so the party is as central as the map's edges allow, and clipped to the rect.
#[must_use]
pub fn automap_window(
    world: &World,
    data: &Data,
    rect: (i32, i32, u32, u32),
    scale: i32,
) -> Vec<DrawOp> {
    let Some(map) = data.maps.get(&world.position.map) else {
        return Vec::new();
    };
    let s = scale.max(1);
    let (rw, rh) = (rect.2 as i32, rect.3 as i32);
    let (mw, mh) = (i32::from(map.def.width) * s, i32::from(map.def.height) * s);
    let place = |room: i32, size: i32, party: i32| {
        if size <= room {
            (room - size) / 2
        } else {
            (room / 2 - party * s - s / 2).clamp(room - size, 0)
        }
    };
    let origin = (
        rect.0 + place(rw, mw, i32::from(world.position.x)),
        rect.1 + place(rh, mh, i32::from(world.position.y)),
    );
    let mut ops = vec![DrawOp::fill((20, 20, 28), rect.0, rect.1, rect.2, rect.3)];
    ops.extend(
        automap(world, data, origin, s)
            .into_iter()
            .filter_map(|op| clip(op, rect)),
    );
    ops
}

/// A fill cut down to `rect`, or `None` when nothing remains. Sprites pass through.
fn clip(op: DrawOp, rect: (i32, i32, u32, u32)) -> Option<DrawOp> {
    let Paint::Fill {
        color,
        width,
        height,
    } = op.paint
    else {
        return Some(op);
    };
    let (x0, y0) = (op.x.max(rect.0), op.y.max(rect.1));
    let x1 = (op.x + width as i32).min(rect.0 + rect.2 as i32);
    let y1 = (op.y + height as i32).min(rect.1 + rect.3 as i32);
    (x1 > x0 && y1 > y0).then(|| DrawOp::fill(color, x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
}

fn dim(c: (u8, u8, u8)) -> (u8, u8, u8) {
    (c.0 / 2, c.1 / 2, c.2 / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_core::{Direction, Facing, Position};
    use omnis_sim::omnis_data::load_packs;
    use omnis_sim::{Command, apply, query};
    use std::path::PathBuf;

    fn data() -> Data {
        load_packs(&[&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/test")])
            .unwrap_or_else(|r| panic!("{r}"))
    }

    fn sprites(ops: &[DrawOp]) -> Vec<&str> {
        ops.iter()
            .filter_map(|o| match &o.paint {
                Paint::Sprite(p) => Some(p.as_str()),
                Paint::Fill { .. } => None,
            })
            .collect()
    }

    #[test]
    fn meadow_start_draws_far_to_near_with_a_horizon_band() {
        let data = data();
        let world = World::new(&data, 1).unwrap();
        let view = query::viewport(&world, &data).unwrap();
        let ops = viewport(&view, &data);
        let fills = ops
            .iter()
            .filter(|o| matches!(o.paint, Paint::Fill { .. }))
            .count();
        assert!(fills > 0, "grass beyond detail depth is a band of fills");
        let paths = sprites(&ops);
        assert!(
            paths
                .iter()
                .all(|p| p.starts_with("assets/tilesets/outdoor/"))
        );
        let first_sprite = ops
            .iter()
            .position(|o| matches!(o.paint, Paint::Sprite(_)))
            .unwrap();
        assert!(
            ops[..first_sprite]
                .iter()
                .all(|o| matches!(o.paint, Paint::Fill { .. })),
            "band first, sprites over it"
        );
        let d3 = paths.iter().position(|p| p.contains("_d3_")).unwrap();
        let d0 = paths.iter().position(|p| p.contains("_d0_")).unwrap();
        assert!(d3 < d0, "far rows before near rows");
        assert!(
            paths
                .iter()
                .any(|p| p.starts_with("assets/tilesets/outdoor/road_d0_o0"))
        );
        assert!(
            !paths.iter().any(|p| p.contains("hedge")),
            "no walls in the open meadow"
        );
        assert_eq!(backdrop(&data, world.position.map), (96, 150, 220));
    }

    #[test]
    fn dungeon_corridor_draws_each_wall_plane_once() {
        let data = data();
        let mut world = World::new(&data, 1).unwrap();
        let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
        world.position = Position {
            map: dungeon,
            x: 5,
            y: 3,
            facing: Facing::East,
        };
        let view = query::viewport(&world, &data).unwrap();
        let ops = viewport(&view, &data);
        let paths = sprites(&ops);
        assert_eq!(
            paths.last().copied(),
            Some("assets/tilesets/dungeon/door_d0_o0.png"),
            "the closed door ahead is drawn last"
        );
        assert!(paths.contains(&"assets/tilesets/dungeon/floor_d0_o0.png"));
        assert!(paths.contains(&"assets/tilesets/dungeon/ceiling_d0_o0.png"));
        assert!(
            !paths
                .iter()
                .any(|p| p.contains("wall.left_d0_o1") || p.contains("wall.right_d0_o-1")),
            "no plane twice"
        );
        assert_eq!(backdrop(&data, dungeon), (8, 6, 12));
        let placed = ops
            .iter()
            .find(|o| matches!(&o.paint, Paint::Sprite(p) if p.ends_with("door_d0_o0.png")))
            .unwrap();
        assert_eq!(
            (placed.x, placed.y),
            (59, 7),
            "slot position comes from the tileset"
        );
    }

    #[test]
    fn automap_marks_known_tiles_walls_and_the_party() {
        let data = data();
        let mut world = World::new(&data, 1).unwrap();
        world.position = Position {
            map: world.position.map,
            x: 6,
            y: 16,
            facing: Facing::West,
        };
        apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
        let ops = automap(&world, &data, (10, 10), 4);
        assert!(ops.len() > 2);
        let party = &ops[ops.len() - 2];
        assert_eq!((party.x, party.y), (10 + 5 * 4 + 1, 10 + 16 * 4 + 1));
        let mark = &ops[ops.len() - 1];
        assert_eq!(
            (mark.x, mark.y),
            (10 + 5 * 4, 10 + 16 * 4 + 2),
            "facing west: mark on the west edge, mid-height"
        );
        let wall_edges = ops
            .iter()
            .filter(|o| {
                matches!(
                    o.paint,
                    Paint::Fill {
                        color: (0, 0, 0),
                        ..
                    }
                )
            })
            .count();
        assert!(wall_edges > 0, "the hedge is in view to the west");
    }

    #[test]
    fn automap_window_centres_small_maps_and_scrolls_large_ones() {
        let data = data();
        let world = World::new(&data, 1).unwrap();
        let rect = (248, 8, 64, 64);
        let fitted = automap_window(&world, &data, rect, 2);
        let inside = |op: &DrawOp| match op.paint {
            Paint::Fill { width, height, .. } => {
                op.x >= rect.0
                    && op.y >= rect.1
                    && op.x + width as i32 <= rect.0 + 64
                    && op.y + height as i32 <= rect.1 + 64
            }
            Paint::Sprite(_) => false,
        };
        assert!(
            fitted.iter().all(inside),
            "a 32x32 map at 2 px fits the sidebar"
        );
        assert_eq!(
            (fitted[1].x, fitted[1].y),
            (248, 8),
            "a 64 px map fills the sidebar; its border is clipped"
        );
        let scrolled = automap_window(&world, &data, rect, 4);
        assert!(
            scrolled.iter().all(inside),
            "a 128 px map is clipped to the 64 px sidebar"
        );
        let party = &scrolled[scrolled.len() - 2];
        assert_eq!(
            (party.x, party.y),
            (248 + 32 - 2 + 1, 8 + 32 - 2 + 1),
            "party centred"
        );
        let mut edge = world.clone();
        edge.position = Position {
            x: 0,
            y: 0,
            ..edge.position
        };
        let cornered = automap_window(&edge, &data, rect, 4);
        let party = &cornered[cornered.len() - 2];
        assert_eq!(
            (party.x, party.y),
            (249, 9),
            "the map's edge stays at the rect's edge"
        );
    }
}
