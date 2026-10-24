//! The editor's preview (M5.7, `tasks/plans/m5-editor-v1.md` §Client side "Preview"): the
//! edited map as the game draws it, seen from the cursor's cell the way the cursor faces, built
//! with no world: every door shut, every object as placed (hidden ones shown: an author sees
//! everything), every standing group in place. The scene is the game's own (`Scene::build_from`
//! with `AsPlaced` answering for the world), so what the preview shows is what a game would.
//! Bevy-free; the shell draws it into an image (`shell/preview.rs`).

use crate::geom::{cell_centre, yaw_of};
use crate::ground::eye;
use crate::pose::Pose;
use crate::scene::{Occupancy, Scene};
use omnis_sim::api::{BeingView, PileView, SeenObject, placed_on_level, standing_as_placed};
use omnis_sim::omnis_core::{Facing, Level, MapId};
use omnis_sim::omnis_data::{Data, MapData};

/// How far the preview sees, in cells: the editor has no party to see for it.
pub const SIGHT: f32 = 8.0;

/// A map as placed: the scene's world with no game running.
#[derive(Debug, Clone, Copy)]
pub struct AsPlaced<'a> {
    /// The map, as the loader last took it.
    pub map: &'a MapData,
}

impl Occupancy for AsPlaced<'_> {
    /// Every door shut.
    fn door_open(&self, _: MapId, _: Level, _: u16, _: u16, _: Facing) -> bool {
        false
    }

    fn objects_on(&self, data: &Data, _: MapId, level: Level) -> (Vec<SeenObject>, Vec<PileView>) {
        placed_on_level(data, self.map, level)
    }

    fn beings_on(&self, data: &Data, _: MapId, level: Level) -> Vec<BeingView> {
        standing_as_placed(data, self.map, level)
    }
}

/// The id `map` has among the loaded packs, or the one it would be given next (only names
/// depend on it, as `MapData::resolve` does).
#[must_use]
pub fn map_id(data: &Data, map: &MapData) -> MapId {
    data.registry
        .maps
        .get(&map.def.id)
        .unwrap_or_else(|| MapId::from(u32::try_from(data.registry.maps.len()).unwrap_or(u32::MAX)))
}

/// `level` of `map` as the game would draw it with every door shut and everything as placed.
#[must_use]
pub fn scene(data: &Data, map: &MapData, level: Level) -> Scene {
    Scene::build_from(data, map, map_id(data, map), level, &AsPlaced { map })
}

/// Standing on `cursor`'s cell, looking along `facing`.
#[must_use]
pub fn pose(cursor: (u16, u16), facing: Facing) -> Pose {
    let (x, z) = cell_centre(cursor.0, cursor.1);
    Pose {
        x,
        z,
        yaw: yaw_of(facing),
    }
}

/// Where the preview's eye stands over `scene`'s ground at `pose`, and its yaw.
#[must_use]
pub fn look(scene: &Scene, pose: Pose) -> ([f32; 3], f32) {
    ([pose.x, eye(&scene.ground, pose), pose.z], pose.yaw)
}
