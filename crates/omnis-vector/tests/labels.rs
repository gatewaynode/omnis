//! Words for what the engine names (presentation-ARCHITECTURE §4.3): money in the right coin,
//! names from the packs, and no text key or debug form on screen.

use crate::common;

use omnis_sim::{CombatOutcome, Event, MessageKey, SaveRule};
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
    }
}

/// A bribe's cost and the victory's loot are copper on the wire (docs/api.md §9); the lines
/// name whole gold, as the other client does. They printed the copper as gold (bug, P2a).
#[test]
fn the_hud_names_a_bribe_and_the_loot_in_gold_not_copper() {
    let data = common::data();
    assert_eq!(
        text::describe(&bribed(), &Names::default(), &data).as_deref(),
        Some("Paid 50 gold; the monsters leave")
    );
    assert_eq!(
        text::describe(&victory(), &Names::default(), &data).as_deref(),
        Some("Victory: 40 xp each, 12 gold")
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
        Some("Victory: 40 xp each, 12 gold")
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
