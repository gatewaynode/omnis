//! The inventory's ground and containers (M8.5 step 2b item 4): each kit row drops one on the
//! party's cell; the pile there stands under the stores, each item with Pick up; an open
//! container in reach lists its items with Take out, and each kit row puts one in it. A shut
//! container shows nothing. Every button is lit by a trial of its exact command. The yard
//! fixture and the fixed party, whose rogue carries daggers.

use crate::common;

use bevy::prelude::*;
use common::app::{config, press_ui, shell, top};
use common::{ctx, objects_data, place, town_session};
use omnis_sim::omnis_core::{CharacterId, Facing};
use omnis_sim::omnis_data::{Data, Verb};
use omnis_sim::{Command, ItemCommand};
use omnis_vector::labels;
use omnis_vector::screen::ScreenId;
use omnis_vector::screen::inventory::ItemAct;
use omnis_vector::screen::roster::Tool;
use omnis_vector::screen::{self, Control, ControlId, Effect, Forms, Payload, Refused, Screen};
use omnis_vector::shell::screen::{Pressable, Side};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::{Config, Session};
use omnis_vector::sim::SimWorld;
use std::path::PathBuf;

const DAGGER: &str = "base:item:dagger";
const ARROWS: &str = "base:item:arrows";

fn yard(data: &Data, x: u16, y: u16, facing: Facing) -> SimWorld {
    let (mut world, _) = town_session(data);
    let map = common::map(data, "objects:map:yard");
    place(&mut world, map, x, y, facing);
    world
}

/// The rogue, who carries daggers and thieves' tools.
fn rogue(world: &SimWorld) -> CharacterId {
    common::id(world, 3)
}

fn forms(owner: CharacterId) -> Forms {
    let mut forms = Forms::default();
    forms.owner = Some(owner);
    forms
}

fn shown(data: &Data, world: &SimWorld, forms: &Forms) -> Screen {
    let views = common::views(world, data);
    screen::content(ScreenId::Inventory, &ctx(data, world, &views), forms)
}

fn press(data: &Data, world: &SimWorld, forms: &Forms, id: ControlId) -> Result<Effect, Refused> {
    let views = common::views(world, data);
    screen::apply(
        ScreenId::Inventory,
        &ctx(data, world, &views),
        &mut forms.clone(),
        id,
        &Payload::Press,
        0,
    )
}

/// The second panel's rows as words, a control as "[label]" or "[label: reason]".
fn beside(screen: &Screen) -> Vec<String> {
    screen.panels[1]
        .rows
        .iter()
        .map(|r| {
            let mut words: Vec<String> = r.spans.iter().map(|s| s.text.clone()).collect();
            words.extend(r.controls.iter().map(|c| match &c.why_not {
                Some(why) => format!("[{}: {why}]", c.label),
                None => format!("[{}]", c.label),
            }));
            words.join("  ")
        })
        .collect()
}

/// The button `act` of the row labelled `label` in `panel`.
fn button<'s>(shown: &'s Screen, panel: usize, label: &str, act: ItemAct) -> &'s Control {
    shown.panels[panel]
        .rows
        .iter()
        .filter(|r| r.spans.first().is_some_and(|s| s.text == label))
        .find_map(|r| {
            r.controls
                .iter()
                .find(|c| matches!(c.id, ControlId::Item { act: a, .. } if a == act))
        })
        .unwrap_or_else(|| panic!("{label}'s {act:?} in panel {panel}: {shown:?}"))
}

/// Whether any control of `screen` is `act`.
fn offers(screen: &Screen, act: ItemAct) -> bool {
    screen
        .controls()
        .any(|c| matches!(c.id, ControlId::Item { act: a, .. } if a == act))
}

/// The command a live button sends; the screen stays open.
fn sends(data: &Data, world: &SimWorld, forms: &Forms, control: &Control) -> Command {
    assert_eq!(control.why_not, None, "{} is dim", control.label);
    match press(data, world, forms, control.id) {
        Ok(Effect::Command {
            command,
            close: false,
        }) => command,
        other => panic!("{}: {other:?}", control.label),
    }
}

/// Every live item button's command is one the engine accepts; a dim one is refused with its
/// reason.
fn live_ones_are_accepted(data: &Data, world: &SimWorld, forms: &Forms) {
    let screen = shown(data, world, forms);
    for c in screen
        .controls()
        .filter(|c| matches!(c.id, ControlId::Item { .. }))
    {
        match press(data, world, forms, c.id) {
            Ok(Effect::Command { command, .. }) => {
                assert!(c.why_not.is_none(), "{} is dim", c.label);
                assert!(
                    world.trial(data, &command).is_ok(),
                    "{}: {command:?}",
                    c.label
                );
            }
            Err(Refused::Dim(why)) => assert_eq!(Some(why), c.why_not.clone()),
            other => panic!("{}: {other:?}", c.label),
        }
    }
}

/// Open the chest by picking its lock; a failed roll leaves it shut to try again.
fn open_chest(data: &Data, world: &mut SimWorld) {
    for _ in 0..50 {
        let _ = world.apply(
            data,
            Command::Act {
                object: "chest_1".to_owned(),
                verb: Verb::Open,
                option: 1,
            },
        );
        let views = common::views(world, data);
        let chest = views.objects.objects.iter().find(|o| o.object == "chest_1");
        if chest.is_some_and(|c| c.contents.is_some()) {
            return;
        }
    }
    panic!("the lock never gave");
}

#[test]
fn the_pile_underfoot_stands_under_the_stores_and_each_item_picks_up_one() {
    let data = objects_data();
    let mut world = yard(&data, 2, 2, Facing::North);
    let f = forms(rogue(&world));
    let s = shown(&data, &world, &f);
    let arrows = labels::item(&data, ARROWS);
    let rows = beside(&s);
    let at = rows
        .iter()
        .position(|r| r == "On the ground")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[at + 1], format!("{arrows} x10  [Pick up]"));
    assert_eq!(rows.len(), at + 2, "nothing else under it: {rows:?}");
    let pick = button(&s, 1, &format!("{arrows} x10"), ItemAct::PickUp);
    assert_eq!(
        sends(&data, &world, &f, pick),
        Command::Item(ItemCommand::PickUp {
            member: rogue(&world),
            item: ARROWS.to_owned(),
            count: 1,
        })
    );
    live_ones_are_accepted(&data, &world, &f);
    world
        .apply(&data, sends(&data, &world, &f, pick))
        .expect("picked up");
    assert!(beside(&shown(&data, &world, &f)).contains(&format!("{arrows} x9  [Pick up]")));
}

#[test]
fn a_kit_row_drops_one_and_the_pile_appears_where_there_was_none() {
    let data = objects_data();
    let mut world = yard(&data, 1, 2, Facing::North);
    let f = forms(rogue(&world));
    let s = shown(&data, &world, &f);
    assert!(
        !beside(&s).iter().any(|r| r == "On the ground"),
        "no pile here: {:?}",
        beside(&s)
    );
    let dagger = labels::item(&data, DAGGER);
    let drop = button(&s, 0, &format!("{dagger} x2"), ItemAct::Drop);
    assert_eq!(drop.label, "Drop");
    let command = sends(&data, &world, &f, drop);
    assert_eq!(
        command,
        Command::Item(ItemCommand::Drop {
            member: rogue(&world),
            item: DAGGER.to_owned(),
            count: 1,
        })
    );
    world.apply(&data, command).expect("dropped");
    let rows = beside(&shown(&data, &world, &f));
    let at = rows.iter().position(|r| r == "On the ground").unwrap();
    assert_eq!(rows[at + 1], format!("{dagger}  [Pick up]"));
    live_ones_are_accepted(&data, &world, &f);
}

#[test]
fn an_open_chest_lists_its_items_with_take_out_and_the_kit_puts_in() {
    let data = objects_data();
    let mut world = yard(&data, 5, 4, Facing::North);
    let f = forms(rogue(&world));
    // Shut: nothing of it here, and nothing to put in.
    let s = shown(&data, &world, &f);
    assert!(!offers(&s, ItemAct::PutIn(0)), "{s:?}");
    assert!(!beside(&s).iter().any(|r| r == "Chest"), "{:?}", beside(&s));
    open_chest(&data, &mut world);
    let s = shown(&data, &world, &f);
    let dagger = labels::item(&data, DAGGER);
    let rows = beside(&s);
    let at = rows
        .iter()
        .position(|r| r == "Chest")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[at + 1], format!("{dagger}  [Take out]"));
    live_ones_are_accepted(&data, &world, &f);
    let take = button(&s, 1, dagger, ItemAct::TakeOut(0));
    let command = sends(&data, &world, &f, take);
    assert_eq!(
        command,
        Command::Item(ItemCommand::TakeOut {
            member: rogue(&world),
            object: "chest_1".to_owned(),
            item: DAGGER.to_owned(),
            count: 1,
        })
    );
    world.apply(&data, command).expect("taken out");
    let s = shown(&data, &world, &f);
    let rows = beside(&s);
    let at = rows.iter().position(|r| r == "Chest").unwrap();
    assert_eq!(rows[at + 1], "Empty");
    // The kit's daggers, three now, go back in one at a time.
    let put = button(&s, 0, &format!("{dagger} x3"), ItemAct::PutIn(0));
    assert_eq!(put.label, "Put in Chest");
    let command = sends(&data, &world, &f, put);
    assert_eq!(
        command,
        Command::Item(ItemCommand::PutIn {
            member: rogue(&world),
            object: "chest_1".to_owned(),
            item: DAGGER.to_owned(),
            count: 1,
        })
    );
    world.apply(&data, command).expect("put in");
    let rows = beside(&shown(&data, &world, &f));
    assert!(rows.contains(&format!("{dagger}  [Take out]")), "{rows:?}");
}

#[test]
fn in_a_town_the_inventory_has_no_ground_or_container_and_offers_drop() {
    let data = common::data();
    let (world, _) = town_session(&data);
    let f = forms(rogue(&world));
    let s = shown(&data, &world, &f);
    assert!(offers(&s, ItemAct::Drop));
    assert!(!offers(&s, ItemAct::PickUp));
    assert!(!offers(&s, ItemAct::TakeOut(0)));
    assert!(!offers(&s, ItemAct::PutIn(0)));
    assert!(!beside(&s).iter().any(|r| r == "On the ground"));
}

#[test]
fn in_the_window_pick_up_takes_one_from_the_pile_and_the_line_says_so() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let mut app = shell(Config { packs, ..config() }, true);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = place(&mut session.world, map, 2, 2, Facing::North);
        session.refresh();
    }
    app.update();
    // The column shows a member's tools once one is selected.
    let first = app.world().resource::<Session>().views.party.members[0].member;
    press_ui(&mut app, Side(ControlId::Member(first)));
    press_ui(&mut app, Side(ControlId::Open(Tool::Inventory)));
    assert_eq!(top(&app), Some(ScreenId::Inventory));
    let text = screen_text(app.world());
    assert!(text.contains("\"On the ground\""), "{text}");
    let ground = app
        .world()
        .resource::<Session>()
        .views
        .objects
        .ground
        .clone();
    assert_eq!(ground[0].item, ARROWS);
    press_ui(
        &mut app,
        Pressable(ControlId::Item {
            at: 0,
            act: ItemAct::PickUp,
        }),
    );
    let s = app.world().resource::<Session>();
    assert_eq!(s.views.objects.ground[0].count, 9);
    assert!(
        s.lines.iter().any(|l| l.contains("picks up")),
        "{:?}",
        s.lines
    );
    assert_eq!(s.binder.refusals, 0);
    assert_eq!(
        top(&app),
        Some(ScreenId::Inventory),
        "the screen stays open"
    );
}

#[test]
fn a_full_chest_dims_put_in_with_its_reason_worded_short() {
    let data = objects_data();
    let mut world = yard(&data, 5, 4, Facing::North);
    open_chest(&data, &mut world);
    let held = |world: &SimWorld| {
        let views = common::views(world, &data);
        let chest = views.objects.objects.iter().find(|o| o.object == "chest_1");
        chest.and_then(|c| c.contents.clone()).unwrap_or_default()
    };
    // Fill its eight kinds from the party's kits.
    for slot in 0..4 {
        let member = common::id(&world, slot);
        let kit = common::views(&world, &data).party.members[usize::from(slot)]
            .equipment
            .clone();
        for item in kit {
            if held(&world).len() == 8 || held(&world).iter().any(|i| i.item == item.item) {
                continue;
            }
            let _ = world.apply(
                &data,
                Command::Item(ItemCommand::PutIn {
                    member,
                    object: "chest_1".to_owned(),
                    item: item.item.clone(),
                    count: 1,
                }),
            );
        }
    }
    assert_eq!(held(&world).len(), 8, "{:?}", held(&world));
    // A kind it does not hold has no room.
    let (owner, label) = (0..4u8)
        .find_map(|slot| {
            let views = common::views(&world, &data);
            let member = &views.party.members[usize::from(slot)];
            let item = member
                .equipment
                .iter()
                .find(|i| !held(&world).iter().any(|h| h.item == i.item))?;
            Some((
                member.member,
                labels::count_label(labels::item(&data, &item.item), item.count),
            ))
        })
        .expect("a kind the chest lacks");
    let f = forms(owner);
    let s = shown(&data, &world, &f);
    assert_eq!(
        button(&s, 0, &label, ItemAct::PutIn(0)).why_not.as_deref(),
        Some("it is full")
    );
    live_ones_are_accepted(&data, &world, &f);
}
