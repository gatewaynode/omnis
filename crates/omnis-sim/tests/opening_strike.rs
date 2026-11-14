//! The opening strike (owner, 2026-11-14; PRD §8.2; `tasks/plans/targeting.md` step 7): on the
//! road an attack, auto-hit or save spell may be sent at a group standing in sight. The cast is
//! paid, the group becomes a fight's stacks, the spell lands on it as in a fight, and the fight
//! starts with the monsters surprised and the caster's action for the first round spent.

use crate::common::{self, party_of};
use crate::objects::{objects, yard};
use omnis_core::{Facing, Level};
use omnis_data::{Data, Disposition, Range, Targeting};
use omnis_sim::{
    ActorRef, Chosen, CombatCommand, Command, DevCommand, EncounterSource, Event, Mode, Pay,
    Rejection, Settings, Sight, Surprise, Target, TargetView, World, apply, targets,
};

/// Ilvara's slot in the six drafts.
const WIZARD: usize = 2;
const MISSILE: &str = "base:spell:magic_missile";
const BOLT: &str = "base:spell:fire_bolt";

fn rat() -> Target {
    Target::Being {
        being: "rat".into(),
    }
}

fn chosen(spell: &str) -> Chosen {
    Chosen::Spell {
        spell: spell.into(),
        pay: Pay::Action,
    }
}

/// The six in the yard at `(x, y)`; the two pack rats stand at (5, 1).
pub(crate) fn at(data: &Data, x: u16, y: u16, facing: Facing) -> World {
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

fn cast(world: &World, spell: &str, target: Target) -> Command {
    Command::Cast {
        caster: world.party.members[WIZARD].id,
        spell: spell.into(),
        target,
    }
}

/// The spell's list, each row checked against the command itself on a copy of the world.
fn listed(world: &World, data: &Data, spell: &str) -> Vec<TargetView> {
    let id = world.party.members[WIZARD].id;
    let rows = targets(world, data, id, &chosen(spell)).expect("a list");
    for row in &rows {
        let mut copy = world.clone();
        let result = apply(&mut copy, data, cast(world, spell, row.target.clone()));
        assert_eq!(result.as_ref().err(), row.refusal.as_ref(), "{row:?}");
        if row.refusal.is_some() {
            assert_eq!(&copy, world, "a refusal changes nothing: {row:?}");
        }
    }
    rows
}

fn refused(world: &World, data: &Data, spell: &str, target: Target) -> Rejection {
    let mut copy = world.clone();
    let result = apply(&mut copy, data, cast(world, spell, target));
    assert_eq!(&copy, world, "a refusal changes nothing");
    result.expect_err("refused")
}

fn fight(world: &World) -> &omnis_sim::CombatState {
    match &world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

/// A harmful spell on the road lists the groups standing in sight and within its range, whichever
/// way the party faces; out of sight there is no one, and no command either.
#[test]
fn a_harmful_spell_lists_the_groups_in_sight() {
    let data = objects();
    for facing in [Facing::East, Facing::West] {
        let world = at(&data, 3, 1, facing);
        for spell in [BOLT, MISSILE] {
            let rows = listed(&world, &data, spell);
            assert_eq!(rows.len(), 1, "{spell} {rows:?}");
            assert_eq!(
                (rows[0].target.clone(), rows[0].distance, rows[0].sight),
                (rat(), 2, Sight::Clear)
            );
            assert_eq!(rows[0].refusal, None);
        }
    }
    // Three tiles south of them, behind the hedge, the rats are not seen: no row, no group.
    // (From the far corner they are seen through the gap in the hedge.)
    let world = at(&data, 5, 4, Facing::North);
    assert!(listed(&world, &data, BOLT).is_empty());
    assert_eq!(listed(&at(&data, 0, 4, Facing::East), &data, BOLT).len(), 1);
    let no_such = |being: &str| Rejection::NoSuchBeing {
        being: being.into(),
    };
    assert_eq!(refused(&world, &data, BOLT, rat()), no_such("rat"));
    // A group that is not there, a stack and a member are no targets of it on the road.
    let world = at(&data, 3, 1, Facing::East);
    let ghost = Target::Being {
        being: "ghost".into(),
    };
    assert_eq!(refused(&world, &data, BOLT, ghost), no_such("ghost"));
    assert_eq!(
        refused(&world, &data, BOLT, Target::Stack(0)),
        Rejection::WrongTarget
    );
    let own = Target::Member(world.party.members[WIZARD].id);
    assert_eq!(refused(&world, &data, BOLT, own), Rejection::WrongTarget);
    // A group is no target of a heal or of a hand.
    for spell in ["base:spell:light", "base:spell:mage_hand"] {
        assert_eq!(
            refused(&world, &data, spell, rat()),
            Rejection::WrongTarget,
            "{spell}"
        );
    }
    // With no points the costed spell is refused before anything, and the cantrip is not.
    let mut broke = world.clone();
    broke.party.members[WIZARD].spell_points = 0;
    assert!(matches!(
        refused(&broke, &data, MISSILE, rat()),
        Rejection::NotEnoughPoints { .. }
    ));
    assert_eq!(listed(&broke, &data, BOLT)[0].refusal, None);
}

/// The spell's range holds for a group as for anything: a tile's range does not reach two, and
/// a touch reaches the group the party could walk up to, the one in the cell it faces.
#[test]
fn a_group_beyond_the_spell_s_range_is_not_listed() {
    let ranged = |targeting: Targeting| {
        let mut data = objects();
        let id = data.registry.spells.get(BOLT).unwrap();
        data.spells.get_mut(&id).unwrap().targeting = Some(targeting);
        data
    };
    let data = ranged(Targeting::of(Range::Tiles(1)));
    let world = at(&data, 3, 1, Facing::East);
    assert!(listed(&world, &data, BOLT).is_empty(), "two tiles off");
    assert_eq!(refused(&world, &data, BOLT, rat()), Rejection::BeyondRange);
    assert_eq!(listed(&at(&data, 4, 1, Facing::West), &data, BOLT).len(), 1);

    let data = ranged(Targeting::TOUCH);
    assert!(listed(&at(&data, 3, 1, Facing::East), &data, BOLT).is_empty());
    assert!(
        listed(&at(&data, 4, 1, Facing::West), &data, BOLT).is_empty(),
        "beside them, back turned"
    );
    let world = at(&data, 4, 1, Facing::East);
    let rows = listed(&world, &data, BOLT);
    assert_eq!(rows.len(), 1, "the group in the faced cell");
    assert_eq!((rows[0].distance, rows[0].refusal.as_ref()), (1, None));
}

/// The strike: paid, the group met as hostile, the spell's damage on it, the monsters surprised.
#[test]
fn the_strike_is_paid_lands_and_starts_the_fight_with_the_monsters_surprised() {
    let mut data = objects();
    // A peaceful group turns hostile when struck.
    let yard = yard(&data);
    let map = data.maps.get_mut(&yard).unwrap();
    map.encounters
        .iter_mut()
        .find(|e| e.id == "rat")
        .unwrap()
        .disposition = Disposition::Friendly;
    let mut world = at(&data, 3, 1, Facing::East);
    let points = world.party.members[WIZARD].spell_points;
    let command = cast(&world, MISSILE, rat());
    let events = apply(&mut world, &data, command).unwrap();
    assert!(world.party.members[WIZARD].spell_points < points, "paid");
    let at_of = |want: fn(&Event) -> bool| events.iter().position(want).expect("the event");
    let met = at_of(|e| {
        matches!(
            e,
            Event::EncounterStarted {
                source: EncounterSource::Fixed(id),
                disposition: Disposition::Hostile,
                noticed: true,
                stealth: None,
                ..
            } if id == "rat"
        )
    });
    let started = at_of(|e| {
        matches!(
            e,
            Event::CombatStarted {
                surprised: Surprise::Monsters
            }
        )
    });
    let hurt = at_of(|e| {
        matches!(
            e,
            Event::Damage {
                target: ActorRef::Monster { stack: 0, .. },
                amount: 1..,
                ..
            }
        )
    });
    let round = at_of(|e| matches!(e, Event::RoundStarted { round: 1 }));
    assert!(
        met < started && started < hurt && hurt < round,
        "{events:?}"
    );
    let state = fight(&world);
    assert_eq!(state.surprised, Surprise::Monsters);
    assert_eq!(state.encounter.disposition, Disposition::Hostile);
    assert_eq!(state.encounter.source, EncounterSource::Fixed("rat".into()));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::TimeAdvanced { .. })),
        "the fight keeps the time from here"
    );
    // The group no longer stands on the map: it is being fought.
    assert!(
        omnis_sim::beings::on_level(&world, &data, yard, Level::GROUND).is_empty(),
        "met"
    );
}

/// The caster's action for the first round is the strike: when their turn comes it is spent,
/// no stack acts before round two, and in round two the caster acts as anyone.
#[test]
fn the_caster_s_first_action_is_spent_and_no_monster_acts_in_round_one() {
    let data = objects();
    let mut world = at(&data, 3, 1, Facing::East);
    let wizard = world.party.members[WIZARD].id;
    // The cantrip, so the rats live to round two.
    let command = cast(&world, BOLT, rat());
    let mut events = apply(&mut world, &data, command).unwrap();
    let first = fight(&world);
    if first.current_actor() != Some(ActorRef::Member(wizard)) {
        assert_eq!(first.struck, Some(wizard), "marked until their turn begins");
    }
    let mut seen_spent = false;
    for _ in 0..40 {
        let state = fight(&world);
        if state.round > 1 {
            break;
        }
        let acting = state.current_actor();
        if acting == Some(ActorRef::Member(wizard)) {
            assert_eq!(state.budget.actions, 0, "the strike was the action");
            let dodge = Command::Combat(CombatCommand::Dodge);
            let mut copy = world.clone();
            assert_eq!(apply(&mut copy, &data, dodge), Err(Rejection::NoActionLeft));
            seen_spent = true;
            let end = Command::Combat(CombatCommand::EndTurn);
            events.extend(apply(&mut world, &data, end).unwrap());
        } else {
            assert_eq!(state.budget.actions, 1, "{acting:?} has their action");
            let dodge = Command::Combat(CombatCommand::Dodge);
            events.extend(apply(&mut world, &data, dodge).unwrap());
        }
    }
    assert!(seen_spent, "the wizard's turn came in round one");
    let second = events
        .iter()
        .position(|e| matches!(e, Event::RoundStarted { round: 2 }))
        .expect("round two");
    assert!(
        !events[..second].iter().any(|e| matches!(
            e,
            Event::Turn {
                actor: ActorRef::Stack(_)
            }
        )),
        "surprised, the rats do not act in round one"
    );
    // The strike is taken off once: the mark is gone, so round two is a turn like any other.
    if let Mode::Combat(state) = &world.mode {
        assert_eq!(state.struck, None);
    }
}

/// A pack may turn the surprise off (`opening_strike_surprise`, on unless written 0): the
/// strike still lands and the fight starts even.
#[test]
fn a_pack_may_turn_the_surprise_off() {
    let mut data = objects();
    data.rules.insert_value("opening_strike_surprise", 0);
    let mut world = at(&data, 3, 1, Facing::East);
    let command = cast(&world, BOLT, rat());
    let events = apply(&mut world, &data, command).unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::CombatStarted {
            surprised: Surprise::None
        }
    )));
}

/// A strike that fells the whole group ends the fight at once: the party is back on the road
/// and the group, met once, is gone for good.
#[test]
fn a_strike_that_fells_the_group_ends_the_fight() {
    let mut data = objects();
    let yard = yard(&data);
    let rat_id = data
        .registry
        .monsters
        .get("objects:monster:pack_rat")
        .unwrap();
    // One rat of one hit point: three darts are more than enough.
    data.monsters.get_mut(&rat_id).unwrap().hit_points = omnis_core::Dice {
        count: 1,
        sides: 1,
        modifier: 0,
    };
    let map = data.maps.get_mut(&yard).unwrap();
    map.encounters
        .iter_mut()
        .find(|e| e.id == "rat")
        .unwrap()
        .stacks[0]
        .1 = 1;
    let mut world = at(&data, 3, 1, Facing::East);
    let command = cast(&world, MISSILE, rat());
    let events = apply(&mut world, &data, command).unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::CombatEnded {
                outcome: omnis_sim::CombatOutcome::Victory,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(matches!(world.mode, Mode::Explore));
    assert!(world.maps[&yard].cleared.contains("rat"));
    assert!(
        listed(&world, &data, BOLT).is_empty(),
        "no one left to strike"
    );
}
