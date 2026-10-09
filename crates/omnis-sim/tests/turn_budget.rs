//! The turn budget (M7c step 3, PRD D21, ARCHITECTURE.md §4.7): several commands in a member's
//! turn within an action and a bonus action, the turn ending by itself when the action is spent
//! and no bonus-action spell is left, or by `EndTurn`, bonus-action spells (D24), Second Wind, Action Surge and Cunning
//! Action with their uses, and the monsters' opportunity attacks. Party: Brenna (human
//! fighter), Durin (dwarf cleric), Ilvara (elf wizard), Pip (halfling rogue); the first three
//! stand in front.

mod common;

use common::{act, data, encounter, party_of, script_for_six, world};
use omnis_core::CharacterId;
use omnis_data::{Cost, Data, Disposition};
use omnis_sim::omnis_rules::RollMode;
use omnis_sim::{
    ActorRef, Budget, CheckKind, CombatCommand, Command, Event, FeatureChoice, Mode, PartyCommand,
    Pay, Rejection, RestCommand, Settings, Surprise, Target, World, apply, combat,
};

const BRENNA: usize = 0;
const DURIN: usize = 1;
const ILVARA: usize = 2;
const PIP: usize = 3;

fn fight(world: &mut World, data: &Data, stacks: &[(&str, u8)]) {
    let here = world.position;
    let goblins = encounter(data, stacks, Disposition::Hostile, here);
    combat::start(world, data, goblins, Surprise::None, &mut Vec::new()).unwrap();
}

fn state(world: &World) -> &omnis_sim::CombatState {
    match &world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

fn current(world: &World) -> Option<ActorRef> {
    match &world.mode {
        Mode::Combat(state) => state.current_actor(),
        _ => None,
    }
}

fn member(world: &World, slot: usize) -> ActorRef {
    ActorRef::Member(world.party.members[slot].id)
}

/// Dodge whole turns until the member in `slot` acts.
fn until_turn_of(world: &mut World, data: &Data, slot: usize) {
    for _ in 0..200 {
        if current(world) == Some(member(world, slot)) {
            return;
        }
        act(world, data, Command::Combat(CombatCommand::Dodge)).unwrap();
    }
    panic!("the turn never came");
}

fn ask(world: &mut World, data: &Data, command: CombatCommand) -> Vec<Event> {
    apply(world, data, Command::Combat(command)).unwrap_or_else(|r| panic!("{command:?}: {r}"))
}

/// The command is refused with `why`, and nothing changed.
fn refused(world: &mut World, data: &Data, command: CombatCommand, why: Rejection) {
    let before = world.clone();
    assert_eq!(
        apply(world, data, Command::Combat(command)),
        Err(why),
        "{command:?}"
    );
    assert_eq!(*world, before, "{command:?} changed the world");
}

fn spell_row(world: &World, data: &Data, slot: usize, name: &str) -> u8 {
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

fn attacks_by(events: &[Event], who: ActorRef) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, Event::AttackResolved { attacker, .. } if *attacker == who))
        .count()
}

#[test]
fn a_wizard_s_turn_ends_with_its_action_and_a_cleric_keeps_the_bonus_for_healing_word() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let healing_word = data.registry.spells.get("base:spell:healing_word").unwrap();
    world.party.members[DURIN].known_spells.push(healing_word);
    fight(&mut world, &data, &[("goblin", 2), ("goblin", 2)]);

    until_turn_of(&mut world, &data, ILVARA);
    assert_eq!(
        state(&world).budget,
        Budget {
            actions: 1,
            bonus_actions: 1
        },
        "the SRD's one of each"
    );
    let ilvara = member(&world, ILVARA);
    ask(&mut world, &data, CombatCommand::Attack { stack: 0 });
    assert_ne!(
        current(&world),
        Some(ilvara),
        "nothing a wizard's bonus action pays for: the turn ends by itself"
    );

    until_turn_of(&mut world, &data, DURIN);
    let durin = member(&world, DURIN);
    ask(&mut world, &data, CombatCommand::Attack { stack: 0 });
    assert_eq!(
        current(&world),
        Some(durin),
        "healing word waits on the bonus action"
    );
    assert_eq!(
        state(&world).budget,
        Budget {
            actions: 0,
            bonus_actions: 1
        }
    );
    refused(
        &mut world,
        &data,
        CombatCommand::Attack { stack: 0 },
        Rejection::NoActionLeft,
    );
    let cure = spell_row(&world, &data, DURIN, "cure_wounds");
    let word = spell_row(&world, &data, DURIN, "healing_word");
    let brenna = world.party.members[BRENNA].id;
    let cast = |spell, pay| CombatCommand::Cast {
        spell,
        target: Target::Member(brenna),
        pay,
    };
    refused(
        &mut world,
        &data,
        cast(cure, Pay::Action),
        Rejection::NoActionLeft,
    );
    refused(
        &mut world,
        &data,
        cast(cure, Pay::BonusAction),
        Rejection::NotABonusAction { spell: cure },
    );
    let events = ask(&mut world, &data, cast(word, Pay::BonusAction));
    assert!(events.iter().any(|e| matches!(e, Event::Healed { .. })));
    assert_ne!(current(&world), Some(durin), "the budget is spent");
}

#[test]
fn two_spells_a_turn_within_the_budget_and_a_readied_spell_cannot_take_the_bonus() {
    // PRD D24: a bonus-action spell does not limit the action to a cantrip.
    let mut data = data();
    let healing_word = data.registry.spells.get("base:spell:healing_word").unwrap();
    let start = |data: &Data| {
        let mut world = common::world(data);
        party_of(&mut world, data, 2);
        world.party.members[DURIN].known_spells.push(healing_word);
        fight(&mut world, data, &[("goblin", 3)]);
        until_turn_of(&mut world, data, DURIN);
        world
    };
    let points = |world: &World| world.party.members[DURIN].spell_points;
    let mut world = start(&data);
    let word = spell_row(&world, &data, DURIN, "healing_word");
    let brenna = world.party.members[BRENNA].id;
    let cast = |spell, pay| CombatCommand::Cast {
        spell,
        target: Target::Member(brenna),
        pay,
    };
    let full = points(&world);
    ask(&mut world, &data, cast(word, Pay::BonusAction));
    let one = full - points(&world);
    assert!(one > 0, "a levelled spell costs points");
    assert_eq!(
        state(&world).budget,
        Budget {
            actions: 1,
            bonus_actions: 0
        },
        "the bonus action paid; the action is left"
    );
    refused(
        &mut world,
        &data,
        cast(word, Pay::BonusAction),
        Rejection::NoBonusActionLeft,
    );
    ask(&mut world, &data, cast(word, Pay::Action));
    assert_eq!(points(&world), full - 2 * one, "both spells paid");
    assert_ne!(
        current(&world),
        Some(member(&world, DURIN)),
        "the budget is spent and the turn passes"
    );

    // Either order: a levelled spell with the action leaves the bonus spell open.
    let mut world = start(&data);
    ask(&mut world, &data, cast(word, Pay::Action));
    assert_eq!(
        current(&world),
        Some(member(&world, DURIN)),
        "the bonus action still has a spell to pay for"
    );
    ask(&mut world, &data, cast(word, Pay::BonusAction));
    assert_eq!(points(&world), full - 2 * one);

    let spell = data.spells.get_mut(&healing_word).unwrap();
    spell.preparation_available = true;
    spell.preparation_required_for_bonus_action = true;
    let mut world = start(&data);
    refused(
        &mut world,
        &data,
        cast(word, Pay::BonusAction),
        Rejection::NeedsPreparation { spell: word },
    );
    ask(&mut world, &data, cast(word, Pay::Action));
    assert_ne!(
        current(&world),
        Some(member(&world, DURIN)),
        "a spell needing preparation leaves the bonus action nothing to pay for"
    );
}

#[test]
fn action_surge_gives_a_second_action_and_second_wind_heals_once_a_rest() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    world.party.members[BRENNA].level = 2;
    fight(&mut world, &data, &[("goblin", 3)]);
    until_turn_of(&mut world, &data, BRENNA);
    let brenna = member(&world, BRENNA);
    // Rows: Second Wind (level 1), Action Surge (level 2).
    let feature = |feature, choice| CombatCommand::Feature { feature, choice };
    refused(
        &mut world,
        &data,
        feature(2, FeatureChoice::None),
        Rejection::NoSuchFeature { feature: 2 },
    );
    refused(
        &mut world,
        &data,
        feature(0, FeatureChoice::Hide),
        Rejection::WrongChoice { feature: 0 },
    );

    // Features come before the action (owner, 2026-10-04): surge, attack, Second Wind, attack.
    let mut events = ask(&mut world, &data, feature(1, FeatureChoice::None));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::FeatureUsed { feature, .. } if feature == "base:text:class.fighter.action_surge"
    )));
    assert_eq!(
        state(&world).budget.actions,
        2,
        "Action Surge costs nothing and gives an action"
    );
    events.extend(ask(&mut world, &data, CombatCommand::Attack { stack: 0 }));
    assert_eq!(current(&world), Some(brenna), "an action is left");
    refused(
        &mut world,
        &data,
        feature(1, FeatureChoice::None),
        Rejection::NoUsesLeft { feature: 1 },
    );

    world.party.members[BRENNA].hp = 3;
    let healed = ask(&mut world, &data, feature(0, FeatureChoice::None));
    let Some(Event::Healed {
        rolls, amount, hp, ..
    }) = healed.iter().find(|e| matches!(e, Event::Healed { .. }))
    else {
        panic!("{healed:?}");
    };
    let die = i64::from(rolls[0].total);
    assert!((1..=10).contains(&die));
    assert_eq!(*amount, die + 2, "1d10 + the fighter's level");
    let max = i64::from(world.party.members[BRENNA].hp_max);
    assert_eq!(
        i64::from(*hp),
        (3 + die + 2).min(max),
        "capped at the maximum"
    );
    assert_eq!(current(&world), Some(brenna), "the second action is left");
    assert_eq!(
        world.party.members[BRENNA].feature_spent,
        [
            ("base:text:class.fighter.action_surge".to_owned(), 1),
            ("base:text:class.fighter.second_wind".to_owned(), 1)
        ]
    );
    if let Some(stack) = state(&world)
        .encounter
        .stacks
        .iter()
        .position(|s| s.alive())
    {
        let stack = u8::try_from(stack).unwrap();
        events.extend(ask(&mut world, &data, CombatCommand::Attack { stack }));
        assert_eq!(attacks_by(&events, brenna), 2, "two attacks in one turn");
    }
    assert_ne!(
        current(&world),
        Some(brenna),
        "no action, no bonus action: the turn ends by itself"
    );
}

#[test]
fn a_short_rest_gives_the_uses_back() {
    let data = data();
    for seed in 0..20u64 {
        let mut world = common::new_world(&data, seed, Settings::default());
        party_of(&mut world, &data, 1);
        world.party.members[BRENNA].feature_spent =
            vec![("base:text:class.fighter.second_wind".to_owned(), 1)];
        let events = apply(
            &mut world,
            &data,
            Command::Rest(RestCommand::Short { dice: Vec::new() }),
        )
        .unwrap();
        if events.iter().any(|e| matches!(e, Event::Rested { .. })) {
            assert!(world.party.members[BRENNA].feature_spent.is_empty());
            return;
        }
    }
    panic!("every rest was ambushed");
}

#[test]
fn cunning_action_exchanges_without_a_swing_and_a_plain_exchange_draws_them() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    let ids = world.party.ids();
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Reorder {
            order: vec![ids[3], ids[0], ids[1], ids[2]],
        }),
    )
    .unwrap();
    world.party.members[0].level = 2;
    fight(&mut world, &data, &[("goblin", 2), ("goblin", 2)]);
    let pip = world.party.members[0].id;
    until_turn_of(&mut world, &data, 0);
    let back = world.party.members[3].id;
    let events = ask(
        &mut world,
        &data,
        CombatCommand::Feature {
            feature: 0,
            choice: FeatureChoice::Exchange { with: back },
        },
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::OpportunityAttack { .. })),
        "Disengage: {events:?}"
    );
    assert_eq!(world.party.members[3].id, pip, "Pip went to the back");
    assert_eq!(
        state(&world).budget,
        Budget {
            actions: 1,
            bonus_actions: 0
        },
        "paid with the bonus action"
    );
    ask(&mut world, &data, CombatCommand::EndTurn);

    // The next front-row member to act exchanges with Pip at the back: a plain exchange out of
    // the front draws a swing from each front stack with its reaction.
    let leaver = loop {
        match current(&world) {
            Some(ActorRef::Member(id)) if world.party.members[..3].iter().any(|m| m.id == id) => {
                break id;
            }
            Some(_) => {
                act(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap();
            }
            None => panic!("the fight ended"),
        }
    };
    assert_eq!(world.party.members[3].id, pip, "Pip is at the back");
    let events = ask(&mut world, &data, CombatCommand::Exchange { with: pip });
    let swings: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            Event::OpportunityAttack { stack, member } if *member == leaver => Some(*stack),
            _ => None,
        })
        .collect();
    assert!(!swings.is_empty(), "{events:?}");
    // Each swing spent its stack's reaction; a stack whose own turn came later in the same
    // command has it back.
    let mut waiting = Vec::new();
    for stack in swings {
        let actor = ActorRef::Stack(stack);
        let turned = events
            .iter()
            .skip_while(|e| !matches!(e, Event::OpportunityAttack { stack: s, .. } if *s == stack))
            .any(|e| matches!(e, Event::Turn { actor: a } if *a == actor));
        if let Mode::Combat(state) = &world.mode {
            assert_eq!(state.reactions_left(actor), u8::from(turned), "{stack}");
        }
        if !turned {
            waiting.push(actor);
        }
    }
    for stack in waiting {
        for _ in 0..50 {
            if !matches!(world.mode, Mode::Combat(_)) {
                return;
            }
            let events = act(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap();
            if events
                .iter()
                .any(|e| matches!(e, Event::Turn { actor } if *actor == stack))
            {
                if let Mode::Combat(state) = &world.mode {
                    assert_eq!(state.reactions_left(stack), 1, "back on its own turn");
                }
                break;
            }
        }
    }
}

#[test]
fn hiding_gives_the_next_attack_advantage() {
    let data = data();
    let (mut hid, mut seen) = (false, false);
    for seed in 0..40u64 {
        let mut world = common::new_world(&data, seed, Settings::default());
        party_of(&mut world, &data, 4);
        world.party.members[PIP].level = 2;
        fight(&mut world, &data, &[("goblin", 1)]);
        until_turn_of(&mut world, &data, PIP);
        let pip = world.party.members[PIP].id;
        let events = ask(
            &mut world,
            &data,
            CombatCommand::Feature {
                feature: 0,
                choice: FeatureChoice::Hide,
            },
        );
        let Some(Event::Check {
            kind, dc, success, ..
        }) = events.iter().find(|e| matches!(e, Event::Check { .. }))
        else {
            panic!("{events:?}");
        };
        assert_eq!(
            (*kind, *dc),
            (CheckKind::Hide, 9),
            "a goblin's passive Perception"
        );
        assert_eq!(state(&world).hidden.contains(&pip), *success);
        let events = ask(&mut world, &data, CombatCommand::Attack { stack: 0 });
        let roll = events
            .iter()
            .find_map(|e| match e {
                Event::AttackResolved { roll, attacker, .. }
                    if *attacker == ActorRef::Member(pip) =>
                {
                    Some(roll.mode)
                }
                _ => None,
            })
            .unwrap();
        if *success {
            hid = true;
            assert_eq!(roll, RollMode::Advantage);
        } else {
            seen = true;
            assert_eq!(roll, RollMode::Normal);
        }
        if let Mode::Combat(state) = &world.mode {
            assert!(!state.hidden.contains(&pip), "the attack gives Pip away");
        }
        if hid && seen {
            return;
        }
    }
    panic!("both outcomes within 40 seeds: hid {hid}, seen {seen}");
}

#[test]
fn end_turn_passes_the_turn_and_a_reaction_is_never_a_turn_s_command() {
    let mut data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    assert_eq!(
        apply(&mut world, &data, Command::Combat(CombatCommand::EndTurn)),
        Err(Rejection::WrongMode)
    );
    fight(&mut world, &data, &[("goblin", 1)]);
    until_turn_of(&mut world, &data, BRENNA);
    let brenna = member(&world, BRENNA);
    let events = ask(&mut world, &data, CombatCommand::EndTurn);
    assert_ne!(current(&world), Some(brenna));
    assert!(events.iter().all(|e| !matches!(
        e,
        Event::AttackResolved { attacker, .. } if *attacker == brenna
    )));

    // A pack whose feature costs a reaction is refused at load; one changed in memory is
    // refused at the command.
    let fighter = data.registry.classes.get("base:class:fighter").unwrap();
    data.classes.get_mut(&fighter).unwrap().features[1].cost = Cost::Reaction;
    let mut world = common::world(&data);
    party_of(&mut world, &data, 1);
    fight(&mut world, &data, &[("goblin", 1)]);
    until_turn_of(&mut world, &data, BRENNA);
    refused(
        &mut world,
        &data,
        CombatCommand::Feature {
            feature: 0,
            choice: FeatureChoice::None,
        },
        Rejection::ReactionOnly,
    );
}

#[test]
fn the_words_name_the_turn_s_commands() {
    let script =
        script_for_six("end\nfeature-0\nfeature-1-2\nfeature-1-hide\ncast-1-0-bonus\ncast-1-m0");
    assert_eq!(
        script,
        [
            Command::Combat(CombatCommand::EndTurn),
            Command::Combat(CombatCommand::Feature {
                feature: 0,
                choice: FeatureChoice::None
            }),
            Command::Combat(CombatCommand::Feature {
                feature: 1,
                choice: FeatureChoice::Exchange {
                    with: CharacterId(2)
                }
            }),
            Command::Combat(CombatCommand::Feature {
                feature: 1,
                choice: FeatureChoice::Hide
            }),
            Command::Combat(CombatCommand::Cast {
                spell: 1,
                target: Target::Stack(0),
                pay: Pay::BonusAction
            }),
            Command::Combat(CombatCommand::Cast {
                spell: 1,
                target: Target::Member(CharacterId(0)),
                pay: Pay::Action
            }),
        ]
    );
    let words: Vec<&str> = script.iter().map(Command::word).collect();
    assert_eq!(
        words,
        ["end", "feature", "feature", "feature", "cast", "cast"]
    );
}

#[test]
fn a_turn_ends_when_its_action_is_spent_though_features_are_left() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 4);
    // Pip to the front, so both may swing at the goblins.
    let ids = world.party.ids();
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Reorder {
            order: vec![ids[3], ids[0], ids[1], ids[2]],
        }),
    )
    .unwrap();
    let (pip, brenna) = (0, 1);
    world.party.members[pip].level = 2;
    world.party.members[brenna].level = 2;
    fight(&mut world, &data, &[("goblin", 3), ("goblin", 3)]);
    for slot in [brenna, pip] {
        until_turn_of(&mut world, &data, slot);
        let who = member(&world, slot);
        let stack = u8::try_from(
            state(&world)
                .encounter
                .stacks
                .iter()
                .position(|s| s.alive())
                .unwrap(),
        )
        .unwrap();
        ask(&mut world, &data, CombatCommand::Attack { stack });
        assert_ne!(
            current(&world),
            Some(who),
            "Second Wind, Action Surge and Cunning Action are used before the action"
        );
    }
}

#[test]
fn getting_away_draws_a_swing_from_each_front_stack() {
    let data = data();
    let mut swung = false;
    for seed in 0..60u64 {
        let mut world = common::new_world(&data, seed, Settings::default());
        party_of(&mut world, &data, 4);
        fight(&mut world, &data, &[("goblin", 2), ("goblin", 2)]);
        let front: Vec<_> = world.party.members[..3].iter().map(|m| m.id).collect();
        let Some(ActorRef::Member(_)) = current(&world) else {
            continue;
        };
        let events = ask(&mut world, &data, CombatCommand::Run);
        let fled = events.iter().any(|e| {
            matches!(
                e,
                Event::Check {
                    kind: CheckKind::Flee,
                    success: true,
                    ..
                }
            )
        });
        let swings: Vec<&Event> = events
            .iter()
            .filter(|e| matches!(e, Event::OpportunityAttack { .. }))
            .collect();
        if !fled {
            assert!(swings.is_empty(), "nobody left: {events:?}");
            continue;
        }
        assert!(swings.len() <= 2, "one a front stack");
        for swing in &swings {
            let Event::OpportunityAttack { member, .. } = swing else {
                unreachable!()
            };
            assert!(front.contains(member), "a front-row member");
        }
        swung |= !swings.is_empty();
        if swung {
            return;
        }
    }
    panic!("no getaway drew a swing in 60 seeds");
}

#[test]
fn a_surprised_party_has_no_reaction_until_its_turn() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let here = world.position;
    let goblins = encounter(&data, &[("goblin", 1)], Disposition::Hostile, here);
    combat::start(&mut world, &data, goblins, Surprise::Party, &mut Vec::new()).unwrap();
    let Mode::Combat(state) = &world.mode else {
        return;
    };
    assert!(state.round >= 2, "the surprised party waits out round one");
    for slot in 0..2 {
        let me = member(&world, slot);
        // Reactions come with the member's first turn: in round two, the members at or before
        // the one the fight waits on have had it.
        let at = state.order.iter().position(|e| e.actor == me).unwrap();
        let had_a_turn = state.round > 2 || at <= usize::from(state.current);
        assert_eq!(state.reactions_left(me), u8::from(had_a_turn), "{slot}");
    }
    assert_eq!(
        state.reactions_left(ActorRef::Stack(0)),
        1,
        "the goblins were ready"
    );
}

#[test]
fn a_member_felled_on_the_way_out_falls_where_they_stood_and_the_turn_passes() {
    let data = data();
    for seed in 0..80u64 {
        let mut world = common::new_world(&data, seed, Settings::default());
        party_of(&mut world, &data, 4);
        fight(&mut world, &data, &[("goblin", 3), ("goblin", 3)]);
        until_turn_of(&mut world, &data, BRENNA);
        let brenna = member(&world, BRENNA);
        world.party.members[BRENNA].hp = 1;
        let pip = world.party.members[PIP].id;
        let events = ask(&mut world, &data, CombatCommand::Exchange { with: pip });
        if !world.party.members[BRENNA].is_down() {
            continue;
        }
        assert_eq!(member(&world, BRENNA), brenna, "she stays in slot 0");
        assert!(!events.iter().any(|e| matches!(e, Event::Exchanged { .. })));
        assert_ne!(
            current(&world),
            Some(brenna),
            "Second Wind is unused, but a fallen member's turn is over"
        );
        return;
    }
    panic!("no swing felled her in 80 seeds");
}
