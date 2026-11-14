//! Combat in feet (owner, 2026-11-14; `tasks/plans/combat-in-feet.md`): each stack of a fight
//! stands at a distance in feet from the party's front line, five being engaged.

use crate::common::{self, data, party_of};
use crate::objects::objects;
use crate::save_and_replay::save_path;
use omnis_core::{Facing, Level};
use omnis_data::Data;
use omnis_sim::{
    Command, DevCommand, EncounterState, Mode, RestCommand, Settings, Target, World, apply,
    combat_view,
};

const WIZARD: usize = 2;

/// The yard, its tiles `feet` across when given, the rats' group joined by `more` rats in a
/// second and a third stack.
fn yard_of(feet: Option<u32>, more: &[u8]) -> Data {
    let mut data = objects();
    let yard = crate::objects::yard(&data);
    let map = data.maps.get_mut(&yard).expect("yard");
    map.def.tile_feet = feet;
    let rat = map.encounters[0].stacks[0].0;
    for count in more {
        map.encounters[0].stacks.push((rat, *count));
    }
    data
}

fn at(data: &Data, x: u16, y: u16, facing: Facing) -> World {
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    let mut world = common::new_world(data, 3, settings);
    party_of(&mut world, data, 6);
    let teleport = DevCommand::Teleport {
        map: "objects:map:yard".into(),
        level: Level::GROUND,
        x,
        y,
        facing,
    };
    apply(&mut world, data, Command::Dev(teleport)).expect("a cell of the yard");
    world
}

fn encounter(world: &World) -> &EncounterState {
    match &world.mode {
        Mode::Encounter(state) => state,
        Mode::Combat(state) => &state.encounter,
        other => panic!("no meeting: {other:?}"),
    }
}

fn distances(world: &World) -> Vec<u16> {
    encounter(world).stacks.iter().map(|s| s.distance).collect()
}

fn approach(data: &Data) -> World {
    approach_from(data, 4)
}

fn approach_from(data: &Data, x: u16) -> World {
    // The rats stand at (5, 1); from (4, 1) facing east they are the faced cell.
    let mut world = at(data, x, 1, Facing::East);
    let being = "rat".into();
    apply(&mut world, data, Command::Approach { being }).expect("in reach");
    world
}

fn strike(data: &Data, x: u16) -> World {
    let mut world = at(data, x, 1, Facing::East);
    let cast = Command::Cast {
        caster: world.party.members[WIZARD].id,
        spell: "base:spell:fire_bolt".into(),
        target: Target::Being {
            being: "rat".into(),
        },
    };
    apply(&mut world, data, cast).expect("in sight and range");
    world
}

/// A group met stands as far off as the tiles between it and the party, in the map's feet; each
/// later stack stands behind the one before. On a map that states no scale a tile is five feet.
#[test]
fn a_group_met_stands_as_far_as_the_tiles_between() {
    assert_eq!(distances(&approach(&yard_of(None, &[]))), [5]);
    assert_eq!(distances(&approach(&yard_of(None, &[3, 1]))), [5, 15, 25]);
    assert_eq!(distances(&approach(&yard_of(Some(15), &[3]))), [15, 25]);
    assert_eq!(distances(&approach(&yard_of(Some(30), &[]))), [30]);
    // On the group's own cell there are no tiles between: engaged, never nearer.
    assert_eq!(distances(&approach_from(&yard_of(Some(30), &[]), 5)), [5]);
}

/// The opening strike starts its fight at the real distance: the tiles the spell crossed.
#[test]
fn a_strike_starts_the_fight_at_the_distance_it_was_cast_from() {
    assert_eq!(distances(&strike(&yard_of(None, &[2]), 3)), [10, 20]);
    assert_eq!(distances(&strike(&yard_of(None, &[]), 2)), [15]);
    assert_eq!(distances(&strike(&yard_of(Some(15), &[2]), 3)), [30, 40]);
}

/// A pack sets how far each later stack stands behind (`stack_spacing_feet`, ten unless written).
#[test]
fn a_pack_sets_the_spacing_of_the_stacks() {
    let mut data = yard_of(None, &[3, 1]);
    data.rules.insert_value("stack_spacing_feet", 5);
    assert_eq!(distances(&approach(&data)), [5, 10, 15]);
    data.rules.insert_value("stack_spacing_feet", 0);
    assert_eq!(distances(&approach(&data)), [5, 5, 5]);
}

/// A group walked onto was a tile away when the step began.
#[test]
fn a_group_walked_onto_was_a_tile_away() {
    let scaled = |feet: Option<u32>| {
        let mut data = data();
        let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
        data.maps.get_mut(&dungeon).unwrap().def.tile_feet = feet;
        let mut world = common::six_world(&data);
        // The walk ends at the first group stepped onto: the placed rats, or one the random
        // table sends on the way. Either was a tile away.
        for command in common::walk_to_the_rats() {
            if world.mode != Mode::Explore {
                break;
            }
            apply(&mut world, &data, command).expect("the way to the rats");
        }
        distances(&world)
    };
    assert_eq!(scaled(None), [5]);
    assert_eq!(scaled(Some(20)), [20]);
}

/// A group that comes on the party at rest is upon it, whatever the map's scale.
#[test]
fn an_ambush_at_rest_is_engaged_at_once() {
    let mut data = data();
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    data.maps.get_mut(&dungeon).unwrap().def.tile_feet = Some(30);
    let (_, world, _) = crate::rest::ambushed(&data, &RestCommand::Long, 5000);
    assert_eq!(distances(&world)[0], 5);
}

/// The fight's view shows each stack's distance, and a fight saved before distances were kept
/// loads with every stack engaged.
#[test]
fn the_view_shows_the_distance_and_an_old_fight_loads_engaged() {
    let data = yard_of(Some(15), &[2]);
    let world = strike(&data, 3);
    let shown: Vec<u16> = combat_view(&world, &data)
        .expect("a fight")
        .stacks
        .iter()
        .map(|s| s.distance)
        .collect();
    assert_eq!(shown, [30, 40]);

    let data = self::data();
    let path = save_path("v5");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("Combat(") && !text.contains("distance"));
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert!(!distances(&world).is_empty());
    assert!(distances(&world).iter().all(|feet| *feet == 5));
}
