//! The party column in the app (presentation-ARCHITECTURE §6.6): it shows while a game is
//! played and no screen is up, at the right in explore and in a fight; pressing a member's
//! entry selects them and shows the row of their screens. The description itself is tested in
//! `roster.rs`.

use crate::common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use common::app::{app_at_title, app_in_game, key, met_with, set};
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::roster::RosterRoot;
use omnis_vector::shell::screen::{Pressable, Screens, Side};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Session;

struct Scratch(String);

impl Scratch {
    fn new(name: &str) -> Scratch {
        Scratch(format!(".omnis/test-saves/column-{name}.ron"))
    }
}

/// Whether the column is drawn.
fn shown(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut root = world.query_filtered::<&Node, With<RosterRoot>>();
    root.single(world).expect("the column's node").display != Display::None
}

fn press_side(app: &mut App, id: ControlId) {
    set(app, &Side(id), Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut buttons = world.query::<(&Side, &mut Interaction)>();
    for (s, mut i) in buttons.iter_mut(world) {
        if s.0 == id {
            *i = Interaction::None;
        }
    }
    app.update();
}

fn press(app: &mut App, id: ControlId) {
    set(app, &Pressable(id), Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut buttons = world.query::<(&Pressable, &mut Interaction)>();
    for (p, mut i) in buttons.iter_mut(world) {
        if p.0 == id {
            *i = Interaction::None;
        }
    }
    app.update();
}

fn tap(app: &mut App, code: KeyCode) {
    key(app, code, ButtonState::Pressed);
    app.update();
    key(app, code, ButtonState::Released);
    app.update();
}

fn top_is_none(app: &App) -> bool {
    let game = app.world().get_resource::<Session>().map(|s| &s.views);
    app.world().resource::<Screens>().nav.top(game).is_none()
}

#[test]
fn the_column_lists_the_party_and_a_member_opens_their_row() {
    let save = Scratch::new("explore");
    let mut app = app_in_game(&save.0);
    app.update();
    app.update();
    assert!(shown(&mut app), "the column shows while exploring");
    let text = screen_text(app.world());
    let members = app
        .world()
        .resource::<Session>()
        .views
        .party
        .members
        .clone();
    for m in &members {
        assert!(text.contains(&m.name), "{} in {text}", m.name);
    }
    assert!(!text.contains("Sheet"), "no row until a member is selected");
    press_side(&mut app, ControlId::Member(members[1].member));
    assert_eq!(
        app.world().resource::<Screens>().forms.selected,
        Some(members[1].member)
    );
    let text = screen_text(app.world());
    assert!(
        text.contains("Sheet") && !text.contains("Sheet:"),
        "the sheet is live: {text}"
    );
    assert!(text.contains("Journal: No quests yet"), "{text}");
    press_side(&mut app, ControlId::Member(members[1].member));
    assert_eq!(app.world().resource::<Screens>().forms.selected, None);
}

#[test]
fn the_column_hides_at_the_title_on_creation_and_under_the_pause() {
    let save = Scratch::new("hidden");
    let mut app = app_at_title(&save.0);
    app.update();
    app.update();
    assert!(!shown(&mut app), "no game at the title");
    press(&mut app, ControlId::NewGame);
    press(&mut app, ControlId::Start);
    let game = app.world().get_resource::<Session>().map(|s| &s.views);
    assert_eq!(
        app.world().resource::<Screens>().nav.top(game),
        Some(ScreenId::CreateParty)
    );
    assert!(!shown(&mut app), "not while the party is made");

    let save = Scratch::new("paused");
    let mut app = app_in_game(&save.0);
    app.update();
    app.update();
    let first = app.world().resource::<Session>().views.party.members[0].member;
    press_side(&mut app, ControlId::Member(first));
    tap(&mut app, KeyCode::Escape);
    let game = app.world().get_resource::<Session>().map(|s| &s.views);
    assert_eq!(
        app.world().resource::<Screens>().nav.top(game),
        Some(ScreenId::Pause)
    );
    assert!(!shown(&mut app), "the pause covers the game");
    // A load starts over, with the same members back: nothing stays selected.
    press(&mut app, ControlId::Save);
    press(&mut app, ControlId::Load);
    assert!(top_is_none(&app), "the save loaded and the pause closed");
    assert_eq!(app.world().resource::<Screens>().forms.selected, None);
    let _ = std::fs::remove_file(&save.0);
}

#[test]
fn the_column_stands_beside_a_fight() {
    let mut app = met_with(1);
    app.update();
    assert!(shown(&mut app), "the column shows in a fight");
    let text = screen_text(app.world());
    let name = app.world().resource::<Session>().views.party.members[0]
        .name
        .clone();
    assert!(text.contains(&name), "{text}");
}
