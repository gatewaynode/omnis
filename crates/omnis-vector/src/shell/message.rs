//! The explore message line (presentation-ARCHITECTURE §6.6; the retired client's band message):
//! the last event worth a line, or the last refusal in the alert role, at the bottom centre
//! between the movement pad and the right-hand buttons. It shows only while the party explores
//! with no screen up: a screen reports its own outcome, and a fight keeps its own log. It has
//! no timer; the next event, refusal or move replaces it. Headless-safe.

use super::screen::{Screens, colour};
use super::session::Session;
use crate::style::Role;
use bevy::prelude::*;

/// Marks the message line's text.
#[derive(Component, Debug)]
pub struct MessageLine;

/// The movement pad's width (three 150-pixel buttons, two 6-pixel gaps) and its margin.
const LEFT: f32 = 16.0 + 3.0 * 150.0 + 2.0 * 6.0 + 16.0;
/// The right-hand group's width (two buttons, one gap) and its margins.
const RIGHT: f32 = 16.0 + 2.0 * 150.0 + 6.0 + 16.0;

pub(super) fn spawn(mut commands: Commands) {
    commands.spawn((
        MessageLine,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(colour(Role::Interface)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(LEFT),
            right: Val::Px(RIGHT),
            bottom: Val::Px(16.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// Show the session's message while the party explores with no screen up.
pub(super) fn paint(
    session: Option<Res<Session>>,
    screens: Res<Screens>,
    mut line: Query<(&mut Text, &mut TextColor, &mut Visibility), With<MessageLine>>,
) {
    let message = session.as_deref().and_then(|s| {
        let free = s.views.exploring() && screens.nav.top(Some(&s.views)).is_none();
        free.then_some(s.message.as_ref()).flatten()
    });
    for (mut text, mut tint, mut visibility) in &mut line {
        let want = if message.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        if let Some(m) = message {
            if text.0 != m.text {
                text.0.clone_from(&m.text);
            }
            tint.0 = colour(m.role);
        }
    }
}
