//! `InputPlugin`: keys and the movement pad to commands. Arrows or WASD move and turn, Q and
//! E sidestep, Space or Enter interact, M toggles the automap, F5 saves, F9 loads, Escape
//! pauses; a click on a pad button sends the same command as its key.

use crate::cursor::UiSet;
use crate::screen::WidgetId;
use crate::sim::{PlayState, PlayerCommand, ShellCommand, SimSet};
use crate::ui::UiClick;
use bevy::prelude::*;
use omnis_sim::Command;
use omnis_sim::omnis_core::{Direction, Rotation};

/// The input plugin.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_message::<UiClick>()
            .add_systems(
                Update,
                map_keys
                    .in_set(SimSet::Collect)
                    .run_if(in_state(PlayState::Explore)),
            )
            .add_systems(
                Update,
                map_pad
                    .in_set(UiSet::Dispatch)
                    .run_if(in_state(PlayState::Explore)),
            );
    }
}

/// The command a key stands for, if any.
#[must_use]
pub fn command_for(key: KeyCode) -> Option<Command> {
    Some(match key {
        KeyCode::ArrowUp | KeyCode::KeyW => Command::Step(Direction::Forward),
        KeyCode::ArrowDown | KeyCode::KeyS => Command::Step(Direction::Back),
        KeyCode::ArrowLeft | KeyCode::KeyA => Command::Turn(Rotation::Left),
        KeyCode::ArrowRight | KeyCode::KeyD => Command::Turn(Rotation::Right),
        KeyCode::KeyQ => Command::Step(Direction::Left),
        KeyCode::KeyE => Command::Step(Direction::Right),
        KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter => Command::Interact,
        _ => return None,
    })
}

/// The shell action a key stands for, if any.
#[must_use]
pub fn shell_for(key: KeyCode) -> Option<ShellCommand> {
    Some(match key {
        KeyCode::F5 => ShellCommand::Save,
        KeyCode::F9 => ShellCommand::Load,
        KeyCode::KeyM => ShellCommand::ToggleAutomap,
        KeyCode::Escape => ShellCommand::Pause,
        _ => return None,
    })
}

fn map_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: MessageWriter<PlayerCommand>,
    mut shell: MessageWriter<ShellCommand>,
) {
    for key in keys.get_just_pressed() {
        if let Some(command) = command_for(*key) {
            commands.write(PlayerCommand(command));
        } else if let Some(action) = shell_for(*key) {
            shell.write(action);
        }
    }
}

fn map_pad(mut clicks: MessageReader<UiClick>, mut commands: MessageWriter<PlayerCommand>) {
    for UiClick(hit) in clicks.read() {
        if let WidgetId::Pad(button) = hit.id {
            commands.write(PlayerCommand(button.command()));
        }
    }
}
