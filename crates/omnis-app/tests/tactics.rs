//! The tactics panel (M7c step 7), driven as a person does: the sheet's TACTICS button opens it
//! on the member shown and Escape goes back to the sheet; in a fight the sheet stays shut and
//! the button would be grey; a wizard declares Shield on being attacked under a condition, edits it to
//! any of two, removes it, and switches reactions off; a fighter may declare only the attack
//! on an enemy fleeing; a reaction set deeper elsewhere is shown and may not be saved; the
//! menus, a number and Save work by pointer; the panel lies inside the map at both window
//! sizes at its fullest.

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use common::feathers::{
    activate, change, click_node, control, controls, layout_faults, settle, shown, text_tree,
    ultrawide,
};
use common::{click, feathers_app, fighter_draft, key, place, play_state, tool, world};
use omnis_app::sheet_menu::ROW_TACTICS;
use omnis_app::sim::{PackData, PlayState, PlayerCommand, SimWorld};
use omnis_app::tactics_panel::{CONDITIONS_MAX, Field, TacticsLabelId, TacticsPanelId};
use omnis_app::tool_bar::ToolButton;
use omnis_app::ui_kit::UiId;
use omnis_app::ui_text::screen_text;
use omnis_app::widget::{Part, WidgetId};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Skill;
use omnis_sim::omnis_rules::{ActionRef, Cmp, Criteria, CriteriaSet, Predicate, Trigger, Who};
use omnis_sim::party::PartyCommand;
use omnis_sim::tactics::TacticsCommand;
use omnis_sim::{Command, Mode};

const BRENNA: usize = 0;
const ILVARA: usize = 1;

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    settle(app);
}

/// A new game on the meadow with Brenna, a fighter, and Ilvara, a wizard who knows Shield.
fn meadow(save: &str) -> App {
    let mut app = feathers_app(save, true);
    settle(&mut app);
    send(
        &mut app,
        Command::Party(PartyCommand::Create(fighter_draft())),
    );
    let mut wizard = fighter_draft();
    wizard.name = "Ilvara".into();
    wizard.class = "base:class:wizard".into();
    wizard.skills = vec![Skill::Arcana, Skill::History];
    send(&mut app, Command::Party(PartyCommand::Create(wizard)));
    place(&mut app, "test:map:meadow", 16, 16, Facing::North);
    let shield = app
        .world()
        .resource::<PackData>()
        .0
        .registry
        .spells
        .get("base:spell:shield")
        .unwrap();
    let known =
        &mut app.world_mut().resource_mut::<SimWorld>().0.party.members[ILVARA].known_spells;
    if !known.contains(&shield) {
        known.push(shield);
    }
    settle(&mut app);
    assert_eq!(play_state(&app), PlayState::Explore);
    app
}

/// The sheet by its tool, on `member`, then TACTICS by pointer.
fn open(app: &mut App, member: usize) {
    tool(app, ToolButton::Sheet);
    assert_eq!(play_state(app), PlayState::Sheet);
    for _ in 0..member {
        key(app, Key::ArrowRight);
    }
    click(app, WidgetId::Row(ROW_TACTICS), Part::Body);
    assert_eq!(play_state(app), PlayState::Tactics);
    settle(app);
}

fn declared(app: &App, member: usize) -> Vec<CriteriaSet> {
    world(app).party.members[member]
        .tactics
        .reactions()
        .into_iter()
        .cloned()
        .collect()
}

fn dim(app: &mut App, id: TacticsPanelId) -> bool {
    let entity = control(app, id);
    app.world().get::<InteractionDisabled>(entity).is_some()
}

#[test]
fn the_sheet_opens_the_panel_on_its_member_and_escape_goes_back() {
    let mut app = meadow("tactics-open.ron");
    open(&mut app, ILVARA);
    assert_eq!(shown(&mut app, TacticsLabelId::Member), "Ilvara");
    assert_eq!(shown(&mut app, TacticsLabelId::Editing), "New reaction");
    let tree = text_tree(&mut app);
    assert!(
        tree.contains("[Save]") && tree.contains("[Reactions]"),
        "{tree}"
    );
    assert!(
        screen_text(app.world()).starts_with("Tactics panel"),
        "{tree}"
    );
    key(&mut app, Key::Escape);
    assert_eq!(
        play_state(&app),
        PlayState::Sheet,
        "Escape goes back to the sheet"
    );
    click(&mut app, WidgetId::Row(ROW_TACTICS), Part::Body);
    activate(&mut app, TacticsPanelId::MemberPick(BRENNA));
    assert_eq!(shown(&mut app, TacticsLabelId::Member), "Brenna");
    activate(&mut app, TacticsPanelId::Close);
    assert_eq!(play_state(&app), PlayState::Sheet, "so does Close");
}

#[test]
fn in_a_fight_tactics_stay_shut() {
    let mut app = meadow("tactics-fight.ron");
    place(&mut app, "test:map:dungeon", 3, 7, Facing::South);
    send(
        &mut app,
        Command::Step(omnis_sim::omnis_core::Direction::Forward),
    );
    assert!(matches!(
        world(&app).mode,
        Mode::Encounter(_) | Mode::Combat(_)
    ));
    let before = play_state(&app);
    tool(&mut app, ToolButton::Sheet);
    assert_eq!(
        play_state(&app),
        before,
        "the bar holds the sheet shut in a fight"
    );
    let data = &app.world().resource::<PackData>().0;
    let view = omnis_app::sheet_menu::sheet_view(world(&app), data, 0).unwrap();
    assert!(!view.tactics, "and were it open, TACTICS would be grey");
}

#[test]
fn a_wizard_declares_shield_under_a_condition_edits_it_and_removes_it() {
    let mut app = meadow("tactics-declare.ron");
    open(&mut app, ILVARA);
    assert!(
        !dim(&mut app, TacticsPanelId::Save),
        "attack on fleeing is ready"
    );
    activate(&mut app, TacticsPanelId::ActionPick(1));
    assert_eq!(shown(&mut app, TacticsLabelId::Action), "Shield");
    assert_eq!(shown(&mut app, TacticsLabelId::Trigger), "attacked");
    activate(&mut app, TacticsPanelId::Add);
    assert_eq!(shown(&mut app, TacticsLabelId::Kind(0)), "hit points %");
    activate(&mut app, TacticsPanelId::FieldPick(0, Field::Cmp, 0));
    change(&mut app, TacticsPanelId::Number(0), 50_i32);
    activate(&mut app, TacticsPanelId::Save);
    let hp = Criteria::Is(Predicate::Hp {
        who: Who::Me,
        cmp: Cmp::Lt,
        percent: 50,
    });
    let sets = declared(&app, ILVARA);
    assert_eq!(sets.len(), 1);
    assert_eq!(
        (&sets[0].name, &sets[0].trigger, &sets[0].when),
        (
            &"Shield on attacked".to_owned(),
            &Trigger::Attacked,
            &Criteria::All(vec![hp.clone()])
        )
    );
    assert_eq!(
        shown(&mut app, TacticsLabelId::Entry(0)),
        "1. Shield on attacked, when me HP < 50%"
    );
    assert_eq!(
        shown(&mut app, TacticsLabelId::Editing),
        "New reaction",
        "a declaration starts the form afresh"
    );

    activate(&mut app, TacticsPanelId::Edit(0));
    assert_eq!(
        shown(&mut app, TacticsLabelId::Editing),
        "Editing reaction 1"
    );
    activate(&mut app, TacticsPanelId::CombinePick(1));
    activate(&mut app, TacticsPanelId::Add);
    activate(&mut app, TacticsPanelId::KindPick(1, 7));
    activate(&mut app, TacticsPanelId::Save);
    let sets = declared(&app, ILVARA);
    assert_eq!(sets.len(), 1, "written over, not added");
    assert_eq!(
        shown(&mut app, TacticsLabelId::Editing),
        "New reaction",
        "the edit is done: the form starts afresh"
    );
    assert_eq!(
        sets[0].when,
        Criteria::Any(vec![hp, Criteria::Is(Predicate::WouldChangeOutcome)])
    );

    activate(&mut app, TacticsPanelId::Remove(0));
    assert!(declared(&app, ILVARA).is_empty());
    assert!(!controls(&mut app).contains(&UiId::Tactics(TacticsPanelId::Edit(0))));

    let switch = control(&mut app, TacticsPanelId::Reactions);
    assert!(app.world().get::<Checked>(switch).is_some(), "on to start");
    change(&mut app, TacticsPanelId::Reactions, false);
    assert!(!world(&app).party.members[ILVARA].tactics.reactions_on);
    let switch = control(&mut app, TacticsPanelId::Reactions);
    assert!(
        app.world().get::<Checked>(switch).is_none(),
        "the box follows"
    );
}

#[test]
fn a_fighter_may_declare_only_the_attack_on_an_enemy_fleeing() {
    let mut app = meadow("tactics-fighter.ron");
    open(&mut app, BRENNA);
    assert_eq!(shown(&mut app, TacticsLabelId::Action), "attack");
    assert_eq!(shown(&mut app, TacticsLabelId::Trigger), "an enemy flees");
    let ids = controls(&mut app);
    assert!(ids.contains(&UiId::Tactics(TacticsPanelId::ActionPick(0))));
    assert!(!ids.contains(&UiId::Tactics(TacticsPanelId::ActionPick(1))));
    assert!(!ids.contains(&UiId::Tactics(TacticsPanelId::TriggerPick(1))));
    activate(&mut app, TacticsPanelId::Save);
    let sets = declared(&app, BRENNA);
    assert_eq!(
        (sets.len(), &sets[0].action, sets[0].trigger),
        (1, &ActionRef::Attack, Trigger::EnemyFlees)
    );
}

#[test]
fn a_reaction_set_deeper_elsewhere_is_shown_and_not_saved() {
    let mut app = meadow("tactics-deep.ron");
    let set = CriteriaSet {
        name: "deep".into(),
        action: ActionRef::Attack,
        trigger: Trigger::EnemyFlees,
        when: Criteria::Any(vec![Criteria::All(vec![Criteria::Is(Predicate::Round {
            cmp: Cmp::Ge,
            n: 2,
        })])]),
    };
    send(
        &mut app,
        Command::Party(PartyCommand::Tactics(TacticsCommand::PutReaction {
            member: 0,
            at: None,
            set,
        })),
    );
    open(&mut app, BRENNA);
    assert_eq!(
        shown(&mut app, TacticsLabelId::Entry(0)),
        "1. attack on an enemy flees, when (round >= 2)"
    );
    activate(&mut app, TacticsPanelId::Edit(0));
    assert!(dim(&mut app, TacticsPanelId::Save));
    assert!(dim(&mut app, TacticsPanelId::Add));
    assert!(shown(&mut app, TacticsLabelId::Message).starts_with("Set elsewhere"));
    activate(&mut app, TacticsPanelId::Remove(0));
    assert!(declared(&app, BRENNA).is_empty(), "removed all the same");
}

#[test]
fn the_menus_a_number_the_switch_and_save_work_by_pointer() {
    let mut app = meadow("tactics-pointer.ron");
    open(&mut app, ILVARA);
    let menu = control(&mut app, TacticsPanelId::Action);
    click_node(&mut app, menu);
    let item = control(&mut app, TacticsPanelId::ActionPick(1));
    click_node(&mut app, item);
    assert_eq!(shown(&mut app, TacticsLabelId::Action), "Shield");
    let add = control(&mut app, TacticsPanelId::Add);
    click_node(&mut app, add);
    let menu = control(&mut app, TacticsPanelId::Field(0, Field::Cmp));
    click_node(&mut app, menu);
    let item = control(&mut app, TacticsPanelId::FieldPick(0, Field::Cmp, 4));
    click_node(&mut app, item);
    assert_eq!(shown(&mut app, TacticsLabelId::Field(0, Field::Cmp)), ">");
    let save = control(&mut app, TacticsPanelId::Save);
    click_node(&mut app, save);
    let sets = declared(&app, ILVARA);
    assert_eq!(sets.len(), 1);
    assert_eq!(
        sets[0].when,
        Criteria::All(vec![Criteria::Is(Predicate::Hp {
            who: Who::Me,
            cmp: Cmp::Gt,
            percent: 0,
        })])
    );
    let switch = control(&mut app, TacticsPanelId::Reactions);
    click_node(&mut app, switch);
    assert!(!world(&app).party.members[ILVARA].tactics.reactions_on);
}

#[test]
fn the_panel_lies_inside_the_map_at_both_window_sizes_at_its_fullest() {
    let mut app = meadow("tactics-layout.ron");
    for _ in 0..16 {
        let set = CriteriaSet {
            name: "attack on an enemy flees".into(),
            action: ActionRef::Attack,
            trigger: Trigger::EnemyFlees,
            when: Criteria::Always,
        };
        send(
            &mut app,
            Command::Party(PartyCommand::Tactics(TacticsCommand::PutReaction {
                member: 1,
                at: None,
                set,
            })),
        );
    }
    assert_eq!(declared(&app, ILVARA).len(), 16);
    open(&mut app, ILVARA);
    assert!(
        dim(&mut app, TacticsPanelId::Save),
        "sixteen rows are the most"
    );
    for row in 0..CONDITIONS_MAX {
        activate(&mut app, TacticsPanelId::Add);
        activate(&mut app, TacticsPanelId::KindPick(row, row % 8));
    }
    assert!(
        dim(&mut app, TacticsPanelId::Add),
        "fifteen conditions are the most"
    );
    for size in ["1280 by 720", "5120 by 1440"] {
        if size.starts_with("5120") {
            ultrawide(&mut app);
            settle(&mut app);
        }
        assert_eq!(layout_faults(&mut app), Vec::new(), "{size}");
        if let Ok(dir) = std::env::var("OMNIS_DUMP_SCREENS")
            && size.starts_with("1280")
        {
            let path = std::path::Path::new(&dir).join("tactics.txt");
            std::fs::write(&path, screen_text(app.world()))
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
    }
    let tree = text_tree(&mut app);
    for id in controls(&mut app) {
        assert!(
            tree.contains(&format!("[{}]", id.name())),
            "{id:?} in\n{tree}"
        );
    }
}
