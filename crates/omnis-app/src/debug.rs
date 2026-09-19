//! `DebugPlugin` (feature `devtools`): the debug menu over the world, opened from the pause
//! overlay's item or with the backtick while exploring or fighting (no function key: macOS
//! takes them). Keys and clicks drive the state machine in `debug_menu.rs`; its intents are
//! `Dev` commands for the simulation, which a release world refuses. Headless-capable.

use crate::cursor::UiSet;
use crate::debug_menu::{DebugIntent, debug_view};
use crate::menu::MenuKey;
use crate::menus::{Active, Screens, Where, menu_key};
use crate::screen::{self, Target};
use crate::sim::{PackData, PlayState, PlayerCommand, SimWorld};
use crate::ui::UiClick;
use crate::widget::Hit;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use omnis_sim::Command;

/// The debug plugin.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<KeyboardInput>()
            .add_message::<UiClick>()
            .add_systems(OnEnter(PlayState::Debug), open_debug)
            .add_systems(Update, debug_keys.in_set(UiSet::Dispatch))
            .add_systems(Update, debug_model.in_set(UiSet::Model));
    }
}

/// The menu opens on the world as it is, whichever door was used.
fn open_debug(
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
) {
    if let Some((world, data)) = world.as_ref().zip(data.as_ref()) {
        let view = debug_view(&world.0, &data.0);
        screens.debug.open(&view);
    }
}

/// Whether a key press opens (or closes) the menu: the backtick.
fn toggles(input: &KeyboardInput) -> bool {
    input.state == ButtonState::Pressed
        && matches!(&input.logical_key, Key::Character(text) if text.as_str() == "`")
}

/// Keys and clicks: the toggle while exploring or fighting, the menu's keys while it is up.
#[allow(clippy::too_many_arguments)]
fn debug_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<UiClick>,
    at: Where,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut player: MessageWriter<PlayerCommand>,
    mut next: ResMut<NextState<PlayState>>,
) {
    let inputs: Vec<KeyboardInput> = keys.read().cloned().collect();
    let hits: Vec<Hit> = clicks.read().map(|c| c.0).collect();
    let active = at.screen();
    let Some((world, data)) = world.as_ref().zip(data.as_ref()) else {
        return;
    };
    let view = debug_view(&world.0, &data.0);
    match active {
        Active::None | Active::Combat if at.playing() => {
            if inputs.iter().any(toggles) {
                next.set(PlayState::Debug);
            }
        }
        Active::Debug => {
            let mut pressed: Vec<MenuKey> = inputs.iter().filter_map(menu_key).collect();
            if inputs.iter().any(toggles) {
                pressed.push(MenuKey::Escape);
            }
            for hit in hits {
                pressed.extend(screen::click(Target::Debug(&mut screens.debug), hit));
            }
            for key in pressed {
                match screens.debug.key(key, &view) {
                    Some(DebugIntent::Command(command)) => {
                        player.write(PlayerCommand(Command::Dev(command)));
                    }
                    Some(DebugIntent::Close) => next.set(PlayState::for_mode(&world.0.mode)),
                    None => {}
                }
            }
        }
        _ => {}
    }
}

/// The menu's selectors follow the world (a member who left, a stack that fell).
fn debug_model(
    at: Where,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
) {
    if at.screen() != Active::Debug {
        return;
    }
    if let Some((world, data)) = world.as_ref().zip(data.as_ref()) {
        let view = debug_view(&world.0, &data.0);
        screens.debug.sync(&view);
    }
}
