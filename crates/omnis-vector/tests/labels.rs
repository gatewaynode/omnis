//! Words for what the engine names (presentation-ARCHITECTURE §4.3): money in the right coin,
//! names from the packs, and no text key or debug form on screen.

use crate::common;

use omnis_sim::{CombatOutcome, Event, ItemPlace, MessageKey, SaveRule};
use omnis_vector::labels;
use omnis_vector::rolllog::{self, Names};
use omnis_vector::shell::text;

fn bribed() -> Event {
    Event::Bribed { cost: 5000 }
}

fn victory() -> Event {
    Event::CombatEnded {
        outcome: CombatOutcome::Victory,
        xp: 40,
        gold: 1234,
        fallen: Vec::new(),
        loot: Vec::new(),
    }
}

/// A bribe's cost and the victory's loot are copper on the wire (docs/api.md §9); the lines
/// name whole gold, as the retired client did. They printed the copper as gold (bug, P2a).
#[test]
fn the_hud_names_a_bribe_and_the_loot_in_gold_not_copper() {
    let data = common::data();
    assert_eq!(
        text::describe(&bribed(), &Names::default(), &data).as_deref(),
        Some("Paid 50 gold; the monsters leave")
    );
    assert_eq!(
        text::describe(&victory(), &Names::default(), &data).as_deref(),
        Some("Victory! 40 XP each, 12 gold")
    );
}

/// The roll log had the same copper-as-gold lines (bug, P2a).
#[test]
fn the_roll_log_names_a_bribe_and_the_loot_in_gold_not_copper() {
    let data = common::data();
    let names = Names::default();
    assert_eq!(
        rolllog::describe(&bribed(), &names, &data).as_deref(),
        Some("Paid 50 gold; the monsters leave")
    );
    assert_eq!(
        rolllog::describe(&victory(), &names, &data).as_deref(),
        Some("Victory! 40 XP each, 12 gold")
    );
}

/// A message reads as words, never its key or its Rust name, though no pack defines the key.
#[test]
fn a_message_reads_as_words() {
    let data = common::data();
    let line = text::describe(
        &Event::Message {
            key: MessageKey::NothingHere,
        },
        &Names::default(),
        &data,
    )
    .expect("a message is shown");
    assert_eq!(line, "There is nothing to do here");
}

#[test]
fn names_come_from_the_packs_and_an_unknown_id_reads_as_itself() {
    let data = common::data();
    assert_eq!(labels::item(&data, "base:item:mace"), "Mace");
    assert_eq!(labels::spell(&data, "base:spell:bless"), "Bless");
    assert_eq!(
        labels::monster(&data, "base:monster:giant_rat"),
        "Giant Rat"
    );
    assert_ne!(labels::map(&data, "test:map:town"), "test:map:town");
    for id in [
        "mod:item:nothing",
        "mod:spell:nothing",
        "mod:monster:nothing",
    ] {
        assert_eq!(labels::item(&data, id), id);
        assert_eq!(labels::spell(&data, id), id);
        assert_eq!(labels::monster(&data, id), id);
        assert_eq!(labels::condition(&data, id), id);
        assert_eq!(labels::service(&data, id), id);
        assert_eq!(labels::map(&data, id), id);
    }
}

#[test]
fn money_reads_by_coin_or_as_whole_gold() {
    assert_eq!(labels::coins(0), "0 gp 0 sp 0 cp");
    assert_eq!(labels::coins(1), "0 gp 0 sp 1 cp");
    assert_eq!(labels::coins(1537), "15 gp 3 sp 7 cp");
    assert_eq!(labels::gold(99), "0 gold");
    assert_eq!(labels::gold(1234), "12 gold");
    assert_eq!(
        labels::save_rule(SaveRule::Relief),
        "Inns, items, and spells"
    );
}

#[test]
fn skills_and_alignments_read_as_words_and_creation_refusals_as_the_screen_says_them() {
    use omnis_sim::Rejection;
    use omnis_sim::omnis_data::{Alignment, Skill};
    use omnis_sim::omnis_rules::CreationError;
    assert_eq!(labels::skill(Skill::SleightOfHand), "Sleight of Hand");
    assert_eq!(labels::skill(Skill::AnimalHandling), "Animal Handling");
    assert_eq!(labels::alignment(Alignment::LawfulGood), "Lawful Good");
    let refusal = |r: Rejection, picks| labels::creation_refusal(&r, picks, 2);
    assert_eq!(refusal(Rejection::PartyFull, 2), "The party is full");
    assert_eq!(
        refusal(Rejection::Character(CreationError::Name), 2),
        "Give the character a name"
    );
    assert_eq!(
        refusal(
            Rejection::Character(CreationError::Points {
                spent: 28,
                budget: 27
            }),
            2
        ),
        "28 points spent of 27"
    );
    let skills = || Rejection::Character(CreationError::Skills("x".into()));
    assert_eq!(refusal(skills(), 1), "Pick 2 skills");
    // The right count but another skill fault: the engine's own words, never a wrong count.
    assert_eq!(refusal(skills(), 2), skills().to_string());
}

/// The sheet's words (parity P6): race and background names from the packs, abilities, signed
/// bonuses, and what a spell spends of the turn.
#[test]
fn the_sheet_s_words_come_from_the_packs_and_read_as_the_other_client_s() {
    use omnis_sim::omnis_data::{Ability, Cost};
    let data = common::data();
    assert_eq!(labels::race(&data, "base:race:dwarf"), "Dwarf (hill dwarf)");
    assert_eq!(
        labels::background(&data, "base:background:acolyte"),
        "Acolyte"
    );
    assert_eq!(labels::race(&data, "mod:race:nothing"), "mod:race:nothing");
    assert_eq!(
        labels::background(&data, "mod:background:x"),
        "mod:background:x"
    );
    assert_eq!(labels::ability(Ability::Wisdom), "Wisdom");
    assert_eq!(
        [3, -1, 0].map(labels::signed),
        ["+3".to_owned(), "-1".to_owned(), "+0".to_owned()]
    );
    assert_eq!(
        [Cost::Action, Cost::BonusAction, Cost::Reaction, Cost::Free].map(labels::pay),
        ["action", "bonus action", "reaction", "free"]
    );
}

/// A rest's and a tactic's refusals in a few words, members by name (parity P6).
#[test]
fn rest_and_tactics_refusals_read_short_with_names() {
    use omnis_sim::Rejection;
    use omnis_sim::omnis_rules::TacticsFault;
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let party = common::views(&world, &data).party;
    let durin = party.members[1].member;
    let cases = [
        (
            Rejection::RestTooSoon { minutes: 605 },
            "rested too recently (10h 05m)",
        ),
        (
            Rejection::NoFood { need: 4, have: 1 },
            "the rest eats 4 food; the stores hold 1",
        ),
        (
            Rejection::NothingToTreat { member: durin },
            "Durin is unhurt",
        ),
        (
            Rejection::NoHitDice {
                member: durin,
                left: 0,
            },
            "Durin has no hit dice left",
        ),
        (Rejection::CannotReact, "cannot answer that"),
        (Rejection::NoSuchEntry { entry: 3 }, "no such entry"),
        (
            Rejection::Tactics(TacticsFault::Name),
            "the name does not fit",
        ),
        (
            Rejection::Tactics(TacticsFault::TooDeep),
            "too many conditions",
        ),
        (
            Rejection::Tactics(TacticsFault::TooMany),
            "no room for another",
        ),
        (
            Rejection::Tactics(TacticsFault::Percent),
            "a percent over 100",
        ),
        (
            Rejection::UnknownId {
                id: "mod:monster:x".to_owned(),
            },
            "not loaded: mod:monster:x",
        ),
    ];
    for (refusal, words) in cases {
        assert_eq!(labels::reason(&party, &refusal), words, "{refusal:?}");
    }
}

/// The rest's and the tactics' events read as the retired client's lines (`service_text.rs`,
/// `spell_text.rs`; parity P6).
#[test]
fn rest_and_tactics_events_read_as_the_other_client_s_lines() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let mut names = Names::default();
    names.observe(&common::views(&world, &data).party);
    let durin = common::id(&world, 1);
    let cases = [
        (
            Event::Rested {
                long: true,
                minutes: 480,
                food: 4,
            },
            "The party rests for the night and eats 4 food",
        ),
        (
            Event::Rested {
                long: false,
                minutes: 60,
                food: 0,
            },
            "The party rests for an hour",
        ),
        (
            Event::Rested {
                long: false,
                minutes: 120,
                food: 0,
            },
            "The party rests for 2 hours",
        ),
        (
            Event::Rested {
                long: false,
                minutes: 40,
                food: 0,
            },
            "The party rests for 40 minutes",
        ),
        (
            Event::HitDiceSpent {
                member: durin,
                dice: 1,
            },
            "Durin spends 1 hit die",
        ),
        (
            Event::HitDiceSpent {
                member: durin,
                dice: 2,
            },
            "Durin spends 2 hit dice",
        ),
        (
            Event::RestInterrupted { minutes: 60 },
            "Ambushed after an hour!",
        ),
        (Event::Folded { interactions: 12 }, "The world has moved on"),
        (
            Event::ReactionsSwitched {
                member: durin,
                on: false,
            },
            "Durin's reactions are off",
        ),
        (
            Event::ReactionsSwitched {
                member: durin,
                on: true,
            },
            "Durin's reactions are on",
        ),
        (
            Event::TacticsChanged { member: durin },
            "Durin's tactics are set",
        ),
    ];
    for (event, line) in cases {
        assert_eq!(
            text::describe(&event, &names, &data).as_deref(),
            Some(line),
            "{event:?}"
        );
    }
}

/// A debugging edit leaves a line, as the retired client's `debug_menu::describe` did, with the
/// member by name and the item, condition and map by their labels (parity P8). Red before P8c:
/// no line was shown.
#[test]
fn dev_edits_read_as_lines_with_names() {
    use omnis_sim::DevCommand;
    use omnis_sim::omnis_core::Facing;
    use omnis_sim::omnis_data::Ability;
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let mut names = Names::default();
    names.observe(&common::views(&world, &data).party);
    let durin = common::id(&world, 1);
    let cases = [
        (
            DevCommand::GiveItem {
                member: Some(durin),
                item: "base:item:spyglass".to_owned(),
                count: 3,
            },
            "dev: Spyglass x3 to Durin",
        ),
        (
            DevCommand::GiveItem {
                member: None,
                item: "base:item:spyglass".to_owned(),
                count: 1,
            },
            "dev: Spyglass to the stores",
        ),
        (
            DevCommand::SetHp {
                member: durin,
                hp: 40,
            },
            "dev: Durin hp 40",
        ),
        (
            DevCommand::SetSpellPoints {
                member: durin,
                points: 3,
            },
            "dev: Durin sp 3",
        ),
        (
            DevCommand::SetXp {
                member: durin,
                xp: 300,
            },
            "dev: Durin xp 300",
        ),
        (
            DevCommand::SetScore {
                member: durin,
                ability: Ability::Strength,
                score: 18,
            },
            "dev: Durin STR 18",
        ),
        (
            DevCommand::SetGold { gold: 150 },
            "dev: gold 1 gp 5 sp 0 cp",
        ),
        (DevCommand::SetFood { food: 40 }, "dev: food 40"),
        (
            DevCommand::SetFlag {
                flag: "base:flag:seen".to_owned(),
                value: 2,
            },
            "dev: base:flag:seen = 2",
        ),
        (
            DevCommand::Teleport {
                map: "test:map:town".to_owned(),
                level: omnis_sim::omnis_core::Level::GROUND,
                x: 3,
                y: 4,
                facing: Facing::North,
            },
            "dev: teleport to Test Town (3, 4) facing north",
        ),
        (
            DevCommand::SetMonsterHp {
                stack: 0,
                index: 1,
                hp: 4,
            },
            "dev: stack 1 #2 hp 4",
        ),
        (DevCommand::KillStack { stack: 1 }, "dev: stack 2 slain"),
        (DevCommand::Fold, "dev: fold"),
    ];
    for (command, line) in cases {
        assert_eq!(
            text::describe(
                &Event::Dev {
                    command: command.clone()
                },
                &names,
                &data
            )
            .as_deref(),
            Some(line),
            "{command:?}"
        );
    }
}

/// Items laid down, picked up, and moved in and out of a container (M8.5 step 2a, item 11).
#[test]
fn the_hud_names_items_on_the_ground_and_in_a_container() {
    let data = common::data();
    let names = Names::default();
    let member = omnis_sim::omnis_core::CharacterId(0);
    let who = names.member(member);
    let dagger = labels::item(&data, "base:item:dagger");
    let chest = || ItemPlace::Object("chest_1".into());
    let lines = [
        (
            ItemPlace::Member(member),
            ItemPlace::Ground,
            format!("{who} drops {dagger}"),
        ),
        (
            ItemPlace::Ground,
            ItemPlace::Member(member),
            format!("{who} picks up {dagger}"),
        ),
        (
            ItemPlace::Member(member),
            chest(),
            format!("{who} puts {dagger} in"),
        ),
        (
            chest(),
            ItemPlace::Member(member),
            format!("{who} takes {dagger} out"),
        ),
    ];
    for (from, to, line) in lines {
        let moved = Event::ItemMoved {
            item: "base:item:dagger".into(),
            count: 1,
            from,
            to,
        };
        assert_eq!(text::describe(&moved, &names, &data), Some(line));
    }
}

/// What the dead dropped, on the death and the victory lines (M8.5 step 2a, item 12).
#[test]
fn the_lines_name_the_loot() {
    let data = common::data();
    let names = Names::default();
    let loot = vec![
        omnis_sim::Loot {
            item: "base:item:gem".into(),
            count: 2,
        },
        omnis_sim::Loot {
            item: "base:item:dagger".into(),
            count: 1,
        },
    ];
    let (gem, dagger) = (
        labels::item(&data, "base:item:gem"),
        labels::item(&data, "base:item:dagger"),
    );
    let won = Event::CombatEnded {
        outcome: CombatOutcome::Victory,
        xp: 40,
        gold: 1234,
        fallen: Vec::new(),
        loot: loot.clone(),
    };
    assert_eq!(
        text::describe(&won, &names, &data),
        Some(format!(
            "Victory! 40 XP each, 12 gold; on the ground: 2 {gem}, {dagger}"
        ))
    );
    let rat = omnis_sim::ActorRef::Monster { stack: 0, index: 0 };
    let died = Event::Death {
        target: rat,
        gold: None,
        loot,
    };
    assert_eq!(
        rolllog::describe(&died, &names, &data),
        Some(format!(
            "{} dies, dropping 2 {gem}, {dagger}",
            names.actor(rat)
        ))
    );
}

#[test]
fn the_clock_reads_days_hours_minutes() {
    assert_eq!(labels::clock(0), "Day 1 00:00");
    assert_eq!(labels::clock(1440 + 61), "Day 2 01:01");
}
