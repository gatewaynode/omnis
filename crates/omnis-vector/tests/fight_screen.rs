//! The fight's controls as a screen (presentation-ARCHITECTURE §6.5, §6.7, §9; parity P7): the
//! budget line, the reactions switch, Features beside the turn's other choices, the spell notes,
//! the item rows with the engine's refusal, where each step stands, and the menu going back to
//! the turn's list when the turn passes. The dungeon's placed group, met by walking into it.

use crate::common;

use common::fight::turn_of;
use omnis_sim::api::TacticsCommand;
use omnis_sim::{CombatCommand, Command, FeatureChoice, Mode, PartyCommand, apply};
use omnis_vector::combat_menu::{Act, Action, CombatMenu, Step};
use omnis_vector::screen::fight::{FightAct, FightForm, Place, content, place, settle};
use omnis_vector::screen::{
    self, Control, ControlId, Effect, Forms, Kind, Payload, Refused, Screen, ScreenId,
};
use omnis_vector::shell::session::Session;

/// The fight's screen for the session at `step`.
fn shown(session: &Session, step: Step) -> Screen {
    let views = common::views(&session.world, &session.data);
    let ctx = common::ctx(&session.data, &session.world, &views);
    let mut form = FightForm::default();
    form.menu = CombatMenu { step };
    content(&session.data, ctx.game.expect("a game"), &form)
}

/// Every control of the screen, in order.
fn controls(screen: &Screen) -> Vec<&Control> {
    screen
        .panels
        .iter()
        .flat_map(|p| &p.rows)
        .flat_map(|r| &r.controls)
        .collect()
}

fn control<'a>(screen: &'a Screen, label: &str) -> &'a Control {
    controls(screen)
        .into_iter()
        .find(|c| c.label == label)
        .unwrap_or_else(|| panic!("a control {label:?} in {screen:?}"))
}

/// The words after a picker's control.
fn note(screen: &Screen, label: &str) -> Option<String> {
    screen
        .panels
        .iter()
        .flat_map(|p| &p.rows)
        .find(|r| r.controls.iter().any(|c| c.label == label))
        .and_then(|r| r.after.first())
        .map(|s| s.text.clone())
}

/// The first row's words: the budget line.
fn budget(screen: &Screen) -> String {
    screen.panels[0].rows[0].spans[0].text.clone()
}

/// What pressing `label` on the screen at `step` does.
fn press(session: &Session, step: Step, label: &str) -> Result<Effect, Refused> {
    let views = common::views(&session.world, &session.data);
    let ctx = common::ctx(&session.data, &session.world, &views);
    let mut forms = Forms::default();
    forms.fight.menu = CombatMenu { step };
    let id = control(&shown(session, step), label).id;
    screen::apply(ScreenId::Fight, &ctx, &mut forms, id, &Payload::Press, 0)
}

/// The command `label` sends at `step`.
fn sent(session: &Session, step: Step, label: &str) -> Command {
    match press(session, step, label) {
        Ok(Effect::Command {
            command,
            close: false,
        }) => command,
        other => panic!("{label} sends a command: {other:?}"),
    }
}

/// Pip's turn at level 2, where a rogue has Cunning Action.
fn rogue() -> Session {
    let mut session = turn_of("Pip");
    session.world.party.members[3].level = 2;
    session.views = common::views(&session.world, &session.data);
    session
}

fn accepted(session: &Session, command: &Command) {
    apply(&mut session.world.clone(), &session.data, command.clone())
        .unwrap_or_else(|r| panic!("{command:?} refused: {r}"));
}

#[test]
fn the_turn_offers_features_beside_the_others() {
    let brenna = turn_of("Brenna");
    let top = shown(&brenna, Step::Top);
    let labels: Vec<&str> = controls(&top).iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Reactions",
            "Attack",
            "Cast",
            "Features",
            "Use",
            "Dodge",
            "Swap",
            "End turn",
            "Flee"
        ]
    );
    assert_eq!(control(&top, "Features").why_not, None);
    let durin = turn_of("Durin");
    assert_eq!(
        control(&shown(&durin, Step::Top), "Features")
            .why_not
            .as_deref(),
        Some("has no features")
    );
    assert_eq!(
        press(&durin, Step::Top, "Features"),
        Err(Refused::Dim("has no features".to_owned())),
        "a dim entry is refused with its reason"
    );
}

#[test]
fn second_wind_reads_its_uses_and_cost_and_dims_once_spent() {
    let mut session = turn_of("Brenna");
    let features = shown(&session, Step::Features);
    assert_eq!(control(&features, "Second Wind").why_not, None);
    assert_eq!(
        note(&features, "Second Wind").as_deref(),
        Some("1 use, bonus action")
    );
    let wind = sent(&session, Step::Features, "Second Wind");
    assert!(matches!(
        &wind,
        Command::Combat(CombatCommand::Feature {
            choice: FeatureChoice::None,
            ..
        })
    ));
    accepted(&session, &wind);
    session.order(wind);
    assert_eq!(
        common::fight::acting(&session).as_deref(),
        Some("Brenna"),
        "the action is left"
    );
    let features = shown(&session, Step::Features);
    assert!(control(&features, "Second Wind").why_not.is_some(), "spent");
    assert_eq!(
        note(&features, "Second Wind").as_deref(),
        Some("0 uses, bonus action")
    );
    assert_eq!(
        control(&shown(&session, Step::Top), "Features")
            .why_not
            .as_deref(),
        Some("no feature can be used now")
    );
}

#[test]
fn cunning_action_splits_into_swap_and_hide() {
    let session = rogue();
    let features = shown(&session, Step::Features);
    assert_eq!(
        note(&features, "Cunning Action: swap").as_deref(),
        Some("at will, bonus action")
    );
    let pip = common::id(&session.world, 3);
    let brenna = common::id(&session.world, 0);
    let Ok(Effect::None) = press(&session, Step::Features, "Cunning Action: swap") else {
        panic!("the swap asks for a member");
    };
    let at = Step::Target(Action::Feature(0));
    let names: Vec<String> = controls(&shown(&session, at))
        .iter()
        .map(|c| c.label.clone())
        .collect();
    assert_eq!(names, ["Brenna", "Durin", "Ilvara", "Back"], "the others");
    let swap = sent(&session, at, "Brenna");
    assert!(
        matches!(
            &swap,
            Command::Combat(CombatCommand::Feature {
                choice: FeatureChoice::Exchange { with },
                ..
            }) if *with == brenna
        ),
        "{swap:?}"
    );
    assert_ne!(brenna, pip);
    accepted(&session, &swap);
    let hide = sent(&session, Step::Features, "Cunning Action: hide");
    assert!(matches!(
        &hide,
        Command::Combat(CombatCommand::Feature {
            choice: FeatureChoice::Hide,
            ..
        })
    ));
    accepted(&session, &hide);
}

#[test]
fn use_lists_only_items_with_the_engine_s_refusal() {
    let mut session = turn_of("Durin");
    let glass = session
        .data
        .registry
        .items
        .get("base:item:spyglass")
        .unwrap();
    session.world.party.members[1].equipment.push((glass, 1));
    session.views = common::views(&session.world, &session.data);
    let items = shown(&session, Step::Items);
    let labels: Vec<&str> = controls(&items).iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        ["Potion of healing", "Spyglass", "Back"],
        "items only"
    );
    assert_eq!(control(&items, "Potion of healing").why_not, None);
    assert_eq!(
        control(&items, "Spyglass").why_not.as_deref(),
        Some("not from a fight")
    );
    // With the action spent, nothing is used, as the engine says.
    if let Mode::Combat(c) = &mut session.world.mode {
        c.budget.actions = 0;
    }
    let items = shown(&session, Step::Items);
    assert_eq!(
        control(&items, "Potion of healing").why_not.as_deref(),
        Some("the turn's action is spent")
    );
    assert_eq!(
        control(&shown(&session, Step::Top), "Use")
            .why_not
            .as_deref(),
        Some("nothing to use now")
    );
}

#[test]
fn the_spell_notes_go_reaction_then_the_refusal_then_in_force_then_bonus_action() {
    // A reaction is never cast from the picker.
    let mut ilvara = turn_of("Ilvara");
    let shield = ilvara
        .data
        .registry
        .spells
        .get("base:spell:shield")
        .unwrap();
    ilvara.world.party.members[2].known_spells.push(shield);
    let spells = shown(&ilvara, Step::Spells);
    assert_eq!(
        control(&spells, "Shield  1 pt").why_not.as_deref(),
        Some("reaction")
    );
    assert_eq!(
        note(&spells, "Shield  1 pt").as_deref(),
        Some("declare it in tactics")
    );
    // Healing Word takes the bonus action; Bless, cast, is in force on Durin's next turn.
    let mut durin = turn_of("Durin");
    let word = durin
        .data
        .registry
        .spells
        .get("base:spell:healing_word")
        .unwrap();
    durin.world.party.members[1].known_spells.push(word);
    let spells = shown(&durin, Step::Spells);
    assert_eq!(
        note(&spells, "Healing Word  1 pt").as_deref(),
        Some("bonus action")
    );
    assert_eq!(note(&spells, "Bless  1 pt"), None);
    let Ok(Effect::None) = press(&durin, Step::Spells, "Bless  1 pt") else {
        panic!("Bless asks for a member");
    };
    let at = Step::Target(Action::Cast(3));
    assert!(durin.order(sent(&durin, at, "Brenna")));
    // Durin's bonus action is left, so his turn goes on until it is ended.
    assert!(durin.order(Command::Combat(CombatCommand::EndTurn)));
    for _ in 0..20 {
        if common::fight::acting(&durin).as_deref() == Some("Durin") {
            break;
        }
        assert!(durin.order(Command::Combat(CombatCommand::Dodge)));
    }
    assert_eq!(common::fight::acting(&durin).as_deref(), Some("Durin"));
    let spells = shown(&durin, Step::Spells);
    assert_eq!(note(&spells, "Bless  1 pt").as_deref(), Some("in force"));
    // Out of points, the refusal is shown and the note is not.
    durin.world.party.members[1].spell_points = 0;
    let spells = shown(&durin, Step::Spells);
    assert_eq!(
        control(&spells, "Bless  1 pt").why_not.as_deref(),
        Some("need 1 pt")
    );
    assert_eq!(note(&spells, "Bless  1 pt"), None);
}

#[test]
fn the_budget_line_reads_the_acting_member_s_turn() {
    let mut session = turn_of("Durin");
    assert_eq!(
        budget(&shown(&session, Step::Top)),
        "Action 1   Bonus 1   Reaction 1 (on)"
    );
    assert_eq!(
        budget(&shown(&session, Step::Spells)),
        "SP 2/2   Action 1   Bonus 1   Reaction 1 (on)",
        "the pickers add the spell points"
    );
    let durin = omnis_sim::ActorRef::Member(common::id(&session.world, 1));
    if let Mode::Combat(c) = &mut session.world.mode {
        // Another's reaction spent does not show; Durin's does.
        for (who, left) in &mut c.reactions {
            *left = u8::from(*who == durin);
        }
    }
    assert_eq!(
        budget(&shown(&session, Step::Top)),
        "Action 1   Bonus 1   Reaction 1 (on)"
    );
    if let Mode::Combat(c) = &mut session.world.mode {
        for (who, left) in &mut c.reactions {
            *left = u8::from(*who != durin);
        }
        c.budget.bonus_actions = 0;
    }
    assert_eq!(
        budget(&shown(&session, Step::Top)),
        "Action 1   Bonus 0   Reaction 0 (on)"
    );
}

#[test]
fn react_flips_the_acting_member_s_reactions_and_costs_nothing() {
    let mut session = turn_of("Durin");
    let durin = common::id(&session.world, 1);
    let top = shown(&session, Step::Top);
    let switch = control(&top, "Reactions");
    assert_eq!(switch.id, ControlId::Fight(FightAct::Reactions));
    assert!(matches!(&switch.kind, Kind::Choice { at: 1, .. }), "on");
    let off = sent(&session, Step::Top, "Reactions");
    assert_eq!(
        off,
        Command::Party(PartyCommand::Tactics(TacticsCommand::SetReactions {
            member: durin,
            on: false
        }))
    );
    let before = session.views.combat.as_ref().map(|v| v.budget);
    session.order(off);
    assert_eq!(common::fight::acting(&session).as_deref(), Some("Durin"));
    assert_eq!(
        session.views.combat.as_ref().map(|v| v.budget),
        before,
        "free"
    );
    let top = shown(&session, Step::Top);
    assert!(matches!(
        &control(&top, "Reactions").kind,
        Kind::Choice { at: 0, .. }
    ));
    assert!(budget(&top).ends_with("(off)"));
    assert_eq!(
        sent(&session, Step::Top, "Reactions"),
        Command::Party(PartyCommand::Tactics(TacticsCommand::SetReactions {
            member: durin,
            on: true
        }))
    );
}

#[test]
fn the_pickers_stand_over_the_picture_and_targets_in_the_column() {
    let mut form = FightForm::default();
    for (step, at) in [
        (Step::Top, Place::Column),
        (Step::Spells, Place::Over),
        (Step::Features, Place::Over),
        (Step::Items, Place::Over),
        (Step::Target(Action::Cast(0)), Place::Column),
        (Step::Target(Action::Feature(0)), Place::Column),
    ] {
        form.menu = CombatMenu { step };
        assert_eq!(place(&form), at, "{step:?}");
    }
}

#[test]
fn the_menu_goes_back_to_the_top_when_the_turn_passes() {
    let mut session = turn_of("Durin");
    let mut form = FightForm::default();
    let views = common::views(&session.world, &session.data);
    settle(
        common::ctx(&session.data, &session.world, &views)
            .game
            .unwrap(),
        &mut form,
    );
    form.menu.step = Step::Target(Action::Attack);
    settle(
        common::ctx(&session.data, &session.world, &views)
            .game
            .unwrap(),
        &mut form,
    );
    assert_eq!(
        form.menu.step,
        Step::Target(Action::Attack),
        "the same turn keeps it"
    );
    session.order(Command::Combat(CombatCommand::Dodge));
    let views = common::views(&session.world, &session.data);
    settle(
        common::ctx(&session.data, &session.world, &views)
            .game
            .unwrap(),
        &mut form,
    );
    assert_eq!(form.menu.step, Step::Top);
}

#[test]
fn every_command_offered_on_a_rogue_s_turn_is_accepted() {
    let session = rogue();
    let mut queue = vec![Step::Top];
    let mut seen = Vec::new();
    let mut commands = 0;
    while let Some(step) = queue.pop() {
        if seen.contains(&step) {
            continue;
        }
        seen.push(step);
        for e in (CombatMenu { step }).entries(&session.world, &session.data) {
            match (&e.act, &e.blocked) {
                (Act::Open(next), None) => queue.push(*next),
                (Act::Command(command), None) => {
                    accepted(&session, command);
                    commands += 1;
                }
                _ => {}
            }
        }
    }
    assert!(seen.contains(&Step::Target(Action::Feature(0))), "{seen:?}");
    assert!(commands >= 8, "{commands} commands offered");
}
