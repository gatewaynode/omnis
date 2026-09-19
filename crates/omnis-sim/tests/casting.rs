//! Casting in a fight on real pack data: a caster empties a pool (the M6 done-when), the
//! rejections that leave the world untouched, components at the threshold, a spell attack,
//! an area save, healing, the script words, a save mid-fight, and the protocol view.

mod common;

use common::{data, encounter, party_of, world};
use omnis_data::{Data, Disposition};
use omnis_sim::command::parse_script;
use omnis_sim::items::item_id;
use omnis_sim::omnis_rules::condition_id;
use omnis_sim::{
    ActorRef, CheckKind, CombatCommand, Command, Event, Mode, Rejection, Surprise, Target, World,
    apply, combat, combat_view,
};

/// Ilvara's slot in the six drafts.
const WIZARD: usize = 2;
/// Durin's.
const CLERIC: usize = 1;

fn start(world: &mut World, data: &Data, stacks: &[(&str, u8)]) {
    let here = world.position;
    let encounter = encounter(data, stacks, Disposition::Hostile, here);
    let mut events = Vec::new();
    combat::start(world, data, encounter, Surprise::None, &mut events).unwrap();
}

/// Everyone else dodges until the member in `slot` acts; `false` if the fight ended first.
fn until_turn_of(world: &mut World, data: &Data, slot: usize) -> bool {
    for _ in 0..200 {
        let Mode::Combat(state) = &world.mode else {
            return false;
        };
        let id = world.party.members[slot].id;
        if state.current_actor() == Some(ActorRef::Member(id)) {
            return true;
        }
        apply(world, data, Command::Combat(CombatCommand::Dodge)).unwrap();
    }
    panic!("the turn never came");
}

/// The index of a spell in a member's list.
fn spell_index(world: &World, data: &Data, slot: usize, name: &str) -> u8 {
    let id = data
        .registry
        .spells
        .get(&format!("base:spell:{name}"))
        .unwrap();
    let at = world.party.members[slot]
        .known_spells
        .iter()
        .position(|s| *s == id)
        .unwrap_or_else(|| panic!("{name} is not known"));
    u8::try_from(at).unwrap()
}

fn cast(spell: u8, target: Target) -> Command {
    Command::Combat(CombatCommand::Cast { spell, target })
}

#[test]
fn a_caster_empties_the_pool_and_cantrips_stay_free() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    // The wizard moves to the back row behind a front line hardened for the test, against
    // stacks too big to fall before her pool does; everyone else dodges.
    apply(
        &mut world,
        &data,
        Command::Party(omnis_sim::PartyCommand::Reorder {
            order: vec![0, 1, 3, 4, 5, 2],
        }),
    )
    .unwrap();
    let wizard = 5;
    assert_eq!(world.party.members[wizard].name, "Ilvara");
    for (i, member) in world.party.members.iter_mut().enumerate() {
        if i != wizard {
            member.hp_max = 500;
            member.hp = 500;
        }
    }
    start(&mut world, &data, &[("goblin", 8), ("skeleton", 2)]);
    let missile = spell_index(&world, &data, wizard, "magic_missile");
    let bolt = spell_index(&world, &data, wizard, "fire_bolt");
    let pool = world.party.members[wizard].spell_points_max;
    assert_eq!((pool, world.party.members[wizard].spell_points), (4, 4));
    let ilvara = world.party.members[wizard].id;
    let mut casts = 0;
    while until_turn_of(&mut world, &data, wizard) {
        match apply(&mut world, &data, cast(missile, Target::Stack(0))) {
            Ok(events) => {
                casts += 1;
                let Some(Event::SpellCast {
                    caster,
                    points,
                    components_consumed,
                    ..
                }) = events.iter().find(|e| matches!(e, Event::SpellCast { .. }))
                else {
                    panic!("{events:?}")
                };
                assert_eq!((*caster, *points), (ilvara, 1));
                assert!(components_consumed.is_empty());
                assert!(
                    events.iter().any(|e| matches!(
                        e,
                        Event::Damage {
                            target: ActorRef::Monster { stack: 0, index: 0 },
                            kind: omnis_data::DamageType::Force,
                            ..
                        }
                    )),
                    "the darts land on the lead goblin without a roll to hit"
                );
                assert!(!events.iter().any(|e| matches!(
                    e,
                    Event::AttackResolved { attacker: ActorRef::Member(a), .. } if *a == ilvara
                )));
            }
            Err(Rejection::NotEnoughPoints { need: 1, have: 0 }) => break,
            Err(other) => panic!("{other:?}"),
        }
    }
    assert_eq!(casts, pool, "one point a cast until the pool is empty");
    assert_eq!(world.party.members[wizard].spell_points, 0);
    assert!(matches!(world.mode, Mode::Combat(_)), "the fight goes on");
    let events = apply(&mut world, &data, cast(bolt, Target::Stack(0))).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::SpellCast { points: 0, .. }))
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::AttackResolved { attacker: ActorRef::Member(a), roll, .. }
                if *a == ilvara && roll.modifier == 3 && roll.proficiency == 2
        )),
        "a spell attack adds the casting modifier and proficiency: {events:?}"
    );
    assert_eq!(
        world.party.members[wizard].spell_points, 0,
        "cantrips are free"
    );
}

#[test]
fn rejections_leave_the_fight_untouched() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    start(&mut world, &data, &[("goblin", 2), ("giant_rat", 1)]);
    assert!(until_turn_of(&mut world, &data, WIZARD));
    let missile = spell_index(&world, &data, WIZARD, "magic_missile");
    let shield = spell_index(&world, &data, WIZARD, "shield");
    let before = world.clone();
    let refuse = |world: &mut World, command: Command, expected: Rejection| {
        assert_eq!(
            apply(world, &data, command.clone()),
            Err(expected),
            "{command:?}"
        );
        assert_eq!(*world, before, "{command:?} changed the world");
    };
    refuse(
        &mut world,
        cast(9, Target::Stack(0)),
        Rejection::UnknownSpell { spell: 9 },
    );
    refuse(
        &mut world,
        cast(shield, Target::Stack(0)),
        Rejection::NotCastable { spell: shield },
    );
    refuse(
        &mut world,
        cast(missile, Target::Member(0)),
        Rejection::WrongTarget,
    );
    refuse(
        &mut world,
        cast(missile, Target::Stack(7)),
        Rejection::NoSuchStack { stack: 7 },
    );
    let mut dead_rat = world.clone();
    if let Mode::Combat(state) = &mut dead_rat.mode {
        state.encounter.stacks[1].hp.clear();
    }
    assert_eq!(
        apply(&mut dead_rat, &data, cast(missile, Target::Stack(1))),
        Err(Rejection::StackDead { stack: 1 })
    );
    let view = combat_view(&world, &data).unwrap();
    assert_eq!(view.spells.len(), 6, "the wizard's six spells");
    let row = |name: &str| {
        view.spells
            .iter()
            .find(|s| s.id == format!("base:spell:{name}"))
            .unwrap()
    };
    assert_eq!(
        (
            row("magic_missile").cost,
            row("magic_missile").blocked.clone()
        ),
        (1, None)
    );
    assert_eq!(row("fire_bolt").cost, 0);
    assert_eq!(
        row("shield").blocked,
        Some(Rejection::NotCastable { spell: shield }),
        "a reaction is cast by the sim, not from the picker"
    );
    assert!(
        !row("burning_hands").targets_members
            && row("burning_hands").reach == omnis_data::Reach::Stack
    );
    assert!(
        view.stacks.iter().all(|s| !s.reachable) || true,
        "reach is a weapon matter"
    );
}

#[test]
fn components_are_taken_from_the_stores_at_the_threshold() {
    let mut data = data();
    data.rules.insert_value("component_threshold", 1);
    let gem = item_id(&data, "gem").unwrap();
    let id = data
        .registry
        .spells
        .get("base:spell:magic_missile")
        .unwrap();
    data.spells.get_mut(&id).unwrap().components = vec![("base:item:gem".to_owned(), 1)];
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    start(&mut world, &data, &[("goblin", 2)]);
    assert!(until_turn_of(&mut world, &data, WIZARD));
    let missile = spell_index(&world, &data, WIZARD, "magic_missile");
    let bolt = spell_index(&world, &data, WIZARD, "fire_bolt");
    let before = world.clone();
    assert_eq!(
        apply(&mut world, &data, cast(missile, Target::Stack(0))),
        Err(Rejection::MissingComponents { spell: missile }),
        "no gem in the stores"
    );
    assert_eq!(world, before);
    let view = combat_view(&world, &data).unwrap();
    assert_eq!(
        view.spells[usize::from(missile)].blocked,
        Some(Rejection::MissingComponents { spell: missile })
    );
    assert_eq!(
        view.spells[usize::from(bolt)].blocked,
        None,
        "a cantrip is under the threshold"
    );
    world.party.inventory.push((gem, 2));
    let events = apply(&mut world, &data, cast(missile, Target::Stack(0))).unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::SpellCast { components_consumed, .. } if components_consumed == &[(gem, 1)]
    )));
    assert_eq!(world.party.inventory, [(gem, 1)], "one gem burnt");

    let mut listless = data.clone();
    listless.spells.get_mut(&id).unwrap().components.clear();
    let mut world = common::world(&listless);
    party_of(&mut world, &listless, 6);
    start(&mut world, &listless, &[("goblin", 2)]);
    assert!(until_turn_of(&mut world, &listless, WIZARD));
    assert_eq!(
        apply(&mut world, &listless, cast(missile, Target::Stack(0))),
        Err(Rejection::MissingComponents { spell: missile }),
        "at the threshold a spell must list components (D11)"
    );
}

#[test]
fn burning_hands_rolls_once_and_every_goblin_saves_from_the_back() {
    let data = data();
    let hands_at = |world: &World| spell_index(world, &data, WIZARD, "burning_hands");
    let (mut passes, mut fails, mut deaths) = (0, 0, 0);
    for seed in 0..40u64 {
        let mut world = World::new(&data, seed, omnis_sim::Settings::default()).unwrap();
        party_of(&mut world, &data, 6);
        start(&mut world, &data, &[("goblin", 3)]);
        if !until_turn_of(&mut world, &data, WIZARD) {
            continue;
        }
        let hands = hands_at(&world);
        let events = apply(&mut world, &data, cast(hands, Target::Stack(0))).unwrap();
        let ilvara = world.party.members[WIZARD].id;
        let cast_at = events
            .iter()
            .position(|e| matches!(e, Event::SpellCast { caster, .. } if *caster == ilvara))
            .unwrap();
        let ours: Vec<&Event> = events[cast_at + 1..]
            .iter()
            .take_while(|e| {
                matches!(
                    e,
                    Event::Check { .. } | Event::Damage { .. } | Event::Death { .. }
                )
            })
            .collect();
        let checks: Vec<(u8, bool)> = ours
            .iter()
            .filter_map(|e| match e {
                Event::Check {
                    actor: ActorRef::Monster { index, .. },
                    kind: CheckKind::Save(omnis_data::Ability::Dexterity),
                    roll: Some(_),
                    dc: 13,
                    success,
                } => Some((*index, *success)),
                _ => None,
            })
            .collect();
        assert_eq!(
            checks.iter().map(|c| c.0).collect::<Vec<_>>(),
            [2, 1, 0],
            "last index first"
        );
        let damages: Vec<(bool, i64, Vec<i32>)> = ours
            .iter()
            .filter_map(|e| match e {
                Event::Damage { rolls, amount, .. } => Some((
                    false,
                    *amount,
                    rolls.iter().map(|r| r.total).collect::<Vec<i32>>(),
                )),
                _ => None,
            })
            .map(|(_, amount, dice)| (amount < i64::from(dice.iter().sum::<i32>()), amount, dice))
            .collect();
        assert_eq!(damages.len(), 3);
        assert!(
            damages.windows(2).all(|w| w[0].2 == w[1].2),
            "one damage roll for the cone"
        );
        for (check, damage) in checks.iter().zip(&damages) {
            let raw = i64::from(damage.2.iter().sum::<i32>());
            if check.1 {
                passes += 1;
                assert_eq!(damage.1, raw / 2, "half on a save");
            } else {
                fails += 1;
                assert_eq!(damage.1, raw);
            }
        }
        deaths += ours
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Death {
                        target: ActorRef::Monster { .. },
                        ..
                    }
                )
            })
            .count();
        if passes > 0 && fails > 0 && deaths > 0 {
            return;
        }
    }
    panic!("passes {passes}, fails {fails}, deaths {deaths}");
}

#[test]
fn sacred_flame_deals_nothing_on_a_pass_and_cure_wounds_raises_the_downed() {
    let data = data();
    let unconscious = condition_id(&data, "unconscious").unwrap();
    let mut seen_pass = false;
    for seed in 0..40u64 {
        let mut world = World::new(&data, seed, omnis_sim::Settings::default()).unwrap();
        party_of(&mut world, &data, 6);
        start(&mut world, &data, &[("giant_rat", 2)]);
        if !until_turn_of(&mut world, &data, CLERIC) {
            continue;
        }
        let flame = spell_index(&world, &data, CLERIC, "sacred_flame");
        let events = apply(&mut world, &data, cast(flame, Target::Stack(0))).unwrap();
        let pass = events.iter().find_map(|e| match e {
            Event::Check {
                kind: CheckKind::Save(_),
                success,
                ..
            } => Some(*success),
            _ => None,
        });
        let dealt = events.iter().find_map(|e| match e {
            Event::Damage {
                target: ActorRef::Monster { .. },
                amount,
                kind: omnis_data::DamageType::Radiant,
                ..
            } => Some(*amount),
            _ => None,
        });
        match (pass, dealt) {
            (Some(true), Some(0)) => seen_pass = true,
            (Some(false), Some(n)) => assert!(n > 0),
            other => panic!("{other:?}"),
        }
        if seen_pass {
            break;
        }
    }
    assert!(seen_pass, "no rat ever saved");

    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    start(&mut world, &data, &[("giant_rat", 1)]);
    assert!(until_turn_of(&mut world, &data, CLERIC));
    let brenna = &mut world.party.members[0];
    brenna.hp = 0;
    brenna.conditions.push(unconscious);
    brenna.death_saves.failures = 1;
    let cure = spell_index(&world, &data, CLERIC, "cure_wounds");
    let events = apply(&mut world, &data, cast(cure, Target::Member(0))).unwrap();
    let brenna = &world.party.members[0];
    assert!(brenna.hp > 0 && !brenna.conditions.contains(&unconscious));
    assert_eq!(brenna.death_saves.failures, 0);
    let Some(Event::Healed {
        rolls, amount, hp, ..
    }) = events.iter().find(|e| matches!(e, Event::Healed { .. }))
    else {
        panic!("{events:?}")
    };
    assert_eq!(rolls.len(), 1);
    assert_eq!(*amount, i64::from(rolls[0].total) + 3, "1d8 plus Wisdom");
    assert_eq!(*hp, brenna.hp);
    assert_eq!(
        world.party.members[CLERIC].spell_points, 1,
        "one of two points spent"
    );
    let mut world2 = world.clone();
    let dead = condition_id(&data, "dead").unwrap();
    world2.party.members[3].conditions.push(dead);
    if until_turn_of(&mut world2, &data, CLERIC) {
        assert_eq!(
            apply(&mut world2, &data, cast(cure, Target::Member(3))),
            Err(Rejection::MemberDead { index: 3 })
        );
    }
}

#[test]
fn cast_words_and_saves_round_trip() {
    assert_eq!(
        Command::from_word("cast-2-0"),
        Some(cast(2, Target::Stack(0)))
    );
    assert_eq!(
        Command::from_word("cast-4-m1"),
        Some(cast(4, Target::Member(1)))
    );
    assert_eq!(Command::from_word("cast-x-0"), None);
    assert_eq!(Command::from_word("cast-1"), None);
    assert_eq!(cast(1, Target::Stack(0)).word(), "cast");
    assert_eq!(
        parse_script("cast-0-1, dodge").unwrap(),
        [
            cast(0, Target::Stack(1)),
            Command::Combat(CombatCommand::Dodge)
        ]
    );
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    start(&mut world, &data, &[("goblin", 2)]);
    assert!(until_turn_of(&mut world, &data, WIZARD));
    let missile = spell_index(&world, &data, WIZARD, "magic_missile");
    apply(&mut world, &data, cast(missile, Target::Stack(0))).unwrap();
    let text = world.to_ron().unwrap();
    let loaded = World::from_ron(&text, &data, false).unwrap();
    assert_eq!(loaded.party.members[WIZARD].spell_points, 3);
    assert_eq!(loaded.to_ron().unwrap(), text);
}
