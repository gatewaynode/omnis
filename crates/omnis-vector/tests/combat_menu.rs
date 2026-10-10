//! The fight's choices on a real fight (presentation-ARCHITECTURE.md §9): the dungeon's placed group,
//! met by walking into it. Every command the menu offers is one the simulation accepts, the
//! spell, item and swap steps reach their targets, clicks choose only offered targets, and a
//! fight runs to its end from the menu alone.

use crate::common;

use common::fight::{acting, met, turn_of};
use common::id;
use omnis_sim::omnis_data::Data;
use omnis_sim::{CombatCommand, Command, Event, Mode, Pay, Target, World, apply, combat_view};
use omnis_vector::combat_menu::{Act, Action, CombatMenu, Entry, Pick, Step};

fn labels(entries: &[Entry]) -> Vec<&str> {
    entries.iter().map(|e| e.label.as_str()).collect()
}

fn entry<'a>(entries: &'a [Entry], start: &str) -> &'a Entry {
    entries
        .iter()
        .find(|e| e.label.starts_with(start))
        .unwrap_or_else(|| panic!("an entry {start:?} in {:?}", labels(entries)))
}

/// The events of `command` applied to a copy of the world, which must accept it.
fn events_of(world: &World, data: &Data, command: Command) -> Vec<Event> {
    apply(&mut world.clone(), data, command.clone())
        .unwrap_or_else(|r| panic!("{command:?} refused: {r}"))
}

#[test]
fn the_encounter_offers_its_four_choices() {
    let session = met();
    let menu = CombatMenu::default();
    let entries = menu.entries(&session.world, &session.data);
    let shown = labels(&entries);
    assert_eq!(shown[0], "Fight");
    assert!(
        shown[1].starts_with("Bribe ("),
        "the cost is shown: {shown:?}"
    );
    assert_eq!(&shown[2..], ["Hide", "Run"]);
    assert_eq!(
        menu.prompt(&session.world, &session.data),
        "Monsters ahead (Hostile)"
    );
}

/// Every step reachable from the turn's first, each with its entries.
fn walk(world: &World, data: &Data) -> Vec<(Step, Vec<Entry>)> {
    let mut seen = Vec::new();
    let mut queue = vec![Step::Top];
    while let Some(step) = queue.pop() {
        if seen.iter().any(|(s, _)| *s == step) {
            continue;
        }
        let entries = CombatMenu { step }.entries(world, data);
        for e in &entries {
            if let (Act::Open(next), None) = (&e.act, &e.blocked) {
                queue.push(*next);
            }
        }
        seen.push((step, entries));
    }
    seen
}

#[test]
fn every_command_offered_anywhere_on_a_casters_turn_is_accepted() {
    let session = turn_of("Durin");
    let steps = walk(&session.world, &session.data);
    let mut commands = 0;
    for (step, entries) in &steps {
        for e in entries {
            if let (Act::Command(command), None) = (&e.act, &e.blocked) {
                events_of(&session.world, &session.data, command.clone());
                commands += 1;
            }
            if step != &Step::Top {
                assert!(
                    entries.last().is_some_and(|l| l.act == Act::Back),
                    "{step:?} ends with Back"
                );
            }
        }
    }
    let reached: Vec<Step> = steps.iter().map(|(s, _)| *s).collect();
    for step in [
        Step::Spells,
        Step::Items,
        Step::Target(Action::Attack),
        Step::Target(Action::Swap),
    ] {
        assert!(reached.contains(&step), "{step:?} reached: {reached:?}");
    }
    assert!(
        reached
            .iter()
            .any(|s| matches!(s, Step::Target(Action::Cast(_)))),
        "a spell's targets reached"
    );
    assert!(commands >= 8, "{commands} commands offered");
}

#[test]
fn a_rogue_who_knows_no_spells_is_told_so() {
    let session = turn_of("Pip");
    let entries = CombatMenu::default().entries(&session.world, &session.data);
    assert_eq!(
        labels(&entries),
        ["Attack", "Cast", "Use", "Dodge", "Swap", "End turn", "Flee"]
    );
    assert_eq!(
        entry(&entries, "Cast").blocked.as_deref(),
        Some("knows no spells")
    );
    assert!(entry(&entries, "Attack").blocked.is_none());
}

#[test]
fn a_spell_at_a_stack_and_a_spell_on_a_member() {
    let session = turn_of("Durin");
    let (world, data) = (&session.world, &session.data);
    let mut menu = CombatMenu::default();
    assert_eq!(
        menu.prompt(world, data),
        "Round 1: Durin's turn (1 action, 1 bonus action)"
    );
    let top = menu.entries(world, data);
    assert_eq!(menu.choose(&entry(&top, "Cast").act), None);
    assert_eq!(menu.step, Step::Spells);
    let spells = menu.entries(world, data);
    // Sacred flame goes to a stack: only stacks are offered.
    menu.choose(&entry(&spells, "Sacred Flame").act);
    assert!(menu.prompt(world, data).ends_with("on whom?"));
    let at = menu.entries(world, data);
    let picks = menu.clickable(world, data);
    assert!(!picks.is_empty() && picks.iter().all(|p| matches!(p, Pick::Stack(_))));
    let first = &at[0];
    let Act::Command(command) = &first.act else {
        panic!("a target sends the cast")
    };
    assert!(matches!(
        command,
        Command::Combat(CombatCommand::Cast {
            target: Target::Stack(_),
            ..
        })
    ));
    let events = events_of(world, data, command.clone());
    assert!(events.iter().any(|e| matches!(e, Event::SpellCast { .. })));
    // Back to the list, and a healing spell goes to a member: only members are offered.
    menu.back();
    assert_eq!(menu.step, Step::Spells);
    menu.choose(&entry(&spells, "Cure Wounds").act);
    let picks = menu.clickable(world, data);
    assert_eq!(picks.len(), world.party.members.len());
    assert!(picks.iter().all(|p| matches!(p, Pick::Member(_))));
    let on = menu.entries(world, data);
    let brenna = entry(&on, "Brenna");
    let Act::Command(command) = brenna.act.clone() else {
        panic!("a member sends the cast")
    };
    assert_eq!(menu.choose(&brenna.act), Some(command.clone()));
    assert_eq!(menu.step, Step::Top, "a sent command resets the menu");
    let events = events_of(world, data, command);
    assert!(events.iter().any(|e| matches!(e, Event::Healed { .. })));
}

#[test]
fn a_spell_whose_target_does_not_matter_is_cast_at_once() {
    let session = turn_of("Durin");
    let spells = CombatMenu { step: Step::Spells }.entries(&session.world, &session.data);
    let light = entry(&spells, "Light");
    assert!(
        matches!(
            &light.act,
            Act::Command(Command::Combat(CombatCommand::Cast {
                target: Target::Member(durin),
                ..
            })) if *durin == id(&session.world, 1)
        ),
        "cast on Durin, slot 1, without asking: {light:?}"
    );
}

#[test]
fn swap_offers_the_others_and_a_potion_goes_to_a_member() {
    let session = turn_of("Durin");
    let (world, data) = (&session.world, &session.data);
    let mut menu = CombatMenu::default();
    menu.choose(&entry(&menu.entries(world, data), "Swap").act);
    let names = menu.entries(world, data);
    assert_eq!(labels(&names), ["Brenna", "Ilvara", "Pip", "Back"]);
    let Some(swap) = menu.choose(&names[0].act) else {
        panic!("a member sends the swap")
    };
    let events = events_of(world, data, swap);
    assert!(events.contains(&Event::Exchanged {
        member: id(world, 1),
        with: id(world, 0)
    }));
    // Use: the potion, then whom.
    menu.choose(&entry(&menu.entries(world, data), "Use").act);
    let items = menu.entries(world, data);
    menu.choose(&entry(&items, "Potion of healing").act);
    assert_eq!(
        menu.prompt(world, data),
        "Durin uses Potion of healing on whom?"
    );
    let Some(potion) = menu.choose(&entry(&menu.entries(world, data), "Brenna").act) else {
        panic!("a member sends the use")
    };
    let events = events_of(world, data, potion);
    assert!(events.iter().any(|e| matches!(e, Event::ItemUsed { .. })));
}

#[test]
fn use_is_blocked_with_nothing_usable() {
    let session = turn_of("Durin");
    let mut world = session.world.clone();
    let data = &session.data;
    let durin = world
        .party
        .members
        .iter_mut()
        .find(|m| m.name == "Durin")
        .expect("Durin");
    durin
        .equipment
        .retain(|(id, _)| data.items.get(id).is_none_or(|i| i.use_effect.is_none()));
    let entries = CombatMenu::default().entries(&world, data);
    assert_eq!(
        entry(&entries, "Use").blocked.as_deref(),
        Some("nothing to use now")
    );
}

#[test]
fn a_click_chooses_only_an_offered_target() {
    let session = turn_of("Durin");
    let (world, data) = (&session.world, &session.data);
    let mut menu = CombatMenu::default();
    // On the turn's first step a click on a stack in reach attacks it.
    assert_eq!(menu.pick(world, data, Pick::Member(id(world, 0))), None);
    let stack = menu.clickable(world, data)[0];
    let Some(attack) = menu.pick(world, data, stack) else {
        panic!("a stack in reach is clickable")
    };
    assert!(matches!(
        attack,
        Command::Combat(CombatCommand::Attack { .. })
    ));
    // While a swap waits for a member, a stack does nothing and a member swaps.
    menu.step = Step::Target(Action::Swap);
    assert_eq!(menu.pick(world, data, stack), None);
    assert_eq!(menu.step, Step::Target(Action::Swap));
    assert_eq!(
        menu.pick(world, data, Pick::Member(id(world, 0))),
        Some(Command::Combat(CombatCommand::Exchange {
            with: id(world, 0)
        }))
    );
    assert_eq!(menu.step, Step::Top);
    // Durin cannot swap with himself.
    menu.step = Step::Target(Action::Swap);
    assert_eq!(menu.pick(world, data, Pick::Member(id(world, 1))), None);
}

#[test]
fn a_fight_runs_to_its_end_from_the_menu_alone() {
    let mut session = met();
    let mut menu = CombatMenu::default();
    for _ in 0..1000 {
        if combat_view(&session.world, &session.data).is_none() {
            break;
        }
        let entries = menu.entries(&session.world, &session.data);
        let first = entries
            .iter()
            .find(|e| e.blocked.is_none())
            .expect("an entry can be chosen");
        if let Some(command) = menu.choose(&first.act) {
            session.order(command);
        }
    }
    assert!(
        matches!(session.world.mode, Mode::Explore),
        "the fight ended"
    );
    assert_eq!(
        session.binder.refusals, 0,
        "every command sent was accepted"
    );
    let last = session.fight_log.last().expect("a roll log");
    assert!(
        ["Victory", "The party got away", "The party has fallen"]
            .iter()
            .any(|e| last.starts_with(e)),
        "{last}"
    );
}

#[test]
fn end_turn_passes_the_turn_with_budget_left() {
    let mut session = turn_of("Durin");
    let menu = CombatMenu::default();
    let prompt = menu.prompt(&session.world, &session.data);
    assert!(
        prompt.ends_with("(1 action, 1 bonus action)"),
        "the budget is shown: {prompt}"
    );
    let entries = menu.entries(&session.world, &session.data);
    let end = entry(&entries, "End turn");
    assert_eq!(end.blocked, None);
    let Act::Command(command) = end.act.clone() else {
        panic!("End turn is a command: {end:?}")
    };
    session.order(command);
    assert_ne!(
        acting(&session).as_deref(),
        Some("Durin"),
        "the turn passed"
    );
}

#[test]
fn a_bonus_action_spell_is_paid_with_the_bonus_action_and_leaves_the_action() {
    let mut session = turn_of("Durin");
    // Durin learns Healing Word (the fixed party knows no bonus-action spell at level 1).
    let word = session
        .data
        .registry
        .spells
        .get("base:spell:healing_word")
        .expect("the base pack's Healing Word");
    session.world.party.members[1].known_spells.push(word);
    let mut menu = CombatMenu { step: Step::Spells };
    let spells = menu.entries(&session.world, &session.data);
    let word = entry(&spells, "Healing Word");
    assert!(word.label.contains("bonus"), "{word:?}");
    menu.choose(&word.act.clone());
    let targets = menu.entries(&session.world, &session.data);
    let Act::Command(cast) = entry(&targets, "Brenna").act.clone() else {
        panic!("a member target sends the cast")
    };
    assert!(
        matches!(
            &cast,
            Command::Combat(CombatCommand::Cast { spell, pay: Pay::BonusAction, target: Target::Member(m) })
                if spell == "base:spell:healing_word" && *m == id(&session.world, 0)
        ),
        "{cast:?}"
    );
    session.order(cast);
    assert_eq!(
        acting(&session).as_deref(),
        Some("Durin"),
        "still Durin's turn"
    );
    let view = combat_view(&session.world, &session.data).expect("the fight goes on");
    assert_eq!((view.budget.actions, view.budget.bonus_actions), (1, 0));
}

#[test]
fn a_member_is_picked_by_identity_wherever_they_stand() {
    let session = turn_of("Durin");
    let (mut world, data) = (session.world.clone(), &session.data);
    let (brenna, ilvara) = (id(&world, 0), id(&world, 2));
    world.party.members.swap(0, 2);
    let mut menu = CombatMenu {
        step: Step::Target(Action::Swap),
    };
    assert_eq!(
        menu.pick(&world, data, Pick::Member(brenna)),
        Some(Command::Combat(CombatCommand::Exchange { with: brenna })),
        "Brenna, now in Ilvara's slot, is still Brenna"
    );
    let names = CombatMenu {
        step: Step::Target(Action::Swap),
    }
    .entries(&world, data);
    assert_eq!(labels(&names), ["Ilvara", "Brenna", "Pip", "Back"]);
    assert_eq!(
        entry(&names, "Ilvara").act,
        Act::Command(Command::Combat(CombatCommand::Exchange { with: ilvara }))
    );
}
