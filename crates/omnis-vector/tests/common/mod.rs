//! Shared builders: the real packs, a world with the fixed party, and a frame driver.

#![allow(dead_code)]

pub mod app;

use omnis_sim::omnis_core::{Facing, MapId, Position};
use omnis_sim::omnis_data::{Data, load_packs};
use omnis_sim::{Command, Event, PartyCommand, Settings, World};
use omnis_vector::bind::{Binder, Outcome};
use omnis_vector::party;
use omnis_vector::pose::Pose;
use std::path::PathBuf;

pub const SEED: u64 = 7;

pub fn data() -> Data {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let (base, test) = (root.join("base"), root.join("test"));
    load_packs(&[base.as_path(), test.as_path()]).expect("the shipped packs load")
}

pub fn map(data: &Data, name: &str) -> MapId {
    *data
        .maps
        .iter()
        .find(|(_, m)| m.def.id == name)
        .map(|(id, _)| id)
        .expect("a test map")
}

/// A fresh world at the entry map with the fixed party, built through the binder so the
/// party commands are in its log.
pub fn session(data: &Data) -> (World, Binder) {
    let mut world = World::new(data, SEED, Settings::default()).expect("an entry map");
    let mut binder = Binder::default();
    for draft in party::fixed() {
        binder
            .apply(
                &mut world,
                data,
                Command::Party(PartyCommand::Create(draft)),
            )
            .expect("a valid draft");
    }
    (world, binder)
}

/// Put the party somewhere directly (tests that do not replay).
pub fn place(world: &mut World, map: MapId, x: u16, y: u16, facing: Facing) -> Pose {
    world.position = Position { map, x, y, facing };
    Pose::at(world.position)
}

/// Drive `frames` frames, moving the ground point by `(dx, dz)` each and holding `yaw`.
/// Returns the final pose and every outcome's events.
pub fn drive(
    binder: &mut Binder,
    world: &mut World,
    data: &Data,
    mut pose: Pose,
    (dx, dz): (f32, f32),
    yaw: f32,
    frames: u32,
) -> (Pose, Vec<Event>, Vec<Outcome>) {
    let mut events = Vec::new();
    let mut outcomes = Vec::new();
    for _ in 0..frames {
        let to = Pose {
            x: pose.x + dx,
            z: pose.z + dz,
            yaw,
        };
        let out = binder.advance(world, data, pose, to);
        pose = out.pose.expect("advance always reports a pose");
        events.extend(out.events.iter().cloned());
        outcomes.push(out);
    }
    (pose, events, outcomes)
}

pub fn steps(log: &[Command]) -> usize {
    log.iter().filter(|c| matches!(c, Command::Step(_))).count()
}
