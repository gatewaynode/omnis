//! The question before a step into or out of a town service (M7 step 4b), driven as a person
//! does: an arrow key raises it, and Go and Stay are clicked through real layout and picking,
//! or answered with Enter and Escape. The panel lies inside the map at both window sizes.

mod common;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use common::feathers::{click_node, control, layout_faults, settle, ultrawide, window};
use common::{feathers_app, place, play_state, world};
use omnis_app::confirm_panel::ConfirmId;
use omnis_app::sim::{PlayState, PlayerCommand};
use omnis_app::ui::RollLog;
use omnis_app::ui_kit::PanelRoot;
use omnis_app::widget::{PadButton, Part, WidgetId};
use omnis_sim::omnis_core::Facing;
use omnis_sim::{Command, ModeKind};

/// A new game standing at the inn's open door, facing it.
fn at_the_inn_door(save: &str) -> App {
    let mut app = feathers_app(save, true);
    settle(&mut app);
    assert_eq!(play_state(&app), PlayState::Explore);
    place(&mut app, "test:map:town", 1, 2, Facing::North);
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(Command::Interact));
    settle(&mut app);
    app
}

/// A key pressed and let go, a frame each, as a keyboard sends it: the same key can be
/// pressed again.
fn press(app: &mut App, key_code: KeyCode, logical_key: Key) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code,
                logical_key: logical_key.clone(),
                state,
                text: None,
                repeat: false,
                window,
            });
        app.update();
    }
    settle(app);
}

/// The texts on the panel, if one is up.
fn panel_texts(app: &mut App) -> Vec<String> {
    let mut roots = app.world_mut().query_filtered::<Entity, With<PanelRoot>>();
    let Some(root) = roots.iter(app.world()).next() else {
        return Vec::new();
    };
    let mut texts = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = app.world().get::<Text>(entity) {
            texts.push(text.0.clone());
        }
        if let Some(children) = app.world().get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    texts.sort();
    texts
}

fn logged(app: &App, line: &str) -> bool {
    app.world()
        .resource::<RollLog>()
        .0
        .iter()
        .any(|l| l == line)
}

#[test]
fn a_step_into_the_inn_asks_and_go_and_stay_are_clicked() {
    let mut app = at_the_inn_door("confirm-pointer.ron");
    let door = world(&app).position;

    press(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    assert_eq!(play_state(&app), PlayState::Confirm);
    assert_eq!(
        panel_texts(&mut app),
        ["Enter the Inn?", "Go", "Stay"],
        "the question and its two buttons"
    );
    assert_eq!(world(&app).position, door, "the step is held");

    let stay = control(&mut app, ConfirmId::Stay);
    click_node(&mut app, stay);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert_eq!(world(&app).position, door, "Stay sends nothing");
    assert!(panel_texts(&mut app).is_empty(), "the panel is gone");

    // The pad's forward button raises the same question.
    common::click(&mut app, WidgetId::Pad(PadButton::Forward), Part::Body);
    assert_eq!(play_state(&app), PlayState::Confirm);
    let go = control(&mut app, ConfirmId::Go);
    click_node(&mut app, go);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert_eq!(world(&app).mode.kind(), ModeKind::Town);
    assert_eq!((world(&app).position.x, world(&app).position.y), (1, 1));
    assert!(logged(&app, "The party enters the Inn"));
    assert!(panel_texts(&mut app).is_empty());
}

#[test]
fn a_step_out_asks_and_escape_stays_and_enter_goes() {
    let mut app = at_the_inn_door("confirm-keys.ron");
    press(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    press(&mut app, KeyCode::Enter, Key::Enter);
    assert_eq!(world(&app).mode.kind(), ModeKind::Town, "Enter goes in");
    let inside = world(&app).position;

    // A turn inside asks nothing.
    press(&mut app, KeyCode::ArrowLeft, Key::ArrowLeft);
    assert_eq!(play_state(&app), PlayState::Explore);
    press(&mut app, KeyCode::ArrowRight, Key::ArrowRight);

    press(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    assert_eq!(play_state(&app), PlayState::Confirm);
    assert_eq!(panel_texts(&mut app), ["Go", "Leave the Inn?", "Stay"]);
    press(&mut app, KeyCode::Escape, Key::Escape);
    assert_eq!(
        play_state(&app),
        PlayState::Explore,
        "Escape stays, and does not pause"
    );
    assert_eq!(world(&app).position, inside);
    assert_eq!(world(&app).mode.kind(), ModeKind::Town);

    press(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    press(&mut app, KeyCode::Enter, Key::Enter);
    assert_eq!(world(&app).mode.kind(), ModeKind::Explore);
    assert_eq!((world(&app).position.x, world(&app).position.y), (1, 2));
    assert!(logged(&app, "The party leaves the Inn"));
}

#[test]
fn the_question_lies_inside_the_map_at_both_window_sizes() {
    let mut app = at_the_inn_door("confirm-layout.ron");
    press(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    assert_eq!(layout_faults(&mut app), Vec::new(), "1280 by 720");
    ultrawide(&mut app);
    assert_eq!(play_state(&app), PlayState::Confirm);
    assert_eq!(layout_faults(&mut app), Vec::new(), "5120 by 1440");
}
