//! The party column in the app (presentation-ARCHITECTURE §6.6): it shows while a game is
//! played and no screen is up, at the right in explore and in a fight; pressing a member's
//! entry selects them and shows the row of their screens. The description itself is tested in
//! `roster.rs`.

use crate::common;
use crate::common::app::Pointing;

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
    set(app, &Side(id), Pointing::Press);
    app.update();
    crate::common::app::let_go(app, &Side(id));
    app.update();
}

fn press(app: &mut App, id: ControlId) {
    set(app, &Pressable(id), Pointing::Press);
    app.update();
    crate::common::app::let_go(app, &Pressable(id));
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

/// A click with the pointer on a button is the button's: it does not also take the view's
/// mouse look. Off the buttons the same click does.
#[test]
fn a_click_on_a_button_is_not_a_click_in_the_view() {
    use common::editor::mouse;
    use omnis_vector::shell::controls::Action;
    use omnis_vector::shell::movement::Intent;
    let save = Scratch::new("over");
    let mut app = app_in_game(&save.0);
    // Mouse look holds the window's cursor: headless there is no window until one is made.
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    app.update();
    let looking = |app: &App| app.world().resource::<Intent>().looking;
    assert!(!looking(&app));
    set(&mut app, &Action::Log, Pointing::Hover);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert!(!looking(&app), "the pointer was on a button");
    set(&mut app, &Action::Log, Pointing::Rest);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert!(looking(&app), "a click in the view takes the mouse look");
}

/// The owner's report (2026-11-12, after the move to Bevy 0.20): the left button stopped
/// working in play. A button that opens a screen over itself is pressed, and the pointer's
/// release lands on the screen, not on the button: the button stayed pressed, and a pressed
/// control takes every click from the view. The button coming up lets every control go.
#[test]
fn a_button_covered_while_pressed_is_let_go_when_the_mouse_button_comes_up() {
    use bevy::ui::Pressed;
    use common::editor::mouse;
    use omnis_vector::shell::controls::Action;
    use omnis_vector::shell::movement::Intent;
    let save = Scratch::new("covered");
    let mut app = app_in_game(&save.0);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    app.update();
    // As the window does it: the button down on Menu, the pause opens over it, the button up
    // with no pointer event reaching Menu.
    set(&mut app, &Action::Menu, Pointing::Press);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    assert_eq!(
        app.world().resource::<Screens>().nav.top(None),
        Some(ScreenId::Pause)
    );
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    let held = {
        let world = app.world_mut();
        world
            .query_filtered::<(), With<Pressed>>()
            .iter(world)
            .count()
    };
    assert_eq!(held, 0, "nothing is held once the button is up");
    // Back in the view, the pointer moved off Menu (picking's part), a click is the view's.
    common::app::let_go(&mut app, &Action::Menu);
    key(&mut app, KeyCode::Escape, ButtonState::Pressed);
    app.update();
    key(&mut app, KeyCode::Escape, ButtonState::Released);
    app.update();
    assert!(!app.world().resource::<Intent>().looking);
    mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
    mouse(&mut app, MouseButton::Left, ButtonState::Released);
    assert!(
        app.world().resource::<Intent>().looking,
        "a click in the view"
    );
}
