//! The opening strike in feet (owner, 2026-11-14; PRD §8.2; `tasks/plans/combat-in-feet.md`
//! step 6): the party rolls Stealth as a group for surprise, the strike of an unseen party has
//! advantage, and a weapon may open a fight from the road as a spell does.

use crate::objects::{objects, yard};
use crate::opening_strike::at;
use omnis_core::Facing;
use omnis_data::{DamageType, Data, EquipSlot};
use omnis_rules::RollMode;
use omnis_sim::{
    ActorRef, CheckKind, Chosen, CombatCommand, Command, DevCommand, Event, Mode, Rejection, Sight,
    Surprise, Target, World, apply, targets,
};

const FIGHTER: usize = 0;
const WIZARD: usize = 2;
const BOLT: &str = "base:spell:fire_bolt";

fn rat() -> Target {
    Target::Being {
        being: "rat".into(),
    }
}

/// The yard with the rats' Wisdom set, so their passive Perception is what a test needs, and
/// its tiles `feet` across when given.
fn yard_with(wisdom: u8, feet: Option<u32>) -> Data {
    let mut data = objects();
    let id = data.registry.monsters.get("objects:monster:pack_rat");
    data.monsters.get_mut(&id.unwrap()).unwrap().abilities[4] = wisdom;
    let yard = yard(&data);
    data.maps.get_mut(&yard).unwrap().def.tile_feet = feet;
    data
}

/// The same world on another seed: the dice differ, nothing else.
fn seeded(world: &World, seed: u64) -> World {
    let mut world = world.clone();
    world.seed = seed;
    world.rngs.clear();
    world
}

fn bolt(world: &World) -> Command {
    Command::Cast {
        caster: world.party.members[WIZARD].id,
        spell: BOLT.into(),
        target: rat(),
    }
}

fn attack(world: &World, slot: usize) -> Command {
    Command::Attack {
        member: world.party.members[slot].id,
        target: rat(),
    }
}

/// The party's Stealth checks among the events: `(passed, dc)` for each.
fn sneaks(events: &[Event]) -> Vec<(bool, i64)> {
    let checks = events.iter().filter_map(|e| match e {
        Event::Check {
            kind: CheckKind::Hide,
            roll: Some(roll),
            dc,
            success,
            ..
        } => {
            assert_eq!(*success, roll.total >= *dc, "{e:?}");
            Some((*success, *dc))
        }
        _ => None,
    });
    checks.collect()
}

fn surprise(events: &[Event]) -> Surprise {
    let started = events.iter().find_map(|e| match e {
        Event::CombatStarted { surprised } => Some(*surprised),
        _ => None,
    });
    started.expect("a fight")
}

/// The strike's own attack roll: the first by a member, before round one.
fn first_roll(events: &[Event], member: omnis_core::CharacterId) -> (RollMode, bool) {
    let round = events
        .iter()
        .position(|e| matches!(e, Event::RoundStarted { round: 1 }))
        .expect("round one");
    let roll = events[..round].iter().find_map(|e| match e {
        Event::AttackResolved {
            attacker: ActorRef::Member(id),
            roll,
            hit,
            ..
        } if *id == member => Some((roll.mode, *hit)),
        _ => None,
    });
    roll.expect("the strike rolled to hit before round one")
}

fn fight(world: &World) -> &omnis_sim::CombatState {
    match &world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

/// Every member who can act rolls Stealth against the group's best passive Perception; half
/// or more passing, the monsters are surprised, and otherwise the fight starts even. Both
/// happen, by the dice alone.
#[test]
fn the_party_rolls_stealth_as_a_group_for_surprise() {
    // Wisdom 18: a passive Perception of 14, which the six pass about as often as not.
    let data = yard_with(18, None);
    let base = at(&data, 3, 1, Facing::East);
    let mut seen = [0, 0];
    for seed in 0..40 {
        let mut world = seeded(&base, seed);
        let command = bolt(&world);
        let events = apply(&mut world, &data, command).unwrap();
        let rolls = sneaks(&events);
        assert_eq!(rolls.len(), 6, "everyone who can act rolls");
        assert!(rolls.iter().all(|(_, dc)| *dc == 14), "{rolls:?}");
        let passed = rolls.iter().filter(|(ok, _)| *ok).count();
        let caught = passed * 2 >= rolls.len();
        let want = if caught {
            Surprise::Monsters
        } else {
            Surprise::None
        };
        assert_eq!(surprise(&events), want, "seed {seed}: {passed} of 6");
        assert_eq!(fight(&world).surprised, want);
        // The rolls come after the meeting and before the fight is announced.
        let at_of = |want: fn(&Event) -> bool| events.iter().position(want).unwrap();
        let met = at_of(|e| matches!(e, Event::EncounterStarted { .. }));
        let check = at_of(|e| matches!(e, Event::Check { .. }));
        let started = at_of(|e| matches!(e, Event::CombatStarted { .. }));
        assert!(met < check && check < started, "seed {seed}");
        seen[usize::from(caught)] += 1;
    }
    assert!(seen[0] > 3 && seen[1] > 3, "both ways: {seen:?}");
}

/// A member who is down does not roll; three of five is still half or more.
#[test]
fn a_member_who_is_down_does_not_roll() {
    let data = yard_with(1, None);
    let mut world = at(&data, 3, 1, Facing::East);
    let down = DevCommand::SetHp {
        member: world.party.members[5].id,
        hp: 0,
    };
    apply(&mut world, &data, Command::Dev(down)).unwrap();
    let command = bolt(&world);
    let events = apply(&mut world, &data, command).unwrap();
    let rolls = sneaks(&events);
    assert_eq!(rolls.len(), 5);
    assert!(rolls.iter().all(|(_, dc)| *dc == 5), "Wisdom 1: {rolls:?}");
}

/// Unseen, the strike's attack has advantage; seen, it has none. Either way the striker is not
/// left hidden for a later attack.
#[test]
fn the_strike_of_an_unseen_party_has_advantage() {
    let data = yard_with(18, None);
    let base = at(&data, 3, 1, Facing::East);
    let wizard = base.party.members[WIZARD].id;
    let mut seen = [0, 0];
    for seed in 0..40 {
        let mut world = seeded(&base, seed);
        let command = bolt(&world);
        let events = apply(&mut world, &data, command).unwrap();
        let caught = surprise(&events) == Surprise::Monsters;
        let want = if caught {
            RollMode::Advantage
        } else {
            RollMode::Normal
        };
        assert_eq!(first_roll(&events, wizard).0, want, "seed {seed}");
        if let Mode::Combat(state) = &world.mode {
            assert!(state.hidden.is_empty(), "seed {seed}");
        }
        seen[usize::from(caught)] += 1;
    }
    assert!(seen[0] > 0 && seen[1] > 0, "{seen:?}");
    // A spell that rolls no attack leaves no one hidden either.
    for seed in 0..10 {
        let mut world = seeded(&base, seed);
        let missile = Command::Cast {
            caster: wizard,
            spell: "base:spell:magic_missile".into(),
            target: rat(),
        };
        apply(&mut world, &data, missile).unwrap();
        if let Mode::Combat(state) = &world.mode {
            assert!(state.hidden.is_empty(), "seed {seed}");
        }
    }
}

/// With the rules value written 0 nobody rolls and nobody is surprised.
#[test]
fn a_pack_that_turns_surprise_off_rolls_nothing() {
    let mut data = yard_with(1, None);
    data.rules.insert_value("opening_strike_surprise", 0);
    let mut world = at(&data, 3, 1, Facing::East);
    let command = bolt(&world);
    let events = apply(&mut world, &data, command).unwrap();
    assert!(sneaks(&events).is_empty());
    assert_eq!(surprise(&events), Surprise::None);
    let wizard = world.party.members[WIZARD].id;
    assert_eq!(first_roll(&events, wizard).0, RollMode::Normal);
}

/// A weapon opens a fight from the road as a spell does: the group is met, the blow lands
/// before round one at the distance it crossed, it was the member's action, and it cost
/// neither points nor time.
#[test]
fn a_bow_opens_a_fight_from_the_road() {
    let mut data = yard_with(10, None);
    data.rules.insert_value("opening_strike_surprise", 0);
    let base = at(&data, 3, 1, Facing::East);
    let fighter = base.party.members[FIGHTER].id;
    let mut hits = 0;
    for seed in 0..20 {
        let mut world = seeded(&base, seed);
        let command = attack(&world, FIGHTER);
        let events = apply(&mut world, &data, command).unwrap();
        assert!(matches!(events[0], Event::EncounterStarted { .. }));
        let (mode, hit) = first_roll(&events, fighter);
        assert_eq!(mode, RollMode::Normal, "within the crossbow's normal range");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::TimeAdvanced { .. }))
        );
        let kit = |w: &World| -> Vec<_> {
            let members = w.party.members.iter();
            members
                .map(|m| (m.spell_points, m.equipment.clone()))
                .collect()
        };
        assert_eq!(kit(&world), kit(&base), "nothing paid");
        if hit {
            hits += 1;
            let round = events
                .iter()
                .position(|e| matches!(e, Event::RoundStarted { round: 1 }))
                .unwrap();
            let kind = events[..round].iter().find_map(|e| match e {
                Event::Damage { kind, .. } => Some(*kind),
                _ => None,
            });
            assert_eq!(kind, Some(DamageType::Piercing), "the crossbow's bolt");
        }
        let Mode::Combat(state) = &world.mode else {
            continue;
        };
        // Two tiles of 5 ft: the rats start 10 ft off, wherever they have got to since.
        let from = events.iter().find_map(|e| match e {
            Event::StackMoved { stack: 0, from, .. } => Some(*from),
            _ => None,
        });
        let start = from.unwrap_or(state.encounter.stacks[0].distance);
        assert_eq!(start, 10, "seed {seed}");
        // The strike was the fighter's action for round one.
        if state.current_actor() == Some(ActorRef::Member(fighter)) {
            assert_eq!(state.budget.actions, 0);
            let mut copy = world.clone();
            let dodge = Command::Combat(CombatCommand::Dodge);
            assert_eq!(apply(&mut copy, &data, dodge), Err(Rejection::NoActionLeft));
        } else if state.round == 1 {
            assert_eq!(state.struck, Some(fighter));
        }
    }
    assert!(hits > 0, "some bolt hit");
}

/// Past the weapon's normal range the shot has disadvantage, which an unseen party's advantage
/// cancels; past its long range it is refused.
#[test]
fn a_shot_is_hindered_past_normal_range_and_refused_past_long() {
    // Tiles of 30 ft: from (2, 1) the rats are 90 ft off, past the light crossbow's 80.
    let mut data = yard_with(1, Some(30));
    let base = at(&data, 2, 1, Facing::East);
    let fighter = base.party.members[FIGHTER].id;
    let mut both = [0, 0];
    for seed in 0..20 {
        let mut world = seeded(&base, seed);
        let command = attack(&world, FIGHTER);
        let events = apply(&mut world, &data, command).unwrap();
        let caught = surprise(&events) == Surprise::Monsters;
        let want = if caught {
            RollMode::Normal
        } else {
            RollMode::Disadvantage
        };
        assert_eq!(first_roll(&events, fighter).0, want, "seed {seed}");
        both[usize::from(caught)] += 1;
    }
    assert!(both[1] > 0, "Wisdom 1 is passed by most: {both:?}");
    data.rules.insert_value("opening_strike_surprise", 0);
    let mut world = at(&data, 2, 1, Facing::East);
    let command = attack(&world, FIGHTER);
    let events = apply(&mut world, &data, command).unwrap();
    assert_eq!(first_roll(&events, fighter).0, RollMode::Disadvantage);
    // At 60 ft, two tiles, it is within normal range.
    let mut world = at(&data, 3, 1, Facing::East);
    let command = attack(&world, FIGHTER);
    let events = apply(&mut world, &data, command).unwrap();
    assert_eq!(first_roll(&events, fighter).0, RollMode::Normal);
    // Tiles of 200 ft: two tiles are past the crossbow's 320.
    let data = yard_with(1, Some(200));
    let world = at(&data, 3, 1, Facing::East);
    assert_eq!(
        refused(&world, &data, FIGHTER, rat()),
        Rejection::BeyondRange
    );
    let rows = listed(&world, &data, FIGHTER);
    assert_eq!(rows.len(), 1, "in sight, so listed");
    assert_eq!(rows[0].2, Some(Rejection::BeyondRange));
}

/// Beside the group a front-row member strikes with the weapon in hand; the back row stands a
/// square further off and needs a ranged weapon, as does anyone at a distance.
#[test]
fn a_melee_weapon_strikes_the_group_beside_the_party() {
    let mut data = yard_with(10, None);
    data.rules.insert_value("opening_strike_surprise", 0);
    let base = at(&data, 4, 1, Facing::East);
    let fighter = base.party.members[FIGHTER].id;
    let mut kinds = Vec::new();
    for seed in 0..20 {
        let mut world = seeded(&base, seed);
        let command = attack(&world, FIGHTER);
        let events = apply(&mut world, &data, command).unwrap();
        if first_roll(&events, fighter).1 {
            kinds.extend(events.iter().find_map(|e| match e {
                Event::Damage { kind, .. } => Some(*kind),
                _ => None,
            }));
        }
    }
    assert!(!kinds.is_empty(), "some blow hit");
    assert!(
        kinds.iter().all(|k| *k == DamageType::Slashing),
        "the longsword, not the crossbow: {kinds:?}"
    );
    // Without a ranged weapon: beside them in front, fine; in the back row, or two tiles off,
    // nothing reaches.
    let unslung = |world: &World| {
        let mut world = world.clone();
        for member in &mut world.party.members {
            member.equipped.remove(&EquipSlot::Ranged);
        }
        world
    };
    let near = unslung(&base);
    assert_eq!(listed(&near, &data, FIGHTER)[0].2, None);
    let back = omnis_sim::party::front_row(&data);
    assert_eq!(
        refused(&near, &data, back, rat()),
        Rejection::NeedsRangedWeapon,
        "the back row"
    );
    let far = unslung(&at(&data, 3, 1, Facing::East));
    assert_eq!(
        refused(&far, &data, FIGHTER, rat()),
        Rejection::NeedsRangedWeapon
    );
}

fn refused(world: &World, data: &Data, slot: usize, target: Target) -> Rejection {
    let mut copy = world.clone();
    let command = Command::Attack {
        member: world.party.members[slot].id,
        target,
    };
    let result = apply(&mut copy, data, command);
    assert_eq!(&copy, world, "a refusal changes nothing");
    result.expect_err("refused")
}

/// The groups in the attack's list for the member in `slot` (the things a weapon may damage
/// are `breaking.rs`'s), each row of the list checked against the command itself on
/// a copy of the world: `(target, tiles, refusal)`.
fn listed(world: &World, data: &Data, slot: usize) -> Vec<(Target, u16, Option<Rejection>)> {
    let id = world.party.members[slot].id;
    let rows = targets(world, data, id, &Chosen::Attack).expect("a list");
    for row in &rows {
        assert_eq!(row.sight, Sight::Clear);
        let mut copy = world.clone();
        let command = Command::Attack {
            member: id,
            target: row.target.clone(),
        };
        let result = apply(&mut copy, data, command);
        assert_eq!(result.as_ref().err(), row.refusal.as_ref(), "{row:?}");
        if row.refusal.is_some() {
            assert_eq!(&copy, world, "a refusal changes nothing: {row:?}");
        }
    }
    let rows = rows.into_iter();
    rows.filter(|r| matches!(r.target, Target::Being { .. }))
        .map(|r| (r.target, r.distance, r.refusal))
        .collect()
}

/// On the road the attack lists the groups in sight, and the command agrees with every row;
/// what is not a group in sight is refused.
#[test]
fn the_attack_lists_the_groups_in_sight() {
    let data = yard_with(10, None);
    let world = at(&data, 3, 1, Facing::West);
    assert_eq!(listed(&world, &data, FIGHTER), [(rat(), 2, None)]);
    // Behind the hedge the rats are not seen: no row, and no group for the command.
    let hidden = at(&data, 5, 4, Facing::North);
    assert!(listed(&hidden, &data, FIGHTER).is_empty());
    let gone = |being: &str| Rejection::NoSuchBeing {
        being: being.into(),
    };
    assert_eq!(refused(&hidden, &data, FIGHTER, rat()), gone("rat"));
    let ghost = Target::Being {
        being: "ghost".into(),
    };
    assert_eq!(refused(&world, &data, FIGHTER, ghost), gone("ghost"));
    let own = Target::Member(world.party.members[FIGHTER].id);
    for target in [Target::Stack(0), own] {
        assert_eq!(
            refused(&world, &data, FIGHTER, target),
            Rejection::WrongTarget
        );
    }
    // The dead and the down strike no one.
    let mut hurt = world.clone();
    let id = hurt.party.members[FIGHTER].id;
    let down = DevCommand::SetHp { member: id, hp: 0 };
    apply(&mut hurt, &data, Command::Dev(down)).unwrap();
    assert!(matches!(
        refused(&hurt, &data, FIGHTER, rat()),
        Rejection::MemberDown { .. } | Rejection::MemberDead { .. }
    ));
    // In a fight the road's attack is not a command.
    let mut world = world;
    let command = attack(&world, FIGHTER);
    apply(&mut world, &data, command).unwrap();
    if matches!(world.mode, Mode::Combat(_)) {
        assert_eq!(refused(&world, &data, FIGHTER, rat()), Rejection::WrongMode);
    }
}

/// The long hall east of the yard (owner, 2026-11-14: "one long hall with a new monster mob";
/// inside the yard since 2026-11-15, behind the gate at (6, 2)): from its west end the
/// shamblers are in sight a hundred and forty feet off; a crossbow opens the fight at that
/// distance, the line may advance, and the mob closes fifteen feet a turn, dashing while it is
/// out of its stones' range. From the yard, with the gate shut, they are not seen.
#[test]
fn the_long_hall_is_a_fight_at_a_distance() {
    let mut data = objects();
    data.rules.insert_value("opening_strike_surprise", 0);
    let settings = omnis_sim::Settings {
        devtools: true,
        ..omnis_sim::Settings::default()
    };
    let mut world = crate::common::new_world(&data, 3, settings);
    crate::common::party_of(&mut world, &data, 6);
    let hall = DevCommand::Teleport {
        map: "objects:map:yard".into(),
        level: omnis_core::Level::GROUND,
        x: 7,
        y: 2,
        facing: Facing::East,
    };
    // In the yard, before the shut gate: only the rats are there to attack.
    let yard = at(&data, 6, 2, Facing::East);
    let seen = targets(
        &yard,
        &data,
        yard.party.members[FIGHTER].id,
        &Chosen::Attack,
    )
    .unwrap();
    let groups = seen
        .iter()
        .filter(|row| matches!(row.target, Target::Being { .. }));
    assert_eq!(groups.map(|row| &row.target).collect::<Vec<_>>(), [&rat()]);
    apply(&mut world, &data, Command::Dev(hall)).expect("the hall's west end");
    let mob = Target::Being {
        being: "shamblers".into(),
    };
    let fighter = world.party.members[FIGHTER].id;
    let rows = targets(&world, &data, fighter, &Chosen::Attack).unwrap();
    // The gate behind the party is a thing to strike too (step 9); the groups are these.
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|row| matches!(row.target, Target::Being { .. }))
        .collect();
    assert_eq!(rows.len(), 1, "seen from end to end");
    assert_eq!(
        (rows[0].target.clone(), rows[0].distance),
        (mob.clone(), 28)
    );
    assert_eq!(rows[0].refusal, None, "within the crossbow's long range");
    let strike = Command::Attack {
        member: fighter,
        target: mob,
    };
    let events = apply(&mut world, &data, strike).unwrap();
    assert_eq!(first_roll(&events, fighter).0, RollMode::Disadvantage);
    let from = events.iter().find_map(|e| match e {
        Event::StackMoved {
            stack: 0, from, to, ..
        } => Some((*from, *to)),
        _ => None,
    });
    let state = fight(&world);
    match from {
        // The mob acted already: out of range of its stones, it moved and dashed.
        Some(moved) => assert_eq!(moved, (140, 125)),
        None => assert_eq!(state.encounter.stacks[0].distance, 140),
    }
    assert_eq!(state.encounter.stacks[0].hp.len(), 4, "four of them");
    let view = omnis_sim::combat_view(&world, &data).unwrap();
    assert_eq!(view.advance, 25, "room to advance");
}
