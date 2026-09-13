//! Shared setup for the simulation's integration tests: real pack data, no mocks.
#![allow(dead_code)]

use omnis_core::{Direction, Rotation};
use omnis_data::{Data, load_packs};
use omnis_sim::{Command, Event, Settings, World, apply};
use std::path::PathBuf;

pub fn test_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/test")
}

pub fn base_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/base")
}

/// The base content and the test maps, in the order every default pack list uses.
pub fn data() -> Data {
    load_packs(&[&base_pack(), &test_pack()]).unwrap_or_else(|r| panic!("{r}"))
}

pub fn world(data: &Data) -> World {
    World::new(data, 0x0123_4567_89ab_cdef, Settings::default()).expect("entry map")
}

pub fn step(world: &mut World, data: &Data) -> Vec<Event> {
    apply(world, data, Command::Step(Direction::Forward)).expect("explore accepts steps")
}

pub fn turn(world: &mut World, data: &Data, rotation: Rotation) -> Vec<Event> {
    apply(world, data, Command::Turn(rotation)).expect("explore accepts turns")
}

pub fn interact(world: &mut World, data: &Data) -> Vec<Event> {
    apply(world, data, Command::Interact).expect("explore accepts interact")
}

/// Everything but the trailing `Visible`.
pub fn without_visible(events: Vec<Event>) -> Vec<Event> {
    events
        .into_iter()
        .filter(|e| !matches!(e, Event::Visible { .. }))
        .collect()
}
