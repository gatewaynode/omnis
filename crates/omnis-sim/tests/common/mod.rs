//! Shared setup for the simulation's integration tests: real pack data, no mocks.
#![allow(dead_code)]

use omnis_core::{Direction, Pcg32, Position, Rotation, StreamName};
use omnis_data::{Alignment, Data, Disposition, Skill, load_packs};
use omnis_sim::omnis_rules::{Draft, monster_hit_points};
use omnis_sim::{
    CombatCommand, Command, EncounterChoice, EncounterSource, EncounterState, Event, Mode,
    PartyCommand, Settings, Stack, World, apply, combat_view,
};
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

/// Everything but the per-step chatter: the trailing `Visible` and the random encounter check
/// a map with a table rolls on every step.
pub fn without_visible(events: Vec<Event>) -> Vec<Event> {
    events
        .into_iter()
        .filter(|e| !matches!(e, Event::Visible { .. } | Event::EncounterCheck { .. }))
        .collect()
}

pub fn draft(name: &str, race: &str, class: &str, scores: [u8; 6], skills: &[Skill]) -> Draft {
    Draft {
        name: name.to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::LawfulGood,
        scores,
        skills: skills.to_vec(),
    }
}

/// Six drafts: two fighters, two clerics, a wizard, a rogue, in marching order.
pub fn six() -> Vec<Draft> {
    let two = [Skill::Athletics, Skill::Perception];
    vec![
        draft("Brenna", "human", "fighter", [15, 14, 13, 12, 10, 8], &two),
        draft(
            "Durin",
            "dwarf",
            "cleric",
            [10, 8, 14, 10, 15, 8],
            &[Skill::Medicine, Skill::History],
        ),
        draft(
            "Ilvara",
            "elf",
            "wizard",
            [8, 14, 13, 15, 12, 10],
            &[Skill::Arcana, Skill::History],
        ),
        draft(
            "Pip",
            "halfling",
            "rogue",
            [8, 15, 12, 10, 13, 14],
            &[
                Skill::Stealth,
                Skill::Acrobatics,
                Skill::Deception,
                Skill::Perception,
            ],
        ),
        draft("Gorm", "dwarf", "fighter", [15, 10, 14, 8, 13, 8], &two),
        draft(
            "Wren",
            "human",
            "cleric",
            [10, 10, 12, 8, 15, 13],
            &[Skill::Medicine, Skill::Persuasion],
        ),
    ]
}

/// The first `count` of the six drafts, created through commands.
pub fn party_of(world: &mut World, data: &Data, count: usize) {
    for draft in six().into_iter().take(count) {
        apply(world, data, Command::Party(PartyCommand::Create(draft))).expect("a valid draft");
    }
}

/// A hand-built encounter of base monsters by short name, each individual at the rules'
/// hit points, retreating to `retreat`.
pub fn encounter(
    data: &Data,
    stacks: &[(&str, u8)],
    disposition: Disposition,
    retreat: Position,
) -> EncounterState {
    let stream = StreamName::new("combat");
    let mut rng = Pcg32::for_stream(0, &stream);
    let stacks = stacks
        .iter()
        .map(|(name, count)| {
            let id = data
                .registry
                .monsters
                .get(&format!("base:monster:{name}"))
                .unwrap_or_else(|| panic!("{name}"));
            let hp = monster_hit_points(&data.monsters[&id], data, &mut rng, &stream).unwrap();
            Stack {
                monster: id,
                initial: *count,
                hp: vec![hp; usize::from(*count)],
            }
        })
        .collect();
    EncounterState {
        source: EncounterSource::Random,
        stacks,
        disposition,
        retreat,
    }
}

/// From the meadow start to the goblins at (3, 8) of the dungeon: up the road through the
/// portal, down the first room, left to the door, through it, and three tiles in.
pub fn walk_to_goblins() -> Vec<Command> {
    let mut commands = vec![Command::Step(Direction::Forward); 16];
    commands.extend([
        Command::Turn(Rotation::Left),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Turn(Rotation::Right),
        Command::Interact,
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
    ]);
    commands
}

/// The first living stack the acting member can reach.
pub fn reachable_stack(world: &World, data: &Data) -> Option<u8> {
    combat_view(world, data)?
        .stacks
        .iter()
        .find(|s| s.alive && s.reachable)
        .map(|s| s.index)
}

/// Fight whatever stands in the way until the party explores again: attack on an encounter,
/// then the nearest reachable stack on every member turn. Every command taken is appended to
/// `commands`; the events come back.
pub fn settle(world: &mut World, data: &Data, commands: &mut Vec<Command>) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..1000 {
        let command = match &world.mode {
            Mode::Explore => return events,
            Mode::Encounter(_) => Command::Encounter(EncounterChoice::Attack),
            Mode::Combat(_) => {
                let stack = reachable_stack(world, data).expect("something to hit");
                Command::Combat(CombatCommand::Attack { stack })
            }
        };
        events.extend(apply(world, data, command.clone()).unwrap_or_else(|r| panic!("{r}")));
        commands.push(command);
    }
    panic!("the fight did not end");
}

/// Apply commands in order, settling any fight on the way; returns every command taken and
/// every event.
pub fn play(world: &mut World, data: &Data, script: &[Command]) -> (Vec<Command>, Vec<Event>) {
    let mut commands = Vec::new();
    let mut events = Vec::new();
    for command in script {
        events.extend(apply(world, data, command.clone()).unwrap_or_else(|r| panic!("{r}")));
        commands.push(command.clone());
        events.extend(settle(world, data, &mut commands));
    }
    (commands, events)
}
