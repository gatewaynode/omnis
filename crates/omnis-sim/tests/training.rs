//! The trainer and the spell sellers (M7b step 10): a level for `trainer.cost`, the picks it
//! owes chosen one at a time for free, spells bought at the guild and the temple for
//! `spell.learn_cost`, and every refusal leaving the world (its dice streams included) as it
//! was. Party: Brenna (human fighter), Durin (dwarf cleric), Ilvara (elf wizard).

mod common;

use common::{data, inside_with};
use omnis_core::CharacterId;
use omnis_data::Data;
use omnis_sim::{Command, Event, Rejection, ServiceCommand, World, apply};

// Every party here is made by `inside_with` on a new world and never reordered, so its members'
// ids follow their slots: Brenna 0, Durin 1, Ilvara 2.

fn ask(world: &mut World, data: &Data, command: ServiceCommand) -> Vec<Event> {
    apply(world, data, Command::Service(command.clone()))
        .unwrap_or_else(|r| panic!("{command:?}: {r}"))
}

/// The command is refused with `why`, and nothing changed.
fn refused(world: &mut World, data: &Data, command: ServiceCommand, why: Rejection) {
    let before = world.clone();
    assert_eq!(
        apply(world, data, Command::Service(command.clone())),
        Err(why),
        "{command:?}"
    );
    assert_eq!(*world, before, "{command:?} changed the world");
}

/// A base spell's id, as the service commands take it.
fn spell(name: &str) -> String {
    format!("base:spell:{name}")
}

fn minutes(world: &World) -> i64 {
    world.party_clock().elapsed
}

fn learned(events: &[Event]) -> Option<u32> {
    events.iter().find_map(|e| match e {
        Event::SpellLearned { cost, .. } => Some(*cost),
        _ => None,
    })
}

const TRAIN_BRENNA: ServiceCommand = ServiceCommand::Train {
    member: CharacterId(0),
};

#[test]
fn a_level_is_granted_at_the_trainer_for_its_price() {
    let data = data();
    let mut world = inside_with(&data, "trainer", 3);
    refused(
        &mut world,
        &data,
        TRAIN_BRENNA,
        Rejection::NotReady {
            member: CharacterId(0),
            xp: 0,
            needed: 300,
        },
    );
    world.party.members[0].xp = 300;
    world.party.gold = 1999;
    refused(
        &mut world,
        &data,
        TRAIN_BRENNA,
        Rejection::CannotAfford {
            cost: 2000,
            gold: 1999,
        },
    );
    world.party.gold = 5000;
    let clock = minutes(&world);
    let events = ask(&mut world, &data, TRAIN_BRENNA);
    let Some(Event::LevelUp {
        member,
        level,
        cost,
        gains,
    }) = events.iter().find(|e| matches!(e, Event::LevelUp { .. }))
    else {
        panic!("no level: {events:?}");
    };
    let brenna = &world.party.members[0];
    assert_eq!(
        (*member, *level, *cost),
        (brenna.id, 2, 2000),
        "1000 cp a level"
    );
    assert_eq!(gains.hp, 8);
    assert_eq!(gains.features, ["base:text:class.fighter.action_surge"]);
    assert_eq!((brenna.level, brenna.hp_max, brenna.hp), (2, 20, 20));
    assert_eq!(world.party.gold, 3000);
    assert_eq!(minutes(&world), clock + 10, "service_minutes");

    refused(
        &mut world,
        &data,
        TRAIN_BRENNA,
        Rejection::NotReady {
            member: CharacterId(0),
            xp: 300,
            needed: 900,
        },
    );
    world.party.members[0].xp = 900;
    ask(&mut world, &data, TRAIN_BRENNA);
    assert_eq!(world.party.members[0].level, 3);
    assert_eq!(world.party.gold, 0, "3000 for level 3");

    world.party.members[0].level = 20;
    world.party.members[0].xp = u32::MAX;
    refused(
        &mut world,
        &data,
        TRAIN_BRENNA,
        Rejection::MaxLevel {
            member: CharacterId(0),
        },
    );
    refused(
        &mut world,
        &data,
        ServiceCommand::Train {
            member: CharacterId(3),
        },
        Rejection::NoSuchMember {
            member: CharacterId(3),
        },
    );
}

#[test]
fn casters_owe_picks_and_choose_them_one_at_a_time() {
    let data = data();
    let mut world = inside_with(&data, "trainer", 3);
    world.party.gold = 6000;
    for member in &mut world.party.members {
        member.xp = 300;
    }
    refused(
        &mut world,
        &data,
        ServiceCommand::Choose {
            member: CharacterId(1),
            spell: spell("healing_word"),
        },
        Rejection::NoPicks {
            member: CharacterId(1),
        },
    );
    for member in 0..3 {
        ask(
            &mut world,
            &data,
            ServiceCommand::Train {
                member: CharacterId(member),
            },
        );
    }
    let picks: Vec<u8> = world.party.members.iter().map(|m| m.spell_picks).collect();
    assert_eq!(picks, [0, 1, 2], "fighter none, cleric one, wizard two");
    assert_eq!(world.party.members[1].spell_points_max, 5);

    // The cleric's list: guidance, light, sacred flame, bless, cure wounds, healing word, guiding
    // bolt, inflict wounds, spiritual weapon (2nd), prayer of healing (2nd).
    let choose = |name| ServiceCommand::Choose {
        member: CharacterId(1),
        spell: spell(name),
    };
    refused(
        &mut world,
        &data,
        choose("guidance"),
        Rejection::CantripNotLearned,
    );
    refused(
        &mut world,
        &data,
        choose("bless"),
        Rejection::AlreadyKnown {
            member: CharacterId(1),
        },
    );
    refused(
        &mut world,
        &data,
        choose("magic_missile"),
        Rejection::NoSuchSpell {
            spell: spell("magic_missile"),
        },
    );
    refused(
        &mut world,
        &data,
        choose("spiritual_weapon"),
        Rejection::SpellTooHigh { level: 2, max: 1 },
    );
    let clock = minutes(&world);
    let gold = world.party.gold;
    let known = world.party.members[1].known_spells.clone();
    let events = ask(&mut world, &data, choose("healing_word"));
    assert_eq!(learned(&events), Some(0), "a pick is free");
    let durin = &world.party.members[1];
    let healing_word = data.registry.spells.get("base:spell:healing_word").unwrap();
    assert_eq!(
        durin.known_spells[..known.len()],
        known,
        "known rows stay put"
    );
    assert_eq!(durin.known_spells.last(), Some(&healing_word));
    assert_eq!(durin.spell_picks, 0);
    assert_eq!(
        (minutes(&world), world.party.gold),
        (clock, gold),
        "no time, no money"
    );
    refused(
        &mut world,
        &data,
        choose("healing_word"),
        Rejection::NoPicks {
            member: CharacterId(1),
        },
    );

    // The wizard knows every first-level spell on its list and the second-level ones open at
    // level 3: the picks wait.
    let wizard = |name| ServiceCommand::Choose {
        member: CharacterId(2),
        spell: spell(name),
    };
    refused(
        &mut world,
        &data,
        wizard("magic_missile"),
        Rejection::AlreadyKnown {
            member: CharacterId(2),
        },
    );
    refused(
        &mut world,
        &data,
        wizard("shatter"),
        Rejection::SpellTooHigh { level: 2, max: 1 },
    );
    assert_eq!(world.party.members[2].spell_picks, 2);
}

#[test]
fn spells_are_bought_at_the_temple_and_the_guild() {
    let data = data();
    let mut world = inside_with(&data, "temple", 3);
    // The temple's spells: bless, cure wounds, healing word, guiding bolt, inflict wounds,
    // spiritual weapon (2nd), prayer of healing (2nd).
    let learn = |slot, name| ServiceCommand::Learn {
        member: CharacterId(slot),
        spell: spell(name),
    };
    world.party.members[1].spell_picks = 1;
    world.party.gold = 4999;
    refused(
        &mut world,
        &data,
        learn(1, "healing_word"),
        Rejection::CannotAfford {
            cost: 5000,
            gold: 4999,
        },
    );
    world.party.gold = 6000;
    refused(
        &mut world,
        &data,
        learn(0, "healing_word"),
        Rejection::NotOnList {
            member: CharacterId(0),
        },
    );
    refused(
        &mut world,
        &data,
        learn(1, "bless"),
        Rejection::AlreadyKnown {
            member: CharacterId(1),
        },
    );
    refused(
        &mut world,
        &data,
        learn(1, "magic_missile"),
        Rejection::NoSuchSpell {
            spell: spell("magic_missile"),
        },
    );
    refused(
        &mut world,
        &data,
        learn(1, "spiritual_weapon"),
        Rejection::SpellTooHigh { level: 2, max: 1 },
    );
    let clock = minutes(&world);
    let events = ask(&mut world, &data, learn(1, "healing_word"));
    assert_eq!(
        learned(&events),
        Some(5000),
        "50 gp for a first-level spell"
    );
    assert_eq!(world.party.gold, 1000);
    assert_eq!(minutes(&world), clock + 10, "service_minutes");
    assert_eq!(
        world.party.members[1].spell_picks, 1,
        "a purchase spends no pick"
    );

    // The guild's spells: burning hands, magic missile, shield, thunderwave, shatter, acid arrow.
    let mut world = inside_with(&data, "guild", 3);
    refused(
        &mut world,
        &data,
        learn(1, "magic_missile"),
        Rejection::NotOnList {
            member: CharacterId(1),
        },
    );
    refused(
        &mut world,
        &data,
        learn(2, "magic_missile"),
        Rejection::AlreadyKnown {
            member: CharacterId(2),
        },
    );

    // The price follows the spell's level: a second-level healing word, to a third-level cleric.
    let mut data = data;
    let healing_word = data.registry.spells.get("base:spell:healing_word").unwrap();
    data.spells.get_mut(&healing_word).unwrap().level = 2;
    let mut world = inside_with(&data, "temple", 3);
    world.party.gold = 20_000;
    refused(
        &mut world,
        &data,
        learn(1, "healing_word"),
        Rejection::SpellTooHigh { level: 2, max: 1 },
    );
    world.party.members[1].level = 3;
    let events = ask(&mut world, &data, learn(1, "healing_word"));
    assert_eq!(learned(&events), Some(10_000), "50 gp a spell level");
}

#[test]
fn the_dead_neither_train_nor_learn() {
    let data = data();
    let mut world = inside_with(&data, "trainer", 3);
    let dead = omnis_rules::condition_id(&data, "dead").unwrap();
    let brenna = &mut world.party.members[0];
    brenna.xp = 300;
    brenna.hp = 0;
    brenna.conditions.push(dead);
    world.party.members[1].spell_picks = 1;
    world.party.members[1].hp = 0;
    world.party.members[1].conditions.push(dead);
    refused(
        &mut world,
        &data,
        TRAIN_BRENNA,
        Rejection::MemberDead {
            member: CharacterId(0),
        },
    );
    refused(
        &mut world,
        &data,
        ServiceCommand::Choose {
            member: CharacterId(1),
            spell: spell("healing_word"),
        },
        Rejection::MemberDead {
            member: CharacterId(1),
        },
    );
}

#[test]
fn the_words_name_the_new_commands() {
    let data = data();
    let mut world = inside_with(&data, "trainer", 3);
    world.party.members[1].spell_picks = 1;
    // `choose-M-R` counts row R of member M's class list; `learn-M-R` row R of the service's
    // spells, which a trainer has none of.
    let script = common::script(&world, &data, "train-1\nchoose-1-5");
    assert_eq!(
        script,
        [
            Command::Service(ServiceCommand::Train {
                member: CharacterId(1)
            }),
            Command::Service(ServiceCommand::Choose {
                member: CharacterId(1),
                spell: spell("healing_word"),
            }),
        ]
    );
    let resolve = |world: &World, text: &str| {
        omnis_sim::Word::parse(text).and_then(|w| w.command(world, &data))
    };
    assert_eq!(
        resolve(&world, "learn-2-0"),
        None,
        "a trainer sells no spells"
    );
    assert_eq!(
        resolve(&world, "choose-1-10"),
        None,
        "the list has ten rows"
    );
    let guild = inside_with(&data, "guild", 3);
    assert_eq!(
        resolve(&guild, "learn-2-0"),
        Some(Command::Service(ServiceCommand::Learn {
            member: CharacterId(2),
            spell: spell("burning_hands"),
        }))
    );
    let words: Vec<&str> = script.iter().map(Command::word).collect();
    assert_eq!(words, ["train", "choose"]);
}

#[test]
fn the_views_count_the_dungeon_s_groups_and_say_who_may_train() {
    use omnis_sim::ops::{party_view, status};
    let data = data();
    let mut world = common::world(&data);
    common::party_of(&mut world, &data, 2);
    let (_, events) = common::play(&mut world, &data, &common::walk_to_the_rats());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::CombatEnded { .. })),
        "the rats are fought"
    );
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    assert_eq!(world.position.map, dungeon);
    // Ten groups placed once: the hostile and the friendly rats and the eight goblin and
    // skeleton groups (the wary rat comes back, so it is not counted).
    assert_eq!(
        status(&world, &data).unwrap().groups_cleared,
        (1, 10),
        "the rats at (3, 8)"
    );
    assert_eq!(omnis_sim::groups_cleared(&world, &data, dungeon), (1, 10));

    world.party.members[0].xp = 300;
    world.party.members[1].spell_picks = 2;
    let party = party_view(&world, &data);
    let flags: Vec<(bool, u8)> = party
        .members
        .iter()
        .map(|m| (m.ready, m.spell_picks))
        .collect();
    assert_eq!(flags, [(true, 0), (false, 2)]);
}
