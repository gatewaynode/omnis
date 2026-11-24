//! The roll log, on a real fight (presentation-ARCHITECTURE.md §9): meeting the dungeon's placed group
//! starts a fresh log, and every line of the fight, the last command's included, names who
//! acted, though that command ends the fight and the stacks leave the world with it.

use crate::common;

use common::app::met_with;
use omnis_sim::omnis_data::Disposition;
use omnis_sim::{
    CombatCommand, CombatOutcome, Command, EncounterChoice, EncounterSource, Event, Mode,
    combat_view,
};
use omnis_vector::shell::session::Session;

/// Fight the placed group to the end: each member attacks the first stack they reach, or
/// dodges when they reach none.
fn fought(seed: u64) -> Session {
    let mut app = met_with(seed);
    let mut session = app
        .world_mut()
        .remove_resource::<Session>()
        .expect("a session");
    session.order(Command::Encounter(EncounterChoice::Attack));
    for _ in 0..300 {
        let Some(view) = combat_view(&session.world, &session.data) else {
            break;
        };
        let command = view
            .stacks
            .iter()
            .find(|s| s.alive && s.reachable)
            .map_or(CombatCommand::Dodge, |s| CombatCommand::Attack {
                stack: s.stack,
            });
        session.order(Command::Combat(command));
    }
    assert!(
        matches!(session.world.mode, Mode::Explore),
        "the fight ended"
    );
    assert_eq!(session.binder.refusals, 0);
    session
}

fn monster_name(session: &Session) -> String {
    let Some(placed) = session.data.monsters.values().find(|m| {
        session
            .fight_log
            .first()
            .is_some_and(|l| l.contains(session.data.label("en", &m.name)))
    }) else {
        panic!("the first line names a monster: {:?}", session.fight_log);
    };
    session.data.label("en", &placed.name).to_owned()
}

#[test]
fn a_fight_is_logged_from_the_meeting_to_the_ending() {
    let session = fought(1);
    let log = &session.fight_log;
    assert!(log[0].starts_with("Met "), "{log:?}");
    assert!(log.iter().any(|l| l.starts_with("Initiative: ")), "{log:?}");
    assert!(log.iter().any(|l| l == "Round 1"), "{log:?}");
    assert!(
        log.iter()
            .any(|l| l.contains(" hits ") || l.contains(" misses ")),
        "{log:?}"
    );
    let last = log.last().expect("lines");
    let ending = ["Victory", "The party got away", "The party has fallen"];
    assert!(ending.iter().any(|e| last.starts_with(e)), "{log:?}");
    assert!(log.len() <= 40);
}

#[test]
fn the_last_commands_lines_still_name_everyone() {
    let session = fought(1);
    let log = &session.fight_log;
    // The fallbacks for a name the log no longer knows.
    for unknown in ["someone", "a monster", "?"] {
        assert!(
            !log.iter().any(|l| l.contains(unknown)),
            "{unknown:?} in {log:?}"
        );
    }
    let name = monster_name(&session);
    if log.last().is_some_and(|l| l.starts_with("Victory")) {
        // The killing blow came in the command that ended the fight.
        let mut deaths = log
            .iter()
            .filter(|l| l.ends_with("dies") || l.contains(" dies,"));
        assert!(
            deaths.next_back().is_some_and(|l| l.starts_with(&name)),
            "{log:?}"
        );
    }
}

#[test]
fn each_monster_keeps_the_number_it_was_met_with() {
    // Seed 1 meets two giant rats and kills the first before the second acts: the simulation
    // then calls the second "index 0", and the log must still call it the second.
    let session = fought(1);
    let log = &session.fight_log;
    let name = monster_name(&session);
    let deaths: Vec<&String> = log
        .iter()
        .filter(|l| l.starts_with(&name) && (l.ends_with(" dies") || l.contains(" dies,")))
        .collect();
    assert_eq!(deaths.len(), 2, "{log:?}");
    assert_ne!(deaths[0], deaths[1], "two different rats died: {log:?}");
    let first = log.iter().position(|l| l == deaths[0]).expect("a line");
    assert!(
        log[first + 1..]
            .iter()
            .all(|l| !l.starts_with(&format!("{name} 1 "))),
        "the first rat is not heard from after its death: {log:?}"
    );
}

#[test]
fn a_member_the_fight_removed_is_still_named() {
    let mut session = fought(1);
    let gone = session.world.party.members.remove(0);
    session.note(&[Event::CombatEnded {
        outcome: CombatOutcome::Defeat,
        xp: 0,
        gold: 0,
        fallen: vec![gone.id],
        loot: Vec::new(),
    }]);
    let last = session.fight_log.last().expect("a line");
    assert_eq!(last, &format!("The party has fallen. Lost: {}", gone.name));
}

#[test]
fn meeting_monsters_starts_a_fresh_log() {
    let mut session = fought(1);
    assert!(!session.fight_log.is_empty());
    let monster = "test:monster:giant_rat".to_owned();
    session.note(&[Event::EncounterStarted {
        source: EncounterSource::Random,
        stacks: vec![(monster, 2)],
        disposition: Disposition::Wary,
        counts: Vec::new(),
        stealth: None,
        perception: 10,
        noticed: true,
    }]);
    assert_eq!(session.fight_log.len(), 1, "{:?}", session.fight_log);
    assert!(session.fight_log[0].starts_with("Met 2 "));
    assert!(session.fight_log[0].ends_with("(wary)"));
}

/// The fight's log says who closes and how far, and when the party's line advances.
#[test]
fn the_log_tells_the_moves_in_feet() {
    use omnis_sim::Event;
    use omnis_vector::rolllog::{Names, describe};
    let data = crate::common::data();
    let mut names = Names::default();
    let met = Event::EncounterStarted {
        source: omnis_sim::EncounterSource::Random,
        stacks: vec![("base:monster:giant_rat".into(), 2)],
        disposition: omnis_sim::omnis_data::Disposition::Hostile,
        counts: Vec::new(),
        stealth: None,
        perception: 10,
        noticed: true,
    };
    names.follow(&met, &data);
    let line = |event: Event| describe(&event, &names, &data).expect("a line");
    let moved = |to: u16| Event::StackMoved {
        stack: 0,
        from: 60,
        to,
    };
    assert_eq!(line(moved(30)), "Giant Rat closes to 30 ft");
    assert_eq!(line(moved(5)), "Giant Rat closes with the party");
    assert_eq!(
        line(Event::PartyAdvanced { feet: 10 }),
        "The party advances 10 ft"
    );
}

/// A spell that pushes (owner, 2026-11-16): the log says who was thrown back, and those thrown
/// out of a stack keep the numbers they were met with in the stack they become.
#[test]
fn the_log_tells_who_is_thrown_back_and_keeps_their_numbers() {
    use omnis_sim::{ActorRef, Event};
    use omnis_vector::rolllog::{Names, describe};
    let data = crate::common::data();
    let mut names = Names::default();
    let met = Event::EncounterStarted {
        source: omnis_sim::EncounterSource::Random,
        stacks: vec![("base:monster:giant_rat".into(), 4)],
        disposition: omnis_sim::omnis_data::Disposition::Hostile,
        counts: Vec::new(),
        stealth: None,
        perception: 10,
        noticed: true,
    };
    names.follow(&met, &data);
    let whole = Event::StackMoved {
        stack: 0,
        from: 5,
        to: 15,
    };
    let line = |names: &Names, event: &Event| describe(event, names, &data).expect("a line");
    assert_eq!(line(&names, &whole), "Giant Rat is thrown back to 15 ft");
    assert_eq!(
        line(&names, &Event::PartyDrivenBack { feet: 10 }),
        "The party is thrown back 10 ft"
    );
    // One who dies of it is told as thrown before the blow that kills it.
    let body = Event::Thrown {
        target: ActorRef::Monster { stack: 0, index: 1 },
        feet: 50,
    };
    assert_eq!(line(&names, &body), "Giant Rat 2 is thrown back 50 ft");
    // The second and the fourth rat are thrown out of the stack.
    let split = |moved: Vec<u8>| Event::StackSplit {
        stack: 0,
        into: 1,
        moved,
        from: 5,
        to: 15,
    };
    let one = split(vec![2]);
    assert_eq!(line(&names, &one), "Giant Rat 3 is thrown back to 15 ft");
    let two = split(vec![1, 3]);
    assert_eq!(
        line(&names, &two),
        "Giant Rat 2 and Giant Rat 4 are thrown back to 15 ft"
    );
    names.follow(&two, &data);
    let rat = |stack: u8, index: u8| names.actor(ActorRef::Monster { stack, index });
    assert_eq!(
        [rat(0, 0), rat(0, 1), rat(1, 0), rat(1, 1)],
        ["Giant Rat 1", "Giant Rat 3", "Giant Rat 2", "Giant Rat 4"]
    );
    assert_eq!(names.actor(ActorRef::Stack(1)), "Giant Rat");
    let closes = Event::StackMoved {
        stack: 1,
        from: 15,
        to: 5,
    };
    assert_eq!(line(&names, &closes), "Giant Rat closes with the party");
}
