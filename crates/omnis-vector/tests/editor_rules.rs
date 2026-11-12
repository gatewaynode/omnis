//! The editor's rules list (rule priority, owner 2026-11-11): a Rules button in the tools column
//! opens a modal naming every slot, value and table with the file whose definition stands and
//! its priority; a rule that beat another is marked with what it beat; a search narrows the
//! list; a rule's file opens as text when the pack being edited wrote it. Headless.

use crate::common;
use bevy::prelude::*;
use common::app::{config, press_ui, shell};
use common::editor::{editing, in_editor, press_edit, scratch_pack, top_screen, type_ui};
use omnis_sim::omnis_data::rules::RuleKind;
use omnis_vector::editor::EditAct;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::editor::Editing;
use omnis_vector::shell::screen::Pressable;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;

/// Where `kind` `name` stands among the loaded rules: the number its file's button carries.
fn rule(app: &App, kind: RuleKind, name: &str) -> u16 {
    let data = &app.world().resource::<Packs>().data;
    let at = (data.standings.keys())
        .position(|(k, n)| *k == kind && n == name)
        .expect("a loaded rule");
    u16::try_from(at).unwrap()
}

#[test]
fn the_rules_button_lists_every_rule_with_its_file_and_a_search_narrows_it() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let tools = omnis_vector::editor::view::tools(editing(&app), &data);
    assert!(
        tools.control(ControlId::Edit(EditAct::Rules)).is_some(),
        "the Rules button stands in the tools column"
    );
    press_edit(&mut app, EditAct::Rules);
    assert_eq!(top_screen(&app), Some(ScreenId::EditorRules));
    let text = screen_text(app.world());
    assert!(text.contains("69 rules, 0 overriding another"), "{text}");
    assert!(text.contains("slot attack.total"), "{text}");
    assert!(text.contains("value death_save_dc"), "{text}");
    assert!(text.contains("table run_dc"), "{text}");
    assert!(text.contains("base:rules:combat  priority 0"), "{text}");
    assert!(!text.contains("OVERRIDES"), "{text}");
    // The search reads the kind, the name, the file and the pack.
    type_ui(&mut app, ControlId::Edit(EditAct::RulesFilter), "RUN_dc");
    let text = screen_text(app.world());
    assert!(text.contains("table run_dc"), "{text}");
    assert!(!text.contains("attack.total"), "{text}");
    type_ui(
        &mut app,
        ControlId::Edit(EditAct::RulesFilter),
        "no such rule",
    );
    assert!(screen_text(app.world()).contains("No rule matches."));
    // Base is not the pack being edited: its file's button is dim, so nothing presses it.
    type_ui(&mut app, ControlId::Edit(EditAct::RulesFilter), "");
    let total = Pressable(ControlId::Edit(EditAct::RuleFile(rule(
        &app,
        RuleKind::Slot,
        "attack.total",
    ))));
    let world = app.world_mut();
    // A dim control is drawn and takes no interaction.
    let mut buttons = world.query::<(&Pressable, Has<Interaction>)>();
    let found: Vec<bool> = (buttons.iter(world))
        .filter(|(p, _)| **p == total)
        .map(|(_, pressable)| pressable)
        .collect();
    assert_eq!(found, [false]);
    // The model refuses it too (the act can come from elsewhere than the button), in words.
    let n = rule(&app, RuleKind::Slot, "attack.total");
    let mut editing = app.world_mut().resource_mut::<Editing>();
    let asked = editing.0.apply(&data, EditAct::RuleFile(n), 0);
    assert_eq!(asked, omnis_vector::editor::Asked::Nothing);
    assert!(editing.file.is_none());
    assert_eq!(
        editing.message.as_deref(),
        Some("base:rules:combat is the pack base's, not the one being edited")
    );
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_ne!(top_screen(&app), Some(ScreenId::EditorRules));
}

#[test]
fn a_rule_that_beat_another_is_marked_with_what_it_beat_and_its_file_opens() {
    // The pack being edited outranks one base slot.
    let scratch = scratch_pack("editor-rules-override");
    std::fs::create_dir_all(scratch.join("data/rules")).unwrap();
    std::fs::write(
        scratch.join("data/rules/house.ron"),
        r#"(schema: 1, id: "test:rules:house", slots: {
            "attack.total": (priority: 200, inputs: ["die", "bonus", "proficiency"], expr: "die + bonus"),
        })"#,
    )
    .unwrap();
    omnis_sim::omnis_data::seal::seal(&scratch, None).unwrap();
    let mut config = config();
    config.packs.pop();
    config.packs.push(scratch);
    config.editor = Some(Some("test:map:town".to_owned()));
    let mut app = shell(config, false);
    app.update();

    press_edit(&mut app, EditAct::Rules);
    let text = screen_text(app.world());
    assert!(text.contains("69 rules, 1 overriding another"), "{text}");
    assert!(text.contains("OVERRIDES  slot attack.total"), "{text}");
    assert!(text.contains("test:rules:house  priority 200"), "{text}");
    assert!(
        text.contains("over base:rules:combat  priority 0"),
        "{text}"
    );
    assert_eq!(text.matches("OVERRIDES").count(), 1, "{text}");
    // The word finds the rules that override.
    type_ui(&mut app, ControlId::Edit(EditAct::RulesFilter), "overrides");
    let text = screen_text(app.world());
    assert!(text.contains("slot attack.total"), "{text}");
    assert!(!text.contains("attack.hit"), "{text}");
    // Its file is the edited pack's: it opens as text and the list closes.
    let total = rule(&app, RuleKind::Slot, "attack.total");
    press_ui(
        &mut app,
        Pressable(ControlId::Edit(EditAct::RuleFile(total))),
    );
    assert_ne!(top_screen(&app), Some(ScreenId::EditorRules));
    let file = editing(&app).file.as_ref().expect("the rules file is open");
    assert_eq!(file.rel, "data/rules/house.ron");
}
