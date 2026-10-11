//! The on-screen buttons: every action has one, and the keys are shortcuts for them
//! (presentation-ARCHITECTURE.md §8). Headless-safe: the buttons are plain UI nodes whose `Interaction`
//! a test can set.

use super::VectorSet;
use super::movement::Intent;
use super::screen::Screens;
use super::session::Session;
use crate::screen::ScreenId;
use crate::style::{Role, SHADES as SHADE_RGB};
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
    /// Open or close the camp: rest on the road.
    Camp,
    /// Open or close the action menu for the square.
    Actions,
    /// Open the pause: save, load, the log, leave.
    Menu,
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
            Action::TurnRight => "Turn right  E",
            Action::Camp => "Camp  R",
            Action::Actions => "Actions  Space",
            Action::Menu => "Menu  Esc",
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

/// The movement pad's rows, top to bottom, bottom left.
const PAD: [&[Action]; 2] = [
    &[Action::TurnLeft, Action::Forward, Action::TurnRight],
    &[Action::StrafeLeft, Action::Back, Action::StrafeRight],
];

/// The other actions, bottom right, away from the pad: the camp, the square's actions, and the
/// menu (the pause, which holds save, load, the log and quitting).
const SIDE: [&[Action]; 1] = [&[Action::Camp, Action::Actions, Action::Menu]];

/// The interface role's colour (`style::Role::Interface`).
pub const GREEN: Color = srgb(Role::Interface.rgb());
/// A button at rest, hovered and pressed (`style::SHADES`).
pub const SHADES: [Color; 3] = [srgb(SHADE_RGB[0]), srgb(SHADE_RGB[1]), srgb(SHADE_RGB[2])];

const fn srgb([r, g, b]: [f32; 3]) -> Color {
    Color::srgb(r, g, b)
}

/// The movement pad, bottom left, and the other actions, bottom right.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn, super::message::spawn))
            .add_systems(Update, (press, shade).in_set(VectorSet::Input))
            .add_systems(
                Update,
                (super::message::paint, light).in_set(VectorSet::Screens),
            );
    }
}

/// The colour of a button that cannot be pressed now (`style::Role::Dim`).
const DIM: Color = srgb(Role::Dim.rgb());

/// A button: a bordered box with a label, carrying `marker`. A disabled one is drawn dim and
/// takes no clicks (it has no `Button`), so what cannot be done is still shown.
pub fn button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Bundle, enabled: bool) {
    let colour = if enabled { GREEN } else { DIM };
    let mut entity = parent.spawn((
        marker,
        Node {
            min_width: Val::Px(150.0),
            height: Val::Px(34.0),
            padding: UiRect::horizontal(Val::Px(10.0)),
            border: UiRect::all(Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(colour),
        BackgroundColor(SHADES[0]),
    ));
    if enabled {
        entity.insert(Button);
    }
    entity.with_children(|b| {
        b.spawn((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(colour),
        ));
    });
}

/// Which bottom corner a group of buttons sits in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    /// Bottom left: the movement pad.
    Left,
    /// Bottom right: actions, save log, quit.
    Right,
}

fn spawn(mut commands: Commands) {
    group(&mut commands, Corner::Left, &PAD);
    group(&mut commands, Corner::Right, &SIDE);
}

/// A block of button rows anchored 16 pixels from a bottom corner.
fn group(commands: &mut Commands, corner: Corner, rows: &[&[Action]]) {
    let (left, right) = match corner {
        Corner::Left => (Val::Px(16.0), Val::Auto),
        Corner::Right => (Val::Auto, Val::Px(16.0)),
    };
    commands
        .spawn((
            corner,
            Node {
                position_type: PositionType::Absolute,
                left,
                right,
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|block| {
            for &row in rows {
                block
                    .spawn(Node {
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|r| {
                        for &action in row {
                            button(r, action.label(), action, true);
                        }
                    });
            }
        });
}

/// Held buttons add motion every frame they are down; the others fire once per press.
fn press(
    buttons: Query<(Ref<Interaction>, &Action)>,
    mut intent: ResMut<Intent>,
    session: Res<Session>,
    mut screens: ResMut<Screens>,
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
            Action::Camp => screens.nav.toggle(ScreenId::Camp, Some(&session.views)),
            Action::Actions => screens.nav.toggle(ScreenId::Actions, Some(&session.views)),
            Action::Menu => screens.nav.open(ScreenId::Pause, Some(&session.views)),
            _ => {}
        }
    }
}

/// Light the Camp button while the world allows a camp (exploring, outside a service and a
/// fight; the other client's tool bar), and dim it otherwise: a dim button takes no clicks.
fn light(
    mut commands: Commands,
    session: Option<Res<Session>>,
    screens: Res<Screens>,
    buttons: Query<(Entity, &Action, Has<Button>, &Children)>,
    mut borders: Query<&mut BorderColor>,
    mut texts: Query<&mut TextColor>,
) {
    let views = session.as_deref().map(|s| &s.views);
    let live = views.is_some() && screens.nav.allowed(ScreenId::Camp, views);
    for (entity, &action, lit, children) in &buttons {
        if action != Action::Camp || lit == live {
            continue;
        }
        let colour = if live { GREEN } else { DIM };
        if live {
            commands.entity(entity).insert(Button);
        } else {
            // No `Interaction` either, so the pointer cannot press it (`button` draws a disabled
            // one the same way).
            commands.entity(entity).remove::<(Button, Interaction)>();
        }
        if let Ok(mut border) = borders.get_mut(entity) {
            *border = BorderColor::all(colour);
        }
        for &child in children {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = colour;
            }
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
