//! The trainer and the spell sellers (M7b step 10): a level for `trainer.cost`, the picks it
//! owes chosen one at a time for free, spells bought at the guild and the temple for
//! `spell.learn_cost`, and every refusal leaving the world (its dice streams included) as it
//! was. Party: Brenna (human fighter), Durin (dwarf cleric), Ilvara (elf wizard).

mod common;

use common::{data, inside_with};
use omnis_data::Data;
use omnis_sim::{Command, Event, Rejection, ServiceCommand, World, apply};

fn ask(world: &mut World, data: &Data, command: ServiceCommand) -> Vec<Event> {
    apply(world, data, Command::Service(command)).unwrap_or_else(|r| panic!("{command:?}: {r}"))
}

/// The command is refused with `why`, and nothing changed.
fn refused(world: &mut World, data: &Data, command: ServiceCommand, why: Rejection) {
    let before = world.clone();
    assert_eq!(
        apply(world, data, Command::Service(command)),
        Err(why),
        "{command:?}"
    );
    assert_eq!(*world, before, "{command:?} changed the world");
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

const TRAIN_BRENNA: ServiceCommand = ServiceCommand::Train { member: 0 };

#[test]
fn a_level_is_granted_at_the_trainer_for_its_price() {
    let data = data();
    let mut world = inside_with(&data, "trainer", 3);
    refused(
        &mut world,
        &data,
        TRAIN_BRENNA,
        Rejection::NotReady {
            index: 0,
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
            index: 0,
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
        Rejection::MaxLevel { index: 0 },
    );
    refused(
        &mut world,
        &data,
        ServiceCommand::Train { member: 3 },
        Rejection::NoSuchMember { index: 3 },
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
            member: 1,
            spell: 5,
        },
        Rejection::NoPicks { index: 1 },
    );
    for member in 0..3 {
        ask(&mut world, &data, ServiceCommand::Train { member });
    }
    let picks: Vec<u8> = world.party.members.iter().map(|m| m.spell_picks).collect();
    assert_eq!(picks, [0, 1, 2], "fighter none, cleric one, wizard two");
    assert_eq!(world.party.members[1].spell_points_max, 5);

    // The cleric's list: guidance, light, sacred flame, bless, cure wounds, healing word.
    let choose = |spell| ServiceCommand::Choose { member: 1, spell };
    refused(&mut world, &data, choose(0), Rejection::CantripNotLearned);
    refused(
        &mut world,
        &data,
        choose(3),
        Rejection::AlreadyKnown { index: 1 },
    );
    refused(
        &mut world,
        &data,
        choose(6),
        Rejection::NoSuchSpell { row: 6 },
    );
    let clock = minutes(&world);
    let gold = world.party.gold;
    let known = world.party.members[1].known_spells.clone();
    let events = ask(&mut world, &data, choose(5));
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
        choose(5),
        Rejection::NoPicks { index: 1 },
    );

    // The wizard knows every levelled spell on its list: the picks wait for a later level.
    refused(
        &mut world,
        &data,
        ServiceCommand::Choose {
            member: 2,
            spell: 3,
        },
        Rejection::AlreadyKnown { index: 2 },
    );
    assert_eq!(world.party.members[2].spell_picks, 2);
}

#[test]
fn spells_are_bought_at_the_temple_and_the_guild() {
    let data = data();
    let mut world = inside_with(&data, "temple", 3);
    // The temple's spells: bless, cure wounds, healing word.
    let learn = |member, spell| ServiceCommand::Learn { member, spell };
    world.party.members[1].spell_picks = 1;
    world.party.gold = 4999;
    refused(
        &mut world,
        &data,
        learn(1, 2),
        Rejection::CannotAfford {
            cost: 5000,
            gold: 4999,
        },
    );
    world.party.gold = 6000;
    refused(
        &mut world,
        &data,
        learn(0, 2),
        Rejection::NotOnList { index: 0 },
    );
    refused(
        &mut world,
        &data,
        learn(1, 0),
        Rejection::AlreadyKnown { index: 1 },
    );
    refused(
        &mut world,
        &data,
        learn(1, 3),
        Rejection::NoSuchSpell { row: 3 },
    );
    let clock = minutes(&world);
    let events = ask(&mut world, &data, learn(1, 2));
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

    // The guild's spells: burning hands, magic missile, shield.
    let mut world = inside_with(&data, "guild", 3);
    refused(
        &mut world,
        &data,
        learn(1, 1),
        Rejection::NotOnList { index: 1 },
    );
    refused(
        &mut world,
        &data,
        learn(2, 1),
        Rejection::AlreadyKnown { index: 2 },
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
        learn(1, 2),
        Rejection::SpellTooHigh { level: 2, max: 1 },
    );
    world.party.members[1].level = 3;
    let events = ask(&mut world, &data, learn(1, 2));
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
        Rejection::MemberDead { index: 0 },
    );
    refused(
        &mut world,
        &data,
        ServiceCommand::Choose {
            member: 1,
            spell: 5,
        },
        Rejection::MemberDead { index: 1 },
    );
}

#[test]
fn the_words_name_the_new_commands() {
    use omnis_sim::command::parse_script;
    let script = parse_script("train-1\nchoose-1-5\nlearn-2-0").unwrap();
    assert_eq!(
        script,
        [
            Command::Service(ServiceCommand::Train { member: 1 }),
            Command::Service(ServiceCommand::Choose {
                member: 1,
                spell: 5
            }),
            Command::Service(ServiceCommand::Learn {
                member: 2,
                spell: 0
            }),
        ]
    );
    let words: Vec<&str> = script.iter().map(Command::word).collect();
    assert_eq!(words, ["train", "choose", "learn"]);
}
