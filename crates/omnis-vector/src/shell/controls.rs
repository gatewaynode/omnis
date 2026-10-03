//! The on-screen buttons: every action has one, and the keys are shortcuts for them
//! (alt-ARCHITECTURE.md §8). Headless-safe: the buttons are plain UI nodes whose `Interaction`
//! a test can set.

use super::VectorSet;
use super::movement::Intent;
use super::session::Session;
use bevy::prelude::*;
use core::f32::consts::FRAC_PI_2;

/// What a button does.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Walk forward while held.
    Forward,
    /// Walk back while held.
    Back,
    /// Sidestep left while held.
    StrafeLeft,
    /// Sidestep right while held.
    StrafeRight,
    /// Turn a quarter to the left.
    TurnLeft,
    /// Turn a quarter to the right.
    TurnRight,
    /// Use the faced edge.
    Interact,
    /// Write the command log as a replay.
    SaveLog,
    /// Leave.
    Quit,
}

impl Action {
    /// The button's label, naming its key.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Action::Forward => "Forward  W",
            Action::Back => "Back  S",
            Action::StrafeLeft => "Left  A",
            Action::StrafeRight => "Right  D",
            Action::TurnLeft => "Turn left  Q",
            Action::TurnRight => "Turn right  R",
            Action::Interact => "Use  E",
            Action::SaveLog => "Save log",
            Action::Quit => "Quit",
        }
    }

    /// Whether the action lasts while the button is held, rather than firing once a press.
    #[must_use]
    pub const fn held(self) -> bool {
        matches!(
            self,
            Action::Forward | Action::Back | Action::StrafeLeft | Action::StrafeRight
        )
    }
}

/// The pad's rows, top to bottom.
const ROWS: [&[Action]; 3] = [
    &[Action::TurnLeft, Action::Forward, Action::TurnRight],
    &[Action::StrafeLeft, Action::Back, Action::StrafeRight],
    &[Action::Interact, Action::SaveLog, Action::Quit],
];

/// The HUD's green.
pub const GREEN: Color = Color::srgb(0.55, 1.0, 0.65);
/// A button at rest, hovered and pressed.
const SHADES: [Color; 3] = [
    Color::srgba(0.0, 0.08, 0.03, 0.75),
    Color::srgba(0.05, 0.25, 0.1, 0.85),
    Color::srgba(0.15, 0.5, 0.2, 0.95),
];

/// The button pad, bottom left.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn)
            .add_systems(Update, (press, shade).in_set(VectorSet::Input));
    }
}

/// A button: a bordered box with a label, carrying `marker`.
pub fn button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Bundle) {
    parent
        .spawn((
            Button,
            marker,
            Node {
                width: Val::Px(150.0),
                height: Val::Px(34.0),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(GREEN),
            BackgroundColor(SHADES[0]),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(GREEN),
            ));
        });
}

fn spawn(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|pad| {
            for row in ROWS {
                pad.spawn(Node {
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|r| {
                    for &action in row {
                        button(r, action.label(), action);
                    }
                });
            }
        });
}

/// Held buttons add motion every frame they are down; the others fire once per press.
fn press(
    buttons: Query<(Ref<Interaction>, &Action)>,
    mut intent: ResMut<Intent>,
    mut session: ResMut<Session>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, &action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if action.held() {
            let motion = &mut intent.motion;
            match action {
                Action::Forward => motion.forward = (motion.forward + 1.0).min(1.0),
                Action::Back => motion.forward = (motion.forward - 1.0).max(-1.0),
                Action::StrafeLeft => motion.strafe = (motion.strafe - 1.0).max(-1.0),
                _ => motion.strafe = (motion.strafe + 1.0).min(1.0),
            }
            continue;
        }
        if !interaction.is_changed() {
            continue;
        }
        match action {
            Action::TurnLeft => intent.pending_turn += FRAC_PI_2,
            Action::TurnRight => intent.pending_turn -= FRAC_PI_2,
            Action::Interact => intent.interact = true,
            Action::SaveLog => {
                let line = match session.save_log() {
                    Ok(path) => format!("Log saved to {}", path.display()),
                    Err(e) => format!("Log not saved: {e}"),
                };
                session.say(line);
            }
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

fn shade(mut buttons: Query<(&Interaction, &mut BackgroundColor), Changed<Interaction>>) {
    for (interaction, mut colour) in &mut buttons {
        colour.0 = match interaction {
            Interaction::None => SHADES[0],
            Interaction::Hovered => SHADES[1],
            Interaction::Pressed => SHADES[2],
        };
    }
}
