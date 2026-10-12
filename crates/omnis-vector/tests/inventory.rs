//! The inventory (parity P5): the words for the kit and its refusals, and the HUD lines for what
//! the party does with its items. Real packs, the fixed party, the town's start.

use crate::common;
use common::ctx;

use bevy::input::ButtonState;
use bevy::prelude::*;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::{Data, EquipSlot};
use omnis_sim::{Command, Event, ItemCommand, Rejection};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::screen::ScreenId;
use omnis_vector::screen::inventory::ItemAct;
use omnis_vector::screen::nav::Nav;
use omnis_vector::screen::roster::Tool;
use omnis_vector::screen::{self, Control, ControlId, Effect, Forms, Payload, Refused, Screen};
use omnis_vector::shell::screen::{Beside, Pressable, Screens, Side};
use omnis_vector::shell::session::Session;
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;

/// Each command's HUD lines, in order, applied to `world`.
fn lines(world: &mut SimWorld, data: &Data, commands: Vec<ItemCommand>) -> Vec<String> {
    let mut names = Names::default();
    names.observe(&common::views(world, data).party);
    let mut events: Vec<Event> = Vec::new();
    for command in commands {
        events.extend(world.apply(data, Command::Item(command)).expect("accepted"));
    }
    events
        .iter()
        .filter_map(|e| text::describe(e, &names, data))
        .collect()
}

#[test]
fn the_kit_s_events_read_as_the_other_client_s_lines() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let (brenna, durin, pip) = (
        common::id(&world, 0),
        common::id(&world, 1),
        common::id(&world, 3),
    );
    let potion = data
        .registry
        .items
        .get("base:item:potion_of_healing")
        .unwrap();
    world.party.members[1].equipment.push((potion, 1));
    world.party.members[0].hp = 2;
    let item = |id: &str| labels::item(&data, id).to_owned();
    let (sword, mail, bolts) = (
        "base:item:longsword",
        "base:item:chain_mail",
        "base:item:crossbow_bolts",
    );
    let got = lines(
        &mut world,
        &data,
        vec![
            ItemCommand::Unequip {
                member: brenna,
                slot: EquipSlot::MainHand,
            },
            ItemCommand::Equip {
                member: brenna,
                item: sword.into(),
            },
            ItemCommand::Unequip {
                member: brenna,
                slot: EquipSlot::Body,
            },
            ItemCommand::Equip {
                member: brenna,
                item: mail.into(),
            },
            ItemCommand::Stow {
                member: brenna,
                item: bolts.into(),
                count: 1,
            },
            ItemCommand::Take {
                member: pip,
                item: bolts.into(),
                count: 1,
            },
            ItemCommand::Give {
                giver: brenna,
                receiver: pip,
                item: bolts.into(),
                count: 2,
            },
            ItemCommand::Use {
                member: durin,
                item: "base:item:potion_of_healing".into(),
                receiver: Some(brenna),
            },
        ],
    );
    let expected = [
        format!("Brenna takes off {}", item(sword)),
        format!("Brenna wields {}", item(sword)),
        format!("Brenna takes off {}", item(mail)),
        format!("Brenna wears {}", item(mail)),
        format!("Brenna stows {}", item(bolts)),
        format!("Pip takes {}", item(bolts)),
        format!("Brenna gives 2 {} to Pip", item(bolts)),
        format!(
            "Durin uses {} on Brenna",
            item("base:item:potion_of_healing")
        ),
    ];
    assert_eq!(got[..expected.len()], expected);
    let healed = &got[expected.len()];
    assert!(
        healed.starts_with("Brenna regains ") && healed.contains(" hp, now "),
        "{healed}"
    );
}

/// A refusal names the member, where the engine's words give a number.
#[test]
fn a_refusal_reads_in_a_few_words_with_the_member_s_name() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let party = common::views(&world, &data).party;
    let brenna = common::id(&world, 0);
    let reason = |r: Rejection| labels::reason(&party, &r);
    assert_eq!(
        reason(Rejection::MemberDown { member: brenna }),
        "Brenna is down"
    );
    assert_eq!(
        reason(Rejection::TargetDead { member: brenna }),
        "Brenna is dead"
    );
    assert_eq!(reason(Rejection::HandsFull), "no hand free for a shield");
    assert_eq!(
        reason(Rejection::NotEquippable),
        "cannot be worn or wielded"
    );
    assert_eq!(
        reason(Rejection::NotEnoughPoints { need: 2, have: 0 }),
        "need 2 pt"
    );
    assert_eq!(
        reason(Rejection::MissingComponents {
            spell: "base:spell:x".into()
        }),
        "needs components"
    );
    assert_eq!(
        reason(Rejection::SameMember),
        "a member cannot exchange with themselves"
    );
    assert_eq!(labels::slot(EquipSlot::OffHand), "Off hand");
    assert_eq!(labels::count_label("Rations", 5), "Rations x5");
    assert_eq!(labels::count_label("Rations", 1), "Rations");
}

// The screen (P5b): the model, then the shell.

/// The inventory of `owner` with `selected` chosen in the column.
fn forms(owner: CharacterId, selected: Option<CharacterId>) -> Forms {
    let mut forms = Forms::default();
    forms.owner = Some(owner);
    forms.selected = selected;
    forms
}

/// The inventory as it stands for `forms`.
fn shown(data: &Data, world: &SimWorld, forms: &Forms) -> Screen {
    let views = common::views(world, data);
    screen::content(ScreenId::Inventory, &ctx(data, world, &views), forms)
}

/// Use control `id` of the inventory.
fn press(
    data: &Data,
    world: &SimWorld,
    forms: &mut Forms,
    id: ControlId,
) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    let ctx = ctx(data, world, &views);
    screen::apply(ScreenId::Inventory, &ctx, forms, id, &Payload::Press, 0)
}

/// The button `act` of the row labelled `label`, in the kit (panel 0) or the stores (1).
fn button<'s>(shown: &'s Screen, panel: usize, label: &str, act: ItemAct) -> &'s Control {
    shown.panels[panel]
        .rows
        .iter()
        .find(|r| r.spans.first().is_some_and(|s| s.text == label))
        .and_then(|r| {
            r.controls
                .iter()
                .find(|c| matches!(c.id, ControlId::Item { act: a, .. } if a == act))
        })
        .unwrap_or_else(|| panic!("{label}'s {act:?} in panel {panel}: {shown:?}"))
}

#[test]
fn the_kit_and_the_stores_list_what_the_views_hold() {
    let data = common::data();
    let (world, _) = common::town_session(&data);
    let brenna = common::id(&world, 0);
    let s = shown(&data, &world, &forms(brenna, None));
    let views = common::views(&world, &data);
    let me = &views.party.members[0];
    let worn: Vec<&str> = EquipSlot::ALL
        .iter()
        .filter_map(|slot| me.worn.iter().find(|(w, _)| w == slot))
        .map(|(_, id)| labels::item(&data, id))
        .collect();
    let kit = &s.panels[0].rows;
    assert_eq!(kit[1].spans[0].text, format!("wearing {}", worn.join(", ")));
    assert_eq!(kit[2].spans[0].text, "Kit");
    let rows = &kit[3..];
    assert_eq!(rows.len(), me.equipment.len());
    for (row, item) in rows.iter().zip(&me.equipment) {
        assert_eq!(
            row.spans[0].text,
            labels::count_label(labels::item(&data, &item.item), item.count)
        );
    }
    let sword = labels::item(&data, "base:item:longsword");
    let sword_row = rows.iter().find(|r| r.spans[0].text == sword).unwrap();
    assert_eq!(sword_row.after[0].text, "Main hand  worn");
    assert_eq!(button(&s, 0, sword, ItemAct::Unequip).label, "Take off");
    let stores = &s.panels[1].rows;
    assert_eq!(stores[0].spans[0].text, "Stores");
    assert_eq!(
        stores[1].spans[0].text,
        format!(
            "{}  food {}",
            labels::coins(views.party.gold),
            views.party.food
        )
    );
    if views.party.inventory.is_empty() {
        assert_eq!(stores[2].spans[0].text, "Nothing in the stores");
    }
}

/// Every button sends exactly its command; a live one the engine takes, a dim one carries the
/// trial's reason or the column's hint.
#[test]
fn every_live_button_is_taken_and_every_dim_one_says_why() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let potion = data
        .registry
        .items
        .get("base:item:potion_of_healing")
        .unwrap();
    world.party.members[1].equipment.push((potion, 2));
    let bolts = data.registry.items.get("base:item:crossbow_bolts").unwrap();
    world.party.inventory.push((bolts, 3));
    let (mut live, mut dim) = (0, 0);
    for owner in 0..4 {
        let mut f = forms(
            common::id(&world, owner),
            Some(common::id(&world, (owner + 1) % 4)),
        );
        let s = shown(&data, &world, &f);
        for control in s
            .controls()
            .filter(|c| matches!(c.id, ControlId::Item { .. }))
        {
            let used = press(&data, &world, &mut f, control.id);
            match &control.why_not {
                None => {
                    live += 1;
                    let Ok(Effect::Command {
                        command,
                        close: false,
                    }) = used
                    else {
                        panic!("{control:?} sends a command: {used:?}");
                    };
                    assert!(world.trial(&data, &command).is_ok(), "{command:?} is taken");
                }
                Some(why) => {
                    dim += 1;
                    assert_eq!(used, Err(Refused::Dim(why.clone())));
                }
            }
        }
    }
    assert!(live > 20, "{live} live, {dim} dim");
}

#[test]
fn give_goes_to_the_member_selected_in_the_column() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let (brenna, pip) = (common::id(&world, 0), common::id(&world, 3));
    let bolts = labels::item(&data, "base:item:crossbow_bolts");
    let label = labels::count_label(bolts, 20);
    let why = |selected| {
        button(
            &shown(&data, &world, &forms(brenna, selected)),
            0,
            &label,
            ItemAct::Give,
        )
        .why_not
        .clone()
    };
    assert_eq!(
        why(None).as_deref(),
        Some("Select a member in the party column")
    );
    assert_eq!(
        why(Some(brenna)).as_deref(),
        Some("Select another member in the party column")
    );
    assert_eq!(why(Some(pip)), None);
    let mut f = forms(brenna, Some(pip));
    let id = button(&shown(&data, &world, &f), 0, &label, ItemAct::Give).id;
    let Ok(Effect::Command { command, .. }) = press(&data, &world, &mut f, id) else {
        panic!("Give sends a command");
    };
    assert_eq!(
        command,
        Command::Item(ItemCommand::Give {
            giver: brenna,
            receiver: pip,
            item: "base:item:crossbow_bolts".into(),
            count: 1,
        })
    );
    world.apply(&data, command).expect("given");
    let views = common::views(&world, &data);
    assert!(
        views.party.members[3]
            .equipment
            .iter()
            .any(|i| i.item == "base:item:crossbow_bolts" && i.count == 1),
        "Pip (who carries arrows) holds one bolt"
    );
}

/// Take fills the kit shown, whoever is selected; the owner choice steps through the party; a
/// potion goes to the selected member, or with none to the user.
#[test]
fn take_fills_the_kit_shown_and_a_potion_goes_to_the_selection() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let (brenna, durin, pip) = (
        common::id(&world, 0),
        common::id(&world, 1),
        common::id(&world, 3),
    );
    let bolts = data.registry.items.get("base:item:crossbow_bolts").unwrap();
    world.party.inventory.push((bolts, 3));
    let potion = data
        .registry
        .items
        .get("base:item:potion_of_healing")
        .unwrap();
    world.party.members[1].equipment.push((potion, 1));
    let label = labels::count_label(labels::item(&data, "base:item:crossbow_bolts"), 3);

    let mut f = forms(pip, Some(brenna));
    let id = button(&shown(&data, &world, &f), 1, &label, ItemAct::Take).id;
    let take = press(&data, &world, &mut f, id);
    let Ok(Effect::Command {
        command: Command::Item(ItemCommand::Take { member, .. }),
        ..
    }) = take
    else {
        panic!("Take sends Take: {take:?}");
    };
    assert_eq!(member, pip, "the kit shown, not the selection");

    let mut f = forms(brenna, None);
    let views = common::views(&world, &data);
    let ctx = ctx(&data, &world, &views);
    screen::apply(
        ScreenId::Inventory,
        &ctx,
        &mut f,
        ControlId::Owner,
        &Payload::Right,
        0,
    )
    .unwrap();
    assert_eq!(f.owner, Some(durin), "Right steps to the next member");
    screen::apply(
        ScreenId::Inventory,
        &ctx,
        &mut f,
        ControlId::Owner,
        &Payload::Left,
        0,
    )
    .unwrap();
    assert_eq!(f.owner, Some(brenna));

    let drink = labels::item(&data, "base:item:potion_of_healing");
    for (selected, receiver) in [(None, None), (Some(brenna), Some(brenna))] {
        let mut f = forms(durin, selected);
        let id = button(&shown(&data, &world, &f), 0, drink, ItemAct::Use).id;
        let used = press(&data, &world, &mut f, id);
        assert_eq!(
            used,
            Ok(Effect::Command {
                command: Command::Item(ItemCommand::Use {
                    member: durin,
                    item: "base:item:potion_of_healing".into(),
                    receiver,
                }),
                close: false
            })
        );
    }
}

/// A member who is down cannot wear or use anything, and the button says who.
#[test]
fn a_member_down_has_their_equip_dim_with_their_name() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let brenna = common::id(&world, 0);
    world.party.members[0].hp = 0;
    let s = shown(&data, &world, &forms(brenna, None));
    let sword = labels::item(&data, "base:item:longsword");
    assert_eq!(
        button(&s, 0, sword, ItemAct::Unequip).why_not.as_deref(),
        Some("Brenna is down")
    );
    assert_eq!(
        button(&s, 0, sword, ItemAct::Stow).why_not,
        None,
        "a down member's kit still moves"
    );
}

#[test]
fn in_a_fight_the_inventory_only_looks() {
    let session = common::fight::met();
    let views = &session.views;
    assert!(views.combat.is_some(), "monsters met");
    let ctx = omnis_vector::shell::screen::ctx(&session);
    let s = screen::content(
        ScreenId::Inventory,
        &ctx,
        &forms(views.party.members[0].member, None),
    );
    assert!(
        s.controls()
            .all(|c| !matches!(c.id, ControlId::Item { .. })),
        "no buttons in a fight"
    );
    assert!(
        s.panels[0].rows.iter().any(|r| r
            .spans
            .first()
            .is_some_and(|s| s.text.starts_with("In a fight"))),
        "{s:?}"
    );
    let mut nav = Nav::default();
    nav.open(ScreenId::Inventory, Some(views));
    assert_eq!(
        nav.top(Some(views)),
        Some(ScreenId::Inventory),
        "allowed in a fight, to look"
    );
    assert!(nav.paused(Some(views)), "the fight waits while it is open");
}

// The shell.

fn top(app: &App) -> Option<ScreenId> {
    let session = app.world().resource::<Session>();
    app.world()
        .resource::<Screens>()
        .nav
        .top(Some(&session.views))
}

/// Press a button carrying `marker`, then let it go.
fn press_ui<C: Component + PartialEq + core::fmt::Debug + Copy>(app: &mut App, marker: C) {
    common::app::set(app, &marker, Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut q = world.query::<(&C, &mut Interaction)>();
    for (c, mut i) in q.iter_mut(world) {
        if *c == marker {
            *i = Interaction::None;
        }
    }
    app.update();
}

/// The column opens the inventory and stays beside it; a member selected there receives a give;
/// the screen reports it; Esc closes; the party does not walk while it is open.
#[test]
fn the_column_opens_the_inventory_beside_it_and_its_selection_receives() {
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.update();
    let (brenna, pip, here) = {
        let s = app.world().resource::<Session>();
        (
            s.views.party.members[0].member,
            s.views.party.members[3].member,
            s.views.position,
        )
    };
    press_ui(&mut app, Side(ControlId::Member(brenna)));
    press_ui(&mut app, Side(ControlId::Open(Tool::Inventory)));
    assert_eq!(top(&app), Some(ScreenId::Inventory));
    assert!(
        app.world().resource::<Beside>().0 > 0.0,
        "the column stands beside it"
    );

    press_ui(&mut app, Side(ControlId::Member(pip)));
    assert_eq!(app.world().resource::<Screens>().forms.selected, Some(pip));
    assert_eq!(
        top(&app),
        Some(ScreenId::Inventory),
        "selecting keeps the screen"
    );
    let give = {
        let session = app.world().resource::<Session>();
        let screens = app.world().resource::<Screens>();
        let s = screen::content(
            ScreenId::Inventory,
            &omnis_vector::shell::screen::ctx(session),
            &screens.forms,
        );
        let bolts =
            labels::count_label(labels::item(&session.data, "base:item:crossbow_bolts"), 20);
        button(&s, 0, &bolts, ItemAct::Give).id
    };
    press_ui(&mut app, Pressable(give));
    let message = app.world().resource::<Screens>().forms.message.clone();
    let text = message.map(|m| m.text).unwrap_or_default();
    assert!(text.starts_with("Brenna gives "), "{text}");

    common::app::key(&mut app, KeyCode::KeyW, ButtonState::Pressed);
    for _ in 0..30 {
        app.update();
    }
    common::app::key(&mut app, KeyCode::KeyW, ButtonState::Released);
    app.update();
    assert_eq!(
        app.world().resource::<Session>().views.position,
        here,
        "no walking"
    );
    common::app::key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    app.update();
    common::app::key(&mut app, KeyCode::Escape, ButtonState::Released);
    app.update();
    assert_eq!(top(&app), None, "Esc closes it");
    assert_eq!(app.world().resource::<Beside>().0, 0.0);
}
