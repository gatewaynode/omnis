//! What the view draws of a map (presentation-ARCHITECTURE.md §8.4, M8.5 step 1b): the party's
//! level on its ground, and the levels just above and below it, drawn dimmer at their own
//! heights, so a bridge shows over the road and the cellar through a hole. The rules stay per
//! level; only the drawing crosses. Each level brings its fills (M8.5 step 3), so the party's
//! floors and ceilings hide its neighbours except through open air, holes and shafts. Each
//! level's objects and piles stand on its ground (M8.5 step 2b, `shapes`). What the scene asks
//! of the world is an `Occupancy` (M5.7): the game's world answers from its state, the editor's
//! preview as the map is placed, so the same lines are drawn with no game running.
//! Bevy-free, so it is tested directly.

use crate::fill::{Tri, fills};
use crate::geometry::{SegKind, Segment, activities, extract};
use crate::ground::Ground;
use crate::shapes::{figures, shapes};
use crate::sim::SimWorld;
use omnis_sim::api::{BeingView, PileView, SeenObject};
use omnis_sim::omnis_core::{Facing, Level, MapId};
use omnis_sim::omnis_data::{Data, MapData};

/// What the scene asks of a world about a map: the doors' state and what stands on a level.
/// The game answers from its world (`SimWorld`); the editor's preview as the map is placed
/// (`crate::editor::preview::AsPlaced`).
pub trait Occupancy {
    /// Whether the door on this edge stands open.
    fn door_open(&self, map: MapId, level: Level, x: u16, y: u16, facing: Facing) -> bool;
    /// Every object and pile on the level, as drawn.
    fn objects_on(&self, data: &Data, map: MapId, level: Level)
    -> (Vec<SeenObject>, Vec<PileView>);
    /// Every group standing on the level, as drawn.
    fn beings_on(&self, data: &Data, map: MapId, level: Level) -> Vec<BeingView>;
}

/// How bright a neighbouring level's lines are against the party's own (provisional until the
/// colour roles, M8.5 step 5).
pub const NEIGHBOUR_DIM: f32 = 0.2;

/// The lines of a level and its neighbours.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// The party's level's ground, which the camera stands on.
    pub ground: Ground,
    /// The party's level's lines.
    pub own: Vec<Segment>,
    /// The levels just above and below, drawn at [`NEIGHBOUR_DIM`].
    pub neighbours: Vec<Segment>,
    /// The solid surfaces of the party's level and its neighbours.
    pub fills: Vec<Tri>,
}

impl Scene {
    /// `level` of map `map_id` and the levels beside it, doors and objects as `world` has
    /// them; empty when no pack holds the map.
    #[must_use]
    pub fn build(data: &Data, map_id: MapId, level: Level, world: &SimWorld) -> Scene {
        let Some(map) = data.maps.get(&map_id) else {
            return Scene::default();
        };
        Scene::build_from(data, map, map_id, level, world)
    }

    /// `level` of `map` (which need not be among `data`'s: an edited map, M5.7) and the levels
    /// beside it, doors and objects as `world` has them; `map_id` names the map in what is
    /// drawn.
    #[must_use]
    pub fn build_from(
        data: &Data,
        map: &MapData,
        map_id: MapId,
        level: Level,
        world: &dyn Occupancy,
    ) -> Scene {
        let ground = Ground::from_map(map, level);
        let (own, mut all) = level_of(data, map, (map_id, level), world, &ground);
        let mut neighbours = Vec::new();
        for near in [false, true]
            .into_iter()
            .filter_map(|up| map.beyond(level, up))
        {
            let ground = Ground::from_map(map, near);
            let (lines, fills) = level_of(data, map, (map_id, near), world, &ground);
            neighbours.extend(lines);
            all.extend(fills);
        }
        Scene {
            ground,
            own,
            neighbours,
            fills: all,
        }
    }

    /// Every line drawn with the floor grid `grid` shown or hidden, with its brightness: the
    /// party's level at 1, its neighbours at [`NEIGHBOUR_DIM`].
    pub fn strokes(&self, grid: bool) -> impl Iterator<Item = (&Segment, f32)> {
        let own = self.own.iter().map(|s| (s, 1.0));
        let near = self.neighbours.iter().map(|s| (s, NEIGHBOUR_DIM));
        own.chain(near).filter(move |(s, _)| s.kind.drawn(grid))
    }
}

/// A level's lines and fills, its objects and piles among them, without its ceiling grid where
/// the level above has ground on the cell: that floor is drawn instead.
fn level_of(
    data: &Data,
    map: &MapData,
    (map_id, level): (MapId, Level),
    world: &dyn Occupancy,
    ground: &Ground,
) -> (Vec<Segment>, Vec<Tri>) {
    let mut lines = extract(map, map_id, level, world, ground);
    lines.retain(|s| !hidden_ceiling(map, level, s));
    lines.extend(activities(data, map, level, ground));
    let mut tris = fills(map, map_id, level, world, ground);
    let (objects, piles) = world.objects_on(data, map_id, level);
    let (shape_lines, shape_fills) = shapes(&objects, &piles, ground);
    lines.extend(shape_lines);
    tris.extend(shape_fills);
    let (figure_lines, figure_fills) = figures(&world.beings_on(data, map_id, level), ground);
    lines.extend(figure_lines);
    tris.extend(figure_fills);
    (lines, tris)
}

/// Whether a ceiling line of `level` lies under ground of the level above.
fn hidden_ceiling(map: &MapData, level: Level, s: &Segment) -> bool {
    s.kind == SegKind::Ceiling && map.beyond(level, true).is_some_and(|a| covered(map, a, s))
}

/// Whether a ceiling line lies under ground of level `above`. A cell's ceiling lines both start
/// at its north-west corner.
fn covered(map: &MapData, above: Level, segment: &Segment) -> bool {
    let cell = |v: f32| {
        // A map's columns and rows fit in u16; the floor of a corner is its cell.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let c = v.floor().max(0.0) as u16;
        c
    };
    map.cell(above, cell(segment.a[0]), cell(segment.a[2]))
        .is_some()
}
