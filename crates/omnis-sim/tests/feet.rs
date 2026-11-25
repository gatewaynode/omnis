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

// ---- Step 3: the monsters move, reach and shoot in feet ----

use omnis_data::Disposition;
use omnis_sim::omnis_rules::RollMode;
use omnis_sim::{ActorRef, CombatCommand, Event, Rejection, Surprise, combat};

fn monster_mut<'a>(data: &'a mut Data, name: &str) -> &'a mut omnis_data::Monster {
    let id = data
        .registry
        .monsters
        .get(&format!("base:monster:{name}"))
        .unwrap();
    data.monsters.get_mut(&id).unwrap()
}

/// A fight of `members` against base monsters, each stack `(name, count, feet)`, nobody
/// surprised, played by ending every member's turn for `rounds` rounds: the world, and each
/// round's events.
fn play(
    data: &Data,
    members: usize,
    stacks: &[(&str, u8, u16)],
    rounds: usize,
) -> (World, Vec<Vec<Event>>) {
    let mut world = common::world(data);
    party_of(&mut world, data, members);
    for member in &mut world.party.members {
        member.hp = 500;
    }
    let named: Vec<(&str, u8)> = stacks.iter().map(|s| (s.0, s.1)).collect();
    let mut encounter = common::encounter(data, &named, Disposition::Hostile, world.position);
    for (stack, spec) in encounter.stacks.iter_mut().zip(stacks) {
        stack.distance = spec.2;
    }
    let mut events = Vec::new();
    combat::start(&mut world, data, encounter, Surprise::None, &mut events).unwrap();
    let round_of = |world: &World| match &world.mode {
        Mode::Combat(state) => Some(state.round),
        _ => None,
    };
    for _ in 0..500 {
        match round_of(&world) {
            Some(round) if usize::try_from(round).unwrap() <= rounds => {}
            _ => break,
        }
        let end = Command::Combat(CombatCommand::EndTurn);
        events.extend(apply(&mut world, data, end).expect("a member's turn"));
    }
    let mut by_round: Vec<Vec<Event>> = Vec::new();
    for event in events {
        if matches!(event, Event::RoundStarted { .. }) {
            by_round.push(Vec::new());
        }
        if let Some(round) = by_round.last_mut() {
            round.push(event);
        }
    }
    by_round.truncate(rounds);
    (world, by_round)
}

fn moves(events: &[Event], of: u8) -> Vec<(u16, u16)> {
    dashes(events, of).into_iter().map(|(m, _)| m).collect()
}

/// Each move of a stack, and whether it was its Dash.
fn dashes(events: &[Event], of: u8) -> Vec<((u16, u16), bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::StackMoved {
                stack,
                from,
                to,
                dash,
            } if *stack == of => Some(((*from, *to), *dash)),
            _ => None,
        })
        .collect()
}

/// The roll mode of each attack a stack's individuals made.
fn attacks(events: &[Event], of: u8) -> Vec<RollMode> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::AttackResolved {
                attacker: ActorRef::Monster { stack, .. },
                roll,
                ..
            } if *stack == of => Some(roll.mode),
            _ => None,
        })
        .collect()
}

/// A stack with only a melee attack closes by its speed on its turn, dashes when that does not
/// bring it into reach, and does not attack before it arrives; in reach after its move, it
/// attacks the same turn.
#[test]
fn a_melee_stack_closes_and_cannot_hit_before_it_arrives() {
    let data = data();
    let (world, rounds) = play(&data, 6, &[("giant_rat", 2, 100)], 3);
    assert_eq!(moves(&rounds[0], 0), [(100, 70), (70, 40)], "move and dash");
    // The second move is told as the Dash it is; the first is not.
    assert_eq!(
        dashes(&rounds[0], 0),
        [((100, 70), false), ((70, 40), true)]
    );
    assert_eq!(moves(&rounds[1], 0), [(40, 10), (10, 5)]);
    assert!(attacks(&rounds[0], 0).is_empty() && attacks(&rounds[1], 0).is_empty());
    assert!(moves(&rounds[2], 0).is_empty(), "it has arrived");
    assert_eq!(attacks(&rounds[2], 0).len(), 2, "and both bite");
    assert_eq!(distances(&world), [5]);
    // Thirty feet off, its speed brings it into reach and it attacks at once.
    let (_, rounds) = play(&data, 6, &[("giant_rat", 2, 30)], 1);
    assert_eq!(moves(&rounds[0], 0), [(30, 5)]);
    assert_eq!(attacks(&rounds[0], 0).len(), 2);
}

/// A stack that can shoot closes and shoots instead of dashing: at no penalty within its normal
/// range, at disadvantage past it, and not at all past its long range, where it dashes.
#[test]
fn a_stack_that_can_shoot_shoots_on_the_way() {
    let data = data();
    let (_, rounds) = play(&data, 6, &[("goblin", 2, 100)], 1);
    assert_eq!(moves(&rounds[0], 0), [(100, 70)]);
    assert_eq!(attacks(&rounds[0], 0), [RollMode::Normal; 2]);
    let (_, rounds) = play(&data, 6, &[("goblin", 2, 200)], 1);
    assert_eq!(moves(&rounds[0], 0), [(200, 170)]);
    assert_eq!(attacks(&rounds[0], 0), [RollMode::Disadvantage; 2]);
    let (_, rounds) = play(&data, 6, &[("goblin", 2, 400)], 1);
    assert_eq!(moves(&rounds[0], 0), [(400, 370), (370, 340)]);
    assert!(attacks(&rounds[0], 0).is_empty(), "past its long range");
}

/// A stack with nothing but a ranged attack holds where it stands, and shoots at disadvantage
/// with the party upon it.
#[test]
fn a_stack_of_archers_holds_and_is_hindered_when_engaged() {
    let mut data = data();
    monster_mut(&mut data, "goblin")
        .attacks
        .retain(|a| a.ranged);
    let (world, rounds) = play(&data, 6, &[("goblin", 2, 60)], 2);
    assert!(moves(&rounds[0], 0).is_empty() && moves(&rounds[1], 0).is_empty());
    assert_eq!(attacks(&rounds[1], 0), [RollMode::Normal; 2]);
    assert_eq!(distances(&world), [60]);
    let (_, rounds) = play(&data, 6, &[("goblin", 2, 5)], 1);
    assert_eq!(attacks(&rounds[0], 0), [RollMode::Disadvantage; 2]);
}

/// A reach of ten feet stops ten feet off and strikes from there.
#[test]
fn a_long_reach_stops_at_its_reach() {
    let mut data = data();
    monster_mut(&mut data, "giant_rat").attacks[0].reach = 10;
    let (world, rounds) = play(&data, 6, &[("giant_rat", 1, 30)], 1);
    assert_eq!(moves(&rounds[0], 0), [(30, 10)]);
    assert_eq!(attacks(&rounds[0], 0).len(), 1);
    assert_eq!(distances(&world), [10]);
}

/// The front row presents one five-foot square a member; an engaged stack attacks in melee with
/// as many as fit those squares, the rest waiting when they cannot shoot.
#[test]
fn the_front_rows_squares_limit_who_attacks_in_melee() {
    let mut data = data();
    monster_mut(&mut data, "skeleton")
        .attacks
        .retain(|a| !a.ranged);
    // Six skeletons, one a square, against a front row of three.
    let (_, rounds) = play(&data, 6, &[("skeleton", 6, 5)], 1);
    assert_eq!(attacks(&rounds[0], 0).len(), 3);
    // Against one member alone there is one square: one skeleton, or the base rat's three.
    let (_, rounds) = play(&data, 1, &[("skeleton", 6, 5)], 1);
    assert_eq!(attacks(&rounds[0], 0).len(), 1);
    let (_, rounds) = play(&data, 1, &[("giant_rat", 7, 5)], 1);
    assert_eq!(attacks(&rounds[0], 0).len(), 3, "three rats a square");
    // A reach of ten feet lets a second rank strike past the first.
    monster_mut(&mut data, "skeleton").attacks[0].reach = 10;
    let (_, rounds) = play(&data, 1, &[("skeleton", 6, 5)], 1);
    assert_eq!(attacks(&rounds[0], 0).len(), 2);
    monster_mut(&mut data, "skeleton").attacks[0].reach = 5;
    // The squares are filled in the stacks' order: four skeletons take all three, and the
    // rats behind them find none.
    let (_, rounds) = play(&data, 6, &[("skeleton", 4, 5), ("giant_rat", 3, 5)], 1);
    assert_eq!(attacks(&rounds[0], 0).len(), 3);
    assert!(attacks(&rounds[0], 1).is_empty());
    // Two skeletons leave a square, which holds three rats.
    let (_, rounds) = play(&data, 6, &[("skeleton", 2, 5), ("giant_rat", 5, 5)], 1);
    assert_eq!(attacks(&rounds[0], 0).len(), 2);
    assert_eq!(attacks(&rounds[0], 1).len(), 3);
}

/// Those of an engaged stack who find no square shoot, when the stack can.
#[test]
fn those_without_a_square_shoot_when_they_can() {
    let data = data();
    let (_, rounds) = play(&data, 1, &[("skeleton", 3, 5)], 1);
    let modes = attacks(&rounds[0], 0);
    assert_eq!(modes.len(), 3, "one blade and two bows");
    assert_eq!(
        modes
            .iter()
            .filter(|m| **m == RollMode::Disadvantage)
            .count(),
        2,
        "the bows are hindered, engaged as they are"
    );
}

/// "In front" is engaged: a stack five feet off, however many there are. A member's melee
/// reaches those and no other.
#[test]
fn in_front_is_engaged() {
    let data = data();
    let stacks = [
        ("giant_rat", 1, 5),
        ("skeleton", 1, 5),
        ("goblin", 1, 5),
        ("goblin", 1, 40),
    ];
    let mut data_still = data.clone();
    // Nothing moves, so the picture holds: the far goblins only shoot.
    for name in ["giant_rat", "skeleton", "goblin"] {
        monster_mut(&mut data_still, name).speed = 0;
    }
    let (world, _) = play(&data_still, 6, &stacks, 0);
    let view = combat_view(&world, &data_still).expect("a fight");
    let front: Vec<bool> = view.stacks.iter().map(|s| s.in_front).collect();
    assert_eq!(front, [true, true, true, false]);
    let Mode::Combat(state) = &world.mode else {
        panic!("no fight")
    };
    assert_eq!(state.engaged_stacks(), [0, 1, 2]);
    // The acting member is of the front row or not; a sword reaches the engaged or nothing.
    let attack = |stack| Command::Combat(CombatCommand::Attack { stack });
    let mut copy = world.clone();
    let far = apply(&mut copy, &data_still, attack(3));
    if let Err(refusal) = far {
        assert!(
            matches!(
                refusal,
                Rejection::OutOfReach { stack: 3 } | Rejection::NeedsRangedWeapon
            ),
            "{refusal:?}"
        );
        assert_eq!(copy, world);
    }
}
