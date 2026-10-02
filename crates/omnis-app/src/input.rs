//! `InputPlugin`: keys, the movement pad and the tool pad to commands. Arrows or WASD move
//! and turn, Q and E sidestep, Space or Enter interact, M toggles the automap, C opens the
//! cast menu, P the character sheet, Escape pauses; a click on a pad button sends the same command as its key, and a
//! click on a tool button the same shell command (the buttons' states say when they are
//! live, so the keys are the shortcuts). No function key is bound: macOS takes them (saving
//! and loading live on the pause menu).
//!
//! A step into or out of a town service asks first (`confirm_panel.rs`), as does the service
//! panel's Leave: the command is held in `AskFirst`, the play state is `Confirm`, and Go
//! (Enter) sends it or Stay (Escape) drops it, back to the map or the panel; the question's
//! buttons answer the same way (`feathers_confirm.rs`). Inside a service the map's keys and
//! pad are off: the panel takes the keyboard (`feathers_service.rs`).

use crate::confirm_panel::{self, Confirm, ConfirmId};
use crate::cursor::UiSet;
use crate::sim::{PackData, PlayState, PlayerCommand, ShellCommand, SimSet, SimWorld};
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
            .init_resource::<AskFirst>()
            .add_message::<UiClick>()
            .add_message::<ConfirmAnswer>()
            .add_systems(
                Update,
                answer_keys
                    .in_set(UiSet::Dispatch)
                    .run_if(in_state(PlayState::Confirm)),
            )
            .add_systems(
                Update,
                settle_answers
                    .in_set(SimSet::Collect)
                    .after(UiSet::Dispatch)
                    .run_if(in_state(PlayState::Confirm)),
            )
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
        KeyCode::KeyP => ShellCommand::Sheet,
        KeyCode::KeyI => ShellCommand::Inventory,
        KeyCode::KeyL => ShellCommand::Look,
        KeyCode::Escape => ShellCommand::Pause,
        _ => return None,
    })
}

/// The shell action a tool button stands for.
#[must_use]
pub const fn tool_for(button: ToolButton) -> ShellCommand {
    match button {
        ToolButton::Items => ShellCommand::Inventory,
        ToolButton::Spells => ShellCommand::Cast,
        ToolButton::Sheet => ShellCommand::Sheet,
        ToolButton::Look => ShellCommand::Look,
        ToolButton::Map => ShellCommand::ToggleAutomap,
        ToolButton::Menu => ShellCommand::Pause,
    }
}

/// The step held for the confirmation, while `PlayState::Confirm` is up.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct AskFirst(pub Option<Confirm>);

/// An answer to the confirmation, from a key or a panel button.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmAnswer(pub ConfirmId);

/// Where a player command from the map or a panel goes: to the simulation, or held behind a
/// question.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Gate<'w> {
    world: Option<Res<'w, SimWorld>>,
    data: Option<Res<'w, PackData>>,
    held: ResMut<'w, AskFirst>,
    next: ResMut<'w, NextState<PlayState>>,
    out: MessageWriter<'w, PlayerCommand>,
}

impl Gate<'_> {
    /// Send the command, or hold it and ask first.
    pub(crate) fn send(&mut self, command: Command) {
        if self.held.0.is_some() {
            // A second key in the frame that raised the question waits for the answer.
            return;
        }
        let asked = self
            .world
            .as_ref()
            .zip(self.data.as_ref())
            .and_then(|(world, data)| confirm_panel::ask(&world.0, &data.0, &command));
        match asked {
            Some(confirm) => {
                self.held.0 = Some(confirm);
                self.next.set(PlayState::Confirm);
            }
            None => {
                self.out.write(PlayerCommand(command));
            }
        }
    }
}

fn map_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut gate: Gate,
    mut shell: MessageWriter<ShellCommand>,
) {
    for key in keys.get_just_pressed() {
        if let Some(command) = command_for(*key) {
            gate.send(command);
        } else if let Some(action) = shell_for(*key) {
            shell.write(action);
        }
    }
}

fn map_pad(mut clicks: MessageReader<UiClick>, mut gate: Gate) {
    for UiClick(hit) in clicks.read() {
        if let WidgetId::Pad(button) = hit.id {
            gate.send(button.command());
        }
    }
}

/// Enter goes, Escape stays.
fn answer_keys(keys: Res<ButtonInput<KeyCode>>, mut answers: MessageWriter<ConfirmAnswer>) {
    for key in keys.get_just_pressed() {
        match key {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                answers.write(ConfirmAnswer(ConfirmId::Go));
            }
            KeyCode::Escape => {
                answers.write(ConfirmAnswer(ConfirmId::Stay));
            }
            _ => {}
        }
    }
}

/// The first answer sends the held command or drops it, and the screen the party was on comes
/// back (the map, or a service's panel; the mode moves it on if the command changed that); any
/// later one in the frame finds nothing held.
fn settle_answers(
    mut answers: MessageReader<ConfirmAnswer>,
    mut held: ResMut<AskFirst>,
    world: Option<Res<SimWorld>>,
    mut next: ResMut<NextState<PlayState>>,
    mut out: MessageWriter<PlayerCommand>,
) {
    for ConfirmAnswer(id) in answers.read() {
        let Some(confirm) = held.0.take() else {
            continue;
        };
        if let Some(command) = confirm_panel::answer(confirm, *id) {
            out.write(PlayerCommand(command));
        }
        next.set(
            world
                .as_ref()
                .map_or(PlayState::Explore, |w| PlayState::for_mode(&w.0.mode)),
        );
    }
}

/// A click on a live tool button, whatever the screen: the states gate it.
fn map_tools(mut clicks: MessageReader<UiClick>, mut shell: MessageWriter<ShellCommand>) {
    for UiClick(hit) in clicks.read() {
        if let WidgetId::Tool(button) = hit.id {
            shell.write(tool_for(button));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_buttons_send_what_their_keys_send() {
        assert_eq!(Some(tool_for(ToolButton::Spells)), shell_for(KeyCode::KeyC));
        assert_eq!(Some(tool_for(ToolButton::Map)), shell_for(KeyCode::KeyM));
        assert_eq!(Some(tool_for(ToolButton::Sheet)), shell_for(KeyCode::KeyP));
        assert_eq!(Some(tool_for(ToolButton::Menu)), shell_for(KeyCode::Escape));
        assert_eq!(Some(tool_for(ToolButton::Items)), shell_for(KeyCode::KeyI));
        assert_eq!(Some(tool_for(ToolButton::Look)), shell_for(KeyCode::KeyL));
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
