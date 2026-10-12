//! What the view draws of a map (presentation-ARCHITECTURE.md §8.4, M8.5 step 1b): the party's
//! level on its ground, and the levels just above and below it, drawn dimmer at their own
//! heights, so a bridge shows over the road and the cellar through a hole. The rules stay per
//! level; only the drawing crosses. Each level brings its fills (M8.5 step 3), so the party's
//! floors and ceilings hide its neighbours except through open air, holes and shafts.
//! Bevy-free, so it is tested directly.

use crate::fill::{Tri, fills};
use crate::geometry::{SegKind, Segment, extract};
use crate::ground::Ground;
use crate::sim::SimWorld;
use omnis_sim::omnis_core::{Level, MapId};
use omnis_sim::omnis_data::MapData;

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
    /// `level` of `map` and the levels beside it, doors as `world` has them.
    #[must_use]
    pub fn build(map: &MapData, map_id: MapId, level: Level, world: &SimWorld) -> Scene {
        let ground = Ground::from_map(map, level);
        let own = lines(map, map_id, level, world, &ground);
        let mut all = fills(map, map_id, level, world, &ground);
        let mut neighbours = Vec::new();
        for near in [false, true]
            .into_iter()
            .filter_map(|up| map.beyond(level, up))
        {
            let ground = Ground::from_map(map, near);
            neighbours.extend(lines(map, map_id, near, world, &ground));
            all.extend(fills(map, map_id, near, world, &ground));
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

/// A level's lines, without its ceiling grid where the level above has ground on the cell: that
/// floor is drawn instead.
fn lines(
    map: &MapData,
    map_id: MapId,
    level: Level,
    world: &SimWorld,
    ground: &Ground,
) -> Vec<Segment> {
    let above = map.beyond(level, true);
    let mut out = extract(map, map_id, level, world, ground);
    out.retain(|s| !(s.kind == SegKind::Ceiling && above.is_some_and(|a| covered(map, a, s))));
    out
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
