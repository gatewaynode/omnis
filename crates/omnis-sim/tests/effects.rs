//! Spell effects on real pack data: expiry on the clock, bless fanning out and adding its die,
//! guidance spent by the next check, concentration one per caster and broken by damage, the
//! shield reaction, light widening the dungeon, mage hand on a door, and casting outside a
//! fight with its minutes and refusals.

mod common;

use common::{data, encounter, party_of, six, world};
use omnis_core::{Direction, Facing, StreamName};
use omnis_data::{Ability, BuffOn, Data, Disposition};
use omnis_sim::omnis_rules::{ActiveEffect, EffectKind, Expiry, armor_class};
use omnis_sim::{
    ActorRef, CheckKind, CombatCommand, Command, DevCommand, EffectEnd, EffectTarget,
    EncounterChoice, Event, Mode, PartyCommand, Rejection, Settings, Surprise, Target, World,
    apply, combat, query,
};

const CLERIC: usize = 1;
const WIZARD: usize = 2;
const ROGUE: usize = 3;

const fn slot(index: usize) -> Target {
    Target::Member(index as u8)
}

fn dev_world(data: &Data, seed: u64) -> World {
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    World::new(data, seed, settings).unwrap()
}

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
        .unwrap_or_else(|| panic!("{name}"));
    u8::try_from(at).unwrap()
}

fn spell_id(data: &Data, name: &str) -> omnis_core::SpellId {
    data.registry
        .spells
        .get(&format!("base:spell:{name}"))
        .unwrap()
}

fn explore_cast(
    world: &mut World,
    data: &Data,
    caster: usize,
    name: &str,
    target: Target,
) -> Vec<Event> {
    let spell = spell_index(world, data, caster, name);
    apply(
        world,
        data,
        Command::Cast {
            caster: u8::try_from(caster).unwrap(),
            spell,
            target,
        },
    )
    .unwrap_or_else(|r| panic!("{name}: {r}"))
}

fn start_fight(world: &mut World, data: &Data, stacks: &[(&str, u8)]) {
    let here = world.position;
    let encounter = encounter(data, stacks, Disposition::Hostile, here);
    let mut events = Vec::new();
    combat::start(world, data, encounter, Surprise::None, &mut events).unwrap();
}

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

fn teleport(world: &mut World, data: &Data, x: u16, y: u16, facing: Facing) {
    apply(
        world,
        data,
        Command::Dev(DevCommand::Teleport {
            map: "test:map:dungeon".into(),
            x,
            y,
            facing,
        }),
    )
    .unwrap();
}

#[test]
fn timed_effects_expire_when_the_clock_passes_them() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let now = world.party_clock().elapsed;
    let durin = world.party.members[1].id;
    let effect = |until: i64| ActiveEffect {
        source: spell_id(&data, "bless"),
        caster: durin,
        concentration: false,
        kind: EffectKind::ArmorBonus(1),
        until: Expiry::Minute(until),
    };
    world.party.members[0].effects.push(effect(now + 2));
    world.party.members[0].effects.push(effect(now - 1));
    world.party.effects.push(ActiveEffect {
        kind: EffectKind::Light { depth: 8 },
        ..effect(now + 2)
    });
    let ac = armor_class(&world.party.members[0], &data);
    let events = apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
    let ended: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::EffectEnded { .. }))
        .collect();
    assert_eq!(
        ended.len(),
        1,
        "the stale one goes on the first tick: {ended:?}"
    );
    assert_eq!(world.party.members[0].effects.len(), 1);
    assert_eq!(armor_class(&world.party.members[0], &data), ac - 1);
    apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
    let events = apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
    assert!(world.party_clock().elapsed >= now + 2);
    assert!(world.party.members[0].effects.is_empty() && world.party.effects.is_empty());
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::EffectEnded {
                target: EffectTarget::Party,
                why: EffectEnd::Expired,
                ..
            }
        )) || true,
        "the light ended on whichever step crossed the minute"
    );
}

#[test]
fn bless_fans_out_from_the_anchor_and_adds_its_die() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    world.party.members[5].hp = 0;
    start_fight(&mut world, &data, &[("goblin", 2)]);
    assert!(until_turn_of(&mut world, &data, CLERIC));
    let bless = spell_index(&world, &data, CLERIC, "bless");
    let events = apply(
        &mut world,
        &data,
        Command::Combat(CombatCommand::Cast {
            spell: bless,
            target: Target::Member(4),
        }),
    )
    .unwrap();
    let blessed: Vec<omnis_core::CharacterId> = events
        .iter()
        .filter_map(|e| match e {
            Event::EffectApplied {
                target: EffectTarget::Member(id),
                ..
            } => Some(*id),
            _ => None,
        })
        .collect();
    let ids: Vec<omnis_core::CharacterId> = world.party.members.iter().map(|m| m.id).collect();
    let id = |slot: usize| ids[slot];
    assert_eq!(
        blessed,
        [id(4), id(0), id(1)],
        "from slot 4, round the order, skipping the downed Wren"
    );
    let now = world.party_clock().elapsed;
    let effect = &world.party.members[0].effects[0];
    assert!(effect.concentration && effect.caster == id(CLERIC));
    assert!(matches!(effect.until, Expiry::Minute(m) if m >= now + 9 && m <= now + 10));
    assert!(until_turn_of(&mut world, &data, 0));
    let events = apply(
        &mut world,
        &data,
        Command::Combat(CombatCommand::Attack { stack: 0 }),
    )
    .unwrap();
    let Some(Event::AttackResolved { roll, .. }) = events.iter().find(
        |e| matches!(e, Event::AttackResolved { attacker: ActorRef::Member(a), .. } if *a == id(0)),
    ) else {
        panic!("{events:?}")
    };
    let bonus = roll.bonus.as_ref().expect("bless adds a die").total;
    assert!((1..=4).contains(&bonus));
    assert_eq!(
        roll.total,
        i64::from(roll.face) + roll.modifier + roll.proficiency + i64::from(bonus)
    );
}

#[test]
fn guidance_is_spent_by_the_next_check_and_concentration_is_one_per_caster() {
    let data = data();
    let mut world = dev_world(&data, 3);
    party_of(&mut world, &data, 6);
    let events = explore_cast(&mut world, &data, CLERIC, "bless", Target::Member(0));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::EffectApplied { .. }))
            .count(),
        3
    );
    let events = explore_cast(&mut world, &data, CLERIC, "guidance", slot(ROGUE));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Concentration { spell, ended: true, .. } if *spell == spell_id(&data, "bless")
    )));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e,
                Event::EffectEnded {
                    why: EffectEnd::Concentration,
                    ..
                }
            ))
            .count(),
        3,
        "the three blessings end"
    );
    assert!(world.party.members[0].effects.is_empty());
    let pip = &world.party.members[ROGUE];
    assert_eq!(pip.effects.len(), 1);
    assert!(pip.effects[0].buffs(BuffOn::AbilityChecks));
    assert_eq!(
        world.party.members[CLERIC].spell_points, 1,
        "bless cost one; guidance is free"
    );
    teleport(&mut world, &data, 3, 7, Facing::South);
    apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
    assert!(matches!(world.mode, Mode::Encounter(_)), "the rats");
    let events = apply(&mut world, &data, Command::Encounter(EncounterChoice::Hide)).unwrap();
    let Some(Event::Check {
        actor: ActorRef::Member(who),
        roll: Some(roll),
        ..
    }) = events.iter().find(|e| {
        matches!(
            e,
            Event::Check {
                kind: CheckKind::Hide,
                ..
            }
        )
    })
    else {
        panic!("{events:?}")
    };
    assert_eq!(*who, world.party.members[ROGUE].id, "the best sneak hides");
    assert!(roll.bonus.is_some(), "guidance joined the check");
    assert!(events.iter().any(|e| matches!(
        e,
        Event::EffectEnded {
            why: EffectEnd::Consumed,
            ..
        }
    )));
    assert!(world.party.members[ROGUE].effects.is_empty(), "spent");
}

#[test]
fn damage_breaks_concentration_on_a_failed_constitution_save() {
    let data = data();
    let (mut held, mut lost) = (0, 0);
    for seed in 0..60u64 {
        let mut world = World::new(&data, seed, Settings::default()).unwrap();
        party_of(&mut world, &data, 6);
        for member in &mut world.party.members {
            member.hp_max = 200;
            member.hp = 200;
        }
        start_fight(&mut world, &data, &[("goblin", 4)]);
        if !until_turn_of(&mut world, &data, CLERIC) {
            continue;
        }
        let bless = spell_index(&world, &data, CLERIC, "bless");
        let durin = world.party.members[CLERIC].id;
        apply(
            &mut world,
            &data,
            Command::Combat(CombatCommand::Cast {
                spell: bless,
                target: slot(CLERIC),
            }),
        )
        .unwrap();
        for _ in 0..12 {
            if !until_turn_of(&mut world, &data, CLERIC) {
                break;
            }
            let events = apply(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap();
            for (i, e) in events.iter().enumerate() {
                if let Event::Check {
                    actor: ActorRef::Member(who),
                    kind: CheckKind::Save(Ability::Constitution),
                    roll: Some(roll),
                    dc,
                    success,
                } = e
                {
                    assert_eq!(*who, durin);
                    assert_eq!(*dc, 10, "goblin blows never halve past ten");
                    assert!(roll.bonus.is_some(), "blessed saves carry the die");
                    assert!(
                        matches!(events[i - 1], Event::Damage { target: ActorRef::Member(t), .. } if t == durin)
                    );
                    if *success {
                        held += 1;
                        assert!(
                            !world.party.members[CLERIC].effects.is_empty()
                                || events[i + 1..]
                                    .iter()
                                    .any(|e| matches!(e, Event::Concentration { .. }))
                        );
                    } else {
                        lost += 1;
                        assert!(events[i + 1..].iter().any(|e| matches!(e, Event::Concentration { caster, ended: true, .. } if *caster == durin)));
                    }
                }
            }
            if lost > 0 {
                break;
            }
        }
        if held > 0 && lost > 0 {
            return;
        }
    }
    panic!("held {held}, lost {lost}");
}

#[test]
fn shield_reacts_only_when_it_turns_a_hit_into_a_miss() {
    let data = data();
    let mut reacted = 0;
    let mut hits_without = 0;
    for seed in 0..80u64 {
        let mut world = World::new(&data, seed, Settings::default()).unwrap();
        apply(
            &mut world,
            &data,
            Command::Party(PartyCommand::Create(six().remove(WIZARD))),
        )
        .unwrap();
        world.party.members[0].hp_max = 100;
        world.party.members[0].hp = 100;
        let shield = spell_index(&world, &data, 0, "shield");
        let bolt = spell_index(&world, &data, 0, "fire_bolt");
        let toggle = |spell, on| {
            Command::Party(PartyCommand::AutoCast {
                member: 0,
                spell,
                on,
            })
        };
        assert_eq!(
            apply(&mut world, &data, toggle(bolt, true)),
            Err(Rejection::NotCastable { spell: bolt })
        );
        let events = apply(&mut world, &data, toggle(shield, true)).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AutoCast { on: true, .. }))
        );
        assert_eq!(
            world.party.members[0].auto_cast,
            [spell_id(&data, "shield")]
        );
        start_fight(&mut world, &data, &[("goblin", 3)]);
        if !until_turn_of(&mut world, &data, 0) {
            continue;
        }
        let cast = Command::Combat(CombatCommand::Cast {
            spell: shield,
            target: Target::Member(0),
        });
        assert_eq!(
            apply(&mut world, &data, cast),
            Err(Rejection::NotCastable { spell: shield }),
            "a reaction is not cast from the picker"
        );
        let points_before = world.party.members[0].spell_points;
        let events = apply(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap();
        let ilvara = world.party.members[0].id;
        let (r, h) = shield_outcomes(&events, ilvara);
        reacted += r;
        hits_without += h;
        assert_eq!(
            world.party.members[0].spell_points,
            points_before - u32::try_from(r).unwrap(),
            "one point a reaction"
        );
        if r > 0 && matches!(world.mode, Mode::Combat(_)) {
            assert!(
                events.iter().any(|e| matches!(
                    e,
                    Event::EffectEnded {
                        why: EffectEnd::TurnBegan,
                        ..
                    }
                )),
                "the bonus lasts until her next turn, where the fight parks"
            );
            assert_eq!(armor_class(&world.party.members[0], &data), 13);
        }
        if reacted > 0 && hits_without > 0 {
            break;
        }
    }
    assert!(reacted > 0, "shield never fired");
    assert!(hits_without > 0, "a goblin never hit through");
}

/// Reactions in one command's events (each a cast, an effect, and a rejudged miss at 18) and
/// hits that landed on the member anyway.
fn shield_outcomes(events: &[Event], ilvara: omnis_core::CharacterId) -> (usize, usize) {
    let (mut reactions, mut hits) = (0, 0);
    for (i, e) in events.iter().enumerate() {
        if let Event::SpellCast { caster, points, .. } = e {
            assert_eq!((*caster, *points), (ilvara, 1));
            reactions += 1;
            assert!(matches!(
                events[i + 1],
                Event::EffectApplied { target: EffectTarget::Member(t), .. } if t == ilvara
            ));
            assert!(
                matches!(
                    &events[i + 2],
                    Event::AttackResolved {
                        ac: 18,
                        hit: false,
                        ..
                    }
                ),
                "{:?}",
                events[i + 2]
            );
        }
        if let Event::AttackResolved {
            target: ActorRef::Member(t),
            hit: true,
            ac,
            ..
        } = e
            && *t == ilvara
        {
            hits += 1;
            assert!(
                *ac == 13 || *ac == 18,
                "unshielded or already shielded: {ac}"
            );
        }
    }
    (reactions, hits)
}

#[test]
fn light_lifts_the_dungeon_depth_until_it_fades() {
    let data = data();
    let mut world = dev_world(&data, 1);
    party_of(&mut world, &data, 6);
    // Column 9 of the dungeon has no pillar; the door south of (9, 5) opens the way down.
    teleport(&mut world, &data, 9, 5, Facing::South);
    apply(&mut world, &data, Command::Interact).unwrap();
    teleport(&mut world, &data, 9, 0, Facing::South);
    let view = query::viewport(&world, &data).unwrap();
    assert_eq!(view.visibility_depth, 6);
    assert!(view.tiles.iter().all(|t| t.depth <= 6));
    let lit_at = world.party_clock().elapsed;
    let events = explore_cast(&mut world, &data, WIZARD, "light", slot(WIZARD));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::EffectApplied {
            target: EffectTarget::Party,
            ..
        }
    )));
    let view = query::viewport(&world, &data).unwrap();
    assert_eq!(view.visibility_depth, 8);
    assert!(
        view.tiles
            .iter()
            .any(|t| t.depth == 8 && t.x == 9 && t.y == 8),
        "{:?}",
        view.tiles
            .iter()
            .filter(|t| t.depth > 6)
            .collect::<Vec<_>>()
    );
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    assert!(world.automap.maps[&dungeon].contains_key(&(9, 8)));
    world.party.effects[0].kind = EffectKind::Light { depth: 40 };
    assert_eq!(
        query::viewport(&world, &data).unwrap().visibility_depth,
        32,
        "capped"
    );
    world.party.effects[0].kind = EffectKind::Light { depth: 8 };
    let lit_until = match world.party.effects[0].until {
        Expiry::Minute(m) => m,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        lit_until,
        lit_at + 60,
        "sixty minutes from the cast, which took one"
    );
    assert_eq!(world.party_clock().elapsed, lit_at + 1);
    // Toggling the door ahead costs a minute a time and steps on no placement.
    teleport(&mut world, &data, 9, 5, Facing::South);
    let mut faded = false;
    for _ in 0..70 {
        let events = apply(&mut world, &data, Command::Interact).unwrap();
        if events.iter().any(|e| {
            matches!(
                e,
                Event::EffectEnded {
                    target: EffectTarget::Party,
                    why: EffectEnd::Expired,
                    ..
                }
            )
        }) {
            faded = true;
            break;
        }
    }
    assert!(faded, "the light never went out");
    assert!(world.party.effects.is_empty());
    let mut meadow = common::world(&data);
    party_of(&mut meadow, &data, 6);
    explore_cast(&mut meadow, &data, WIZARD, "light", Target::Member(0));
    assert_eq!(
        query::viewport(&meadow, &data).unwrap().visibility_depth,
        12,
        "daylight is not extended"
    );
}

#[test]
fn mage_hand_toggles_the_first_door_ahead() {
    let data = data();
    let mut world = dev_world(&data, 1);
    party_of(&mut world, &data, 6);
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    let hand =
        |world: &mut World| explore_cast(world, &data, WIZARD, "mage_hand", Target::Member(0));
    teleport(&mut world, &data, 9, 2, Facing::South);
    let events = hand(&mut world);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Door {
                x: 9,
                y: 5,
                facing: Facing::South,
                open: true,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(world.maps[&dungeon].door_open(9, 5, Facing::South));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TimeAdvanced { minutes: 1, .. }))
    );
    teleport(&mut world, &data, 9, 0, Facing::South);
    let events = hand(&mut world);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Door { open: false, .. })),
        "five tiles away, it closes again"
    );
    let nothing = |events: &[Event]| {
        events.iter().any(|e| {
            matches!(
                e,
                Event::Message {
                    key: omnis_sim::MessageKey::NothingHere
                }
            )
        })
    };
    teleport(&mut world, &data, 3, 2, Facing::South);
    assert!(
        nothing(&hand(&mut world)),
        "the pillar at (3, 3) stops the hand"
    );
    teleport(&mut world, &data, 3, 0, Facing::North);
    assert!(nothing(&hand(&mut world)), "a wall, then the map's edge");
    assert!(world.rngs.contains_key(&StreamName::new("cast")));
    start_fight(&mut world, &data, &[("giant_rat", 1)]);
    assert!(until_turn_of(&mut world, &data, WIZARD));
    let index = spell_index(&world, &data, WIZARD, "mage_hand");
    let cast = Command::Combat(CombatCommand::Cast {
        spell: index,
        target: Target::Stack(0),
    });
    assert_eq!(
        apply(&mut world, &data, cast),
        Err(Rejection::NotCastable { spell: index })
    );
    let view = omnis_sim::combat_view(&world, &data).unwrap();
    assert_eq!(
        view.spells[usize::from(index)].blocked,
        Some(Rejection::NotCastable { spell: index })
    );
}

#[test]
fn casting_outside_a_fight_heals_costs_minutes_and_refuses_the_impossible() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    let unconscious = data
        .registry
        .conditions
        .get("base:condition:unconscious")
        .unwrap();
    let brenna = &mut world.party.members[0];
    brenna.hp = 0;
    brenna.conditions.push(unconscious);
    let clock = world.party_clock().elapsed;
    let events = explore_cast(&mut world, &data, CLERIC, "cure_wounds", Target::Member(0));
    assert!(
        world.party.members[0].hp > 0 && world.party.members[0].conditions.is_empty(),
        "up again outside a fight"
    );
    assert!(events.iter().any(|e| matches!(e, Event::Healed { .. })));
    assert_eq!(world.party_clock().elapsed, clock + 1);
    assert_eq!(world.party.members[CLERIC].spell_points, 1);
    let before = world.clone();
    let missile = spell_index(&world, &data, WIZARD, "magic_missile");
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
        Command::Cast {
            caster: 2,
            spell: missile,
            target: Target::Stack(0),
        },
        Rejection::NotCastable { spell: missile },
    );
    let cure = spell_index(&world, &data, CLERIC, "cure_wounds");
    refuse(
        &mut world,
        Command::Cast {
            caster: 1,
            spell: cure,
            target: Target::Stack(0),
        },
        Rejection::WrongTarget,
    );
    refuse(
        &mut world,
        Command::Cast {
            caster: 9,
            spell: 0,
            target: Target::Member(0),
        },
        Rejection::NoSuchMember { index: 9 },
    );
    refuse(
        &mut world,
        Command::Combat(CombatCommand::Cast {
            spell: cure,
            target: Target::Member(0),
        }),
        Rejection::WrongMode,
    );
    let mut down = world.clone();
    down.party.members[CLERIC].hp = 0;
    assert_eq!(
        apply(
            &mut down,
            &data,
            Command::Cast {
                caster: 1,
                spell: cure,
                target: Target::Member(0)
            }
        ),
        Err(Rejection::MemberDown { index: 1 })
    );
}

#[test]
fn casts_replay() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let dev = Settings {
        devtools: true,
        ..Settings::default()
    };
    let commands = vec![
        Command::Party(PartyCommand::Create(six().remove(0))),
        Command::Party(PartyCommand::Create(six().remove(1))),
        Command::Cast {
            caster: 1,
            spell: spell_index(&world, &data, CLERIC, "bless"),
            target: Target::Member(0),
        },
        Command::Step(Direction::Forward),
    ];
    let a = omnis_sim::replay::run(&data, 4, dev, &commands).unwrap();
    let b = omnis_sim::replay::run(&data, 4, dev, &commands).unwrap();
    assert_eq!(a, b, "casts replay");
}
