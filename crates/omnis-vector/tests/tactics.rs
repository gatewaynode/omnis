//! Tactics (parity P6; presentation-ARCHITECTURE §6.7): the reactions switch, the declared
//! list with Edit and Remove, the form that writes a reaction (action, trigger, conditions
//! joined by all or any), Save as a new row or over the one edited, and the lock on a reaction
//! set deeper than the form writes. Ilvara is given Shield, a reaction spell, so she has
//! something to declare.

use crate::common;

use common::ctx;
use omnis_sim::api::{
    ActionRef, Cmp, Criteria, CriteriaSet, Predicate, TacticsCommand, Trigger, Who,
};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, PartyCommand};
use omnis_vector::screen::tactics::{LOCKED, TacticAct};
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, Screen, ScreenId};
use omnis_vector::sim::SimWorld;

const ILVARA: u8 = 2;

fn world_with_shield(data: &Data) -> SimWorld {
    let (mut world, _) = common::session(data);
    let shield = data.registry.spells.get("base:spell:shield").unwrap();
    let ilvara = &mut world.party.members[usize::from(ILVARA)];
    if !ilvara.known_spells.contains(&shield) {
        ilvara.known_spells.push(shield);
    }
    world
}

fn forms_for(world: &SimWorld, slot: u8) -> Forms {
    let mut forms = Forms::default();
    forms.owner = Some(common::id(world, slot));
    forms
}

fn shown(data: &Data, world: &SimWorld, forms: &Forms) -> Screen {
    let views = common::views(world, data);
    screen::content(ScreenId::Tactics, &ctx(data, world, &views), forms)
}

fn press(
    data: &Data,
    world: &SimWorld,
    forms: &mut Forms,
    a: TacticAct,
    payload: &Payload,
) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    screen::apply(
        ScreenId::Tactics,
        &ctx(data, world, &views),
        forms,
        ControlId::Tactic(a),
        payload,
        0,
    )
}

/// Press `a`, which sends a command, and apply it to the world.
fn send(data: &Data, world: &mut SimWorld, forms: &mut Forms, a: TacticAct) {
    let Ok(Effect::Command { command, close }) = press(data, world, forms, a, &Payload::Press)
    else {
        panic!("{a:?} sends a command");
    };
    assert!(!close, "the screen stays open");
    world.apply(data, command).expect("accepted");
}

fn why_not(screen: &Screen, a: TacticAct) -> Option<String> {
    screen
        .control(ControlId::Tactic(a))
        .unwrap_or_else(|| panic!("{a:?} on {screen:?}"))
        .why_not
        .clone()
}

/// Every text on the screen, spans and after-text.
fn texts(screen: &Screen) -> Vec<String> {
    screen
        .panels
        .iter()
        .flat_map(|p| &p.rows)
        .flat_map(|r| r.spans.iter().chain(&r.after))
        .map(|s| s.text.clone())
        .collect()
}

fn reactions(data: &Data, world: &SimWorld, slot: u8) -> Vec<String> {
    common::views(world, data).party.members[usize::from(slot)]
        .tactics
        .reactions
        .iter()
        .map(|r| r.name.clone())
        .collect()
}

#[test]
fn the_switch_turns_the_member_s_reactions_off_and_on() {
    let data = common::data();
    let (mut world, _) = common::session(&data);
    let mut forms = forms_for(&world, 0);
    let on = |world: &SimWorld| {
        common::views(world, &data).party.members[0]
            .tactics
            .reactions_on
    };
    assert!(on(&world), "on by default");
    send(&data, &mut world, &mut forms, TacticAct::Reactions);
    assert!(!on(&world));
    send(&data, &mut world, &mut forms, TacticAct::Reactions);
    assert!(on(&world));
}

#[test]
fn save_declares_what_the_form_reads_and_edit_writes_over_it() {
    let data = common::data();
    let mut world = world_with_shield(&data);
    let mut forms = forms_for(&world, ILVARA);
    let screen = shown(&data, &world, &forms);
    assert!(texts(&screen).contains(&"None declared".to_owned()));
    assert!(texts(&screen).contains(&"New reaction".to_owned()));
    let action = screen
        .control(ControlId::Tactic(TacticAct::Action))
        .unwrap();
    assert_eq!(
        action.kind,
        screen::Kind::Choice {
            options: vec!["Shield".to_owned()],
            at: 0
        }
    );
    press(&data, &world, &mut forms, TacticAct::Add, &Payload::Press).unwrap();
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Number(0),
        &Payload::Set(50),
    )
    .unwrap();
    assert_eq!(
        why_not(&shown(&data, &world, &forms), TacticAct::Save),
        None
    );
    send(&data, &mut world, &mut forms, TacticAct::Save);
    assert_eq!(reactions(&data, &world, ILVARA), ["Shield on attacked"]);
    let screen = shown(&data, &world, &forms);
    assert!(
        texts(&screen).contains(&"1. Shield on attacked, when me HP < 50%".to_owned()),
        "{:?}",
        texts(&screen)
    );
    assert!(forms.tactics.conditions.is_empty(), "the form starts over");

    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Edit(0),
        &Payload::Press,
    )
    .unwrap();
    assert_eq!(forms.tactics.editing, Some(0));
    assert_eq!(forms.tactics.conditions[0].n, 50, "the row loads");
    assert!(texts(&shown(&data, &world, &forms)).contains(&"Editing reaction 1".to_owned()));
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Number(0),
        &Payload::Set(25),
    )
    .unwrap();
    press(&data, &world, &mut forms, TacticAct::Join, &Payload::Press).unwrap();
    send(&data, &mut world, &mut forms, TacticAct::Save);
    let view = &common::views(&world, &data).party.members[usize::from(ILVARA)].tactics;
    assert_eq!(view.reactions.len(), 1, "written over, not added");
    assert!(
        texts(&shown(&data, &world, &forms))
            .contains(&"1. Shield on attacked, when me HP < 25%".to_owned())
    );
    assert!(matches!(view.reactions[0].when, Criteria::Any(_)));

    send(&data, &mut world, &mut forms, TacticAct::Remove(0));
    assert!(reactions(&data, &world, ILVARA).is_empty());
}

#[test]
fn a_condition_s_kind_and_fields_step_through_their_lists() {
    let data = common::data();
    let world = world_with_shield(&data);
    let mut forms = forms_for(&world, ILVARA);
    press(&data, &world, &mut forms, TacticAct::Add, &Payload::Press).unwrap();
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Number(0),
        &Payload::Set(80),
    )
    .unwrap();
    // Hp, then left twice: monsters standing, which takes any count.
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Kind(0),
        &Payload::Left,
    )
    .unwrap();
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Kind(0),
        &Payload::Left,
    )
    .unwrap();
    let screen = shown(&data, &world, &forms);
    let labels: Vec<_> = screen.panels[1]
        .rows
        .iter()
        .flat_map(|r| &r.controls)
        .map(|c| c.label.as_str())
        .collect();
    assert!(
        labels.contains(&"Monster") && labels.contains(&"Is"),
        "{labels:?}"
    );
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Field(0, 0),
        &Payload::Press,
    )
    .unwrap();
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Field(0, 1),
        &Payload::Left,
    )
    .unwrap();
    let draft = forms.tactics.conditions[0];
    assert_eq!((draft.monster, draft.cmp, draft.n), (1, 4, 80));
    // Back to hit points: the number is held within 100.
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Number(0),
        &Payload::Set(500),
    )
    .unwrap();
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Kind(0),
        &Payload::Press,
    )
    .unwrap();
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Kind(0),
        &Payload::Press,
    )
    .unwrap();
    assert_eq!(forms.tactics.conditions[0].n, 100);
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Drop(0),
        &Payload::Press,
    )
    .unwrap();
    assert!(forms.tactics.conditions.is_empty());
}

#[test]
fn a_reaction_set_deeper_loads_locked_and_may_only_be_removed() {
    let data = common::data();
    let mut world = world_with_shield(&data);
    let ilvara = common::id(&world, ILVARA);
    let hp = |percent| {
        Criteria::Is(Predicate::Hp {
            who: Who::Me,
            cmp: Cmp::Lt,
            percent,
        })
    };
    let nested = CriteriaSet {
        name: "deep".to_owned(),
        action: ActionRef::Spell("base:spell:shield".to_owned()),
        trigger: Trigger::Attacked,
        when: Criteria::All(vec![Criteria::Any(vec![hp(10), hp(20)]), hp(30)]),
    };
    world
        .apply(
            &data,
            Command::Party(PartyCommand::Tactics(TacticsCommand::PutReaction {
                member: ilvara,
                entry: None,
                set: nested,
            })),
        )
        .expect("set over the socket");
    let mut forms = forms_for(&world, ILVARA);
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Edit(0),
        &Payload::Press,
    )
    .unwrap();
    assert!(forms.tactics.locked);
    let screen = shown(&data, &world, &forms);
    assert!(texts(&screen).contains(&LOCKED.to_owned()));
    for a in [
        TacticAct::Save,
        TacticAct::Add,
        TacticAct::Action,
        TacticAct::Join,
    ] {
        assert_eq!(why_not(&screen, a).as_deref(), Some(LOCKED), "{a:?}");
    }
    assert_eq!(
        press(&data, &world, &mut forms, TacticAct::Save, &Payload::Press),
        Err(Refused::Dim(LOCKED.to_owned()))
    );
    assert_eq!(why_not(&screen, TacticAct::Remove(0)), None);
    assert_eq!(why_not(&screen, TacticAct::New), None);
    press(&data, &world, &mut forms, TacticAct::New, &Payload::Press).unwrap();
    assert!(!forms.tactics.locked, "New starts over");
    send(&data, &mut world, &mut forms, TacticAct::Remove(0));
    assert!(reactions(&data, &world, ILVARA).is_empty());
}

#[test]
fn the_form_s_limits_and_a_member_with_nothing_to_declare() {
    let data = common::data();
    let mut world = world_with_shield(&data);
    let mut forms = forms_for(&world, ILVARA);
    for _ in 0..15 {
        press(&data, &world, &mut forms, TacticAct::Add, &Payload::Press).unwrap();
    }
    assert_eq!(
        why_not(&shown(&data, &world, &forms), TacticAct::Add).as_deref(),
        Some("Fifteen conditions at most")
    );

    let ilvara = common::id(&world, ILVARA);
    for n in 1..=16 {
        let set = CriteriaSet {
            name: format!("round {n}"),
            action: ActionRef::Spell("base:spell:shield".to_owned()),
            trigger: Trigger::Attacked,
            when: Criteria::Is(Predicate::Round { cmp: Cmp::Eq, n }),
        };
        world
            .apply(
                &data,
                Command::Party(PartyCommand::Tactics(TacticsCommand::PutReaction {
                    member: ilvara,
                    entry: None,
                    set,
                })),
            )
            .expect("a runbook holds sixteen");
    }
    let mut forms = forms_for(&world, ILVARA);
    assert_eq!(
        why_not(&shown(&data, &world, &forms), TacticAct::Save).as_deref(),
        Some("Sixteen reactions at most")
    );
    press(
        &data,
        &world,
        &mut forms,
        TacticAct::Edit(3),
        &Payload::Press,
    )
    .unwrap();
    assert_eq!(
        why_not(&shown(&data, &world, &forms), TacticAct::Save),
        None,
        "editing a row needs no room"
    );

    let brenna = forms_for(&world, 0);
    let screen = shown(&data, &world, &brenna);
    assert!(texts(&screen).contains(&"nothing to declare yet".to_owned()));
    assert_eq!(
        why_not(&screen, TacticAct::Save).as_deref(),
        Some("Nothing this member can declare")
    );
}
