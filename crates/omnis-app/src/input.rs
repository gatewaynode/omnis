//! `InputPlugin`: keys, the movement pad and the tool pad to commands. Arrows or WASD move
//! and turn, Q and E sidestep, Space or Enter interact, M toggles the automap, C opens the
//! cast menu, Escape pauses; a click on a pad button sends the same command as its key, and a
//! click on a tool button the same shell command (the buttons' states say when they are
//! live, so the keys are the shortcuts). No function key is bound: macOS takes them (saving
//! and loading live on the pause menu).

use crate::cursor::UiSet;
use crate::sim::{PlayState, PlayerCommand, ShellCommand, SimSet};
use crate::ui::UiClick;
use crate::widget::{ToolButton, WidgetId};
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
            )
            .add_systems(Update, map_tools.in_set(UiSet::Dispatch));
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
        KeyCode::KeyM => ShellCommand::ToggleAutomap,
        KeyCode::KeyC => ShellCommand::Cast,
        KeyCode::Escape => ShellCommand::Pause,
        _ => return None,
    })
}

/// The shell action a tool button stands for; `None` for the buttons whose screens are
/// still to come (they are painted dim and never clicked).
#[must_use]
pub fn tool_for(button: ToolButton) -> Option<ShellCommand> {
    Some(match button {
        ToolButton::Spells => ShellCommand::Cast,
        ToolButton::Map => ShellCommand::ToggleAutomap,
        ToolButton::Menu => ShellCommand::Pause,
        ToolButton::Items | ToolButton::Sheet | ToolButton::Look => return None,
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

/// A click on a live tool button, whatever the screen: the states gate it.
fn map_tools(mut clicks: MessageReader<UiClick>, mut shell: MessageWriter<ShellCommand>) {
    for UiClick(hit) in clicks.read() {
        if let WidgetId::Tool(button) = hit.id
            && let Some(command) = tool_for(button)
        {
            shell.write(command);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_buttons_send_what_their_keys_send() {
        assert_eq!(tool_for(ToolButton::Spells), shell_for(KeyCode::KeyC));
        assert_eq!(tool_for(ToolButton::Map), shell_for(KeyCode::KeyM));
        assert_eq!(tool_for(ToolButton::Menu), shell_for(KeyCode::Escape));
        assert_eq!(tool_for(ToolButton::Items), None);
    }

    #[test]
    fn no_function_key_is_bound() {
        for key in [KeyCode::F1, KeyCode::F5, KeyCode::F9, KeyCode::F12] {
            assert_eq!(command_for(key), None, "{key:?}");
            assert_eq!(shell_for(key), None, "{key:?}");
        }
        assert_eq!(shell_for(KeyCode::Escape), Some(ShellCommand::Pause));
        assert_eq!(shell_for(KeyCode::KeyC), Some(ShellCommand::Cast));
    }
}
