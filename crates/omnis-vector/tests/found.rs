//! What an object gives is taken through the loot window (owner, 2026-11-13): the engine lays
//! it on the ground and the window opens on it at once, so a found item is not left behind
//! for want of looking. On the yard fixture: the loose stone at (0, 2) hides a gem.

use crate::common;
use crate::loot::yard_app;

use bevy::prelude::*;
use common::app::{press_ui, tap, top};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Verb;
use omnis_sim::{Command, ItemCommand};
use omnis_vector::screen::loot::LootAct;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::aim::PickUp;
use omnis_vector::shell::controls::Control;
use omnis_vector::shell::screen::Pressable;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Session;

const GEM: &str = "base:item:gem";

fn take_the_stone() -> Command {
    Command::Act {
        object: "stone_1".into(),
        verb: Verb::Take,
        option: 0,
    }
}

fn order(app: &mut App, command: Command) {
    app.world_mut().resource_mut::<Session>().order(command);
    app.update();
    app.update();
}

fn carried(app: &App, item: &str) -> u16 {
    let s = app.world().resource::<Session>();
    s.views
        .party
        .members
        .iter()
        .flat_map(|m| &m.equipment)
        .filter(|i| i.item == item)
        .map(|i| i.count)
        .sum()
}

fn on_the_ground(app: &App) -> Vec<(String, u16)> {
    let s = app.world().resource::<Session>();
    let ground = &s.views.objects.ground;
    ground.iter().map(|i| (i.item.clone(), i.count)).collect()
}

fn pick_up_shown(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Visibility, Has<Control>), With<PickUp>>();
    let (visibility, live) = q.single(world).expect("the button");
    *visibility != Visibility::Hidden && live
}

#[test]
fn a_found_item_opens_the_loot_window_and_is_taken_or_left_as_a_heap() {
    let mut app = yard_app(0, 2, Facing::East);
    assert_eq!(top(&app), None);
    order(&mut app, take_the_stone());
    assert_eq!(top(&app), Some(ScreenId::Loot), "opened by the find");
    let text = screen_text(app.world());
    for word in ["On the ground", "Gem", "Take everything"] {
        assert!(text.contains(word), "{word}: {text}");
    }
    assert_eq!(on_the_ground(&app), [(GEM.to_owned(), 1)]);
    assert_eq!(carried(&app, GEM), 0, "nothing was taken for the party");

    // Closed without taking: it opens no second time, the gem lies as a pile, and the
    // button offers it.
    tap(&mut app, KeyCode::Escape);
    app.update();
    app.update();
    assert_eq!(top(&app), None, "closed, and not opened again");
    assert_eq!(on_the_ground(&app), [(GEM.to_owned(), 1)]);
    assert!(pick_up_shown(&mut app));
    press_ui(&mut app, PickUp);
    assert_eq!(top(&app), Some(ScreenId::Loot));
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Everything)));
    app.update();
    assert_eq!(top(&app), None, "emptied: the window closed itself");
    assert_eq!(carried(&app, GEM), 1);
    assert_eq!(app.world().resource::<Session>().binder.refusals, 0);
}

#[test]
fn only_a_find_opens_the_window_not_a_drop_or_a_pick_up() {
    let mut app = yard_app(1, 2, Facing::East);
    let member = app.world().resource::<Session>().views.party.members[0].member;
    order(
        &mut app,
        Command::Item(ItemCommand::PickUp {
            member,
            item: "base:item:arrows".into(),
            count: 2,
        }),
    );
    assert_eq!(top(&app), None, "a pick-up is not a find");
    order(
        &mut app,
        Command::Item(ItemCommand::Drop {
            member,
            item: "base:item:arrows".into(),
            count: 1,
        }),
    );
    assert_eq!(top(&app), None, "nor is a drop");
}

#[test]
fn a_find_made_from_the_actions_menu_opens_the_window_in_its_place() {
    let mut app = yard_app(0, 2, Facing::East);
    // The stone has one verb, so the actions menu takes it directly and closes.
    tap(&mut app, KeyCode::Space);
    assert_eq!(top(&app), Some(ScreenId::Actions));
    let text = screen_text(app.world());
    assert!(text.contains("\"1  Use\""), "{text}");
    press_ui(&mut app, Pressable(ControlId::Action(0)));
    app.update();
    assert_eq!(top(&app), Some(ScreenId::Loot), "the find's window");
    assert!(screen_text(app.world()).contains("Gem"));
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Everything)));
    app.update();
    assert_eq!(carried(&app, GEM), 1);
    assert_eq!(top(&app), None, "back on the view");
}
