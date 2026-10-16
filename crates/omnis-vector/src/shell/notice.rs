//! The outcome notice drawn (owner, 2026-10-16; presentation-ARCHITECTURE §6.6): the words of the
//! last command the player started, in a bordered panel centred where the screens stand, while
//! the party explores with no screen up (a screen reports its own outcome, a fight keeps its own
//! log). It fades with the distance from where the command was done (`crate::notice::alpha`) and
//! is dropped from the session once out of range. It takes no clicks. Headless-safe.

use super::screen::{colour, shade};
use super::session::Session;
use crate::notice::alpha;
use bevy::prelude::*;

/// Marks the notice's panel.
#[derive(Component, Debug)]
pub struct NoticePanel;

/// Marks the notice's text.
#[derive(Component, Debug)]
pub struct NoticeText;

pub(super) fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|layer| {
            // Where the screens' panel starts (`shell/screen.rs`).
            layer.spawn((
                Node {
                    height: Val::Percent(22.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            layer
                .spawn((
                    NoticePanel,
                    Node {
                        max_width: Val::Percent(60.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                        ..default()
                    },
                    BorderColor::all(Color::NONE),
                    BackgroundColor(Color::NONE),
                    Visibility::Hidden,
                    Pickable::IGNORE,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        NoticeText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        TextLayout::justify(Justify::Center),
                        Visibility::Hidden,
                        Pickable::IGNORE,
                    ));
                });
        });
}

/// The notice text's words, colour and visibility.
type Words = (
    &'static mut Text,
    &'static mut TextColor,
    &'static mut Visibility,
);

/// Fade the notice by the party's distance from where it was done, drop it once out of range,
/// and show it while the party explores with no screen up.
pub(super) fn paint(
    session: Option<ResMut<Session>>,
    screens: Res<super::screen::Screens>,
    mut panel: Query<(&mut BorderColor, &mut BackgroundColor, &mut Visibility), With<NoticePanel>>,
    mut text: Query<Words, (With<NoticeText>, Without<NoticePanel>)>,
) {
    let mut shown = None;
    if let Some(mut session) = session {
        let a = session
            .notice
            .as_ref()
            .map_or(0.0, |n| alpha(n, &session.views.position, &session.pose));
        if a <= 0.0 {
            session.notice = None;
        }
        let free = session.views.exploring() && screens.nav.top(Some(&session.views)).is_none();
        if free && let Some(n) = &session.notice {
            shown = Some((n.lines.join("\n"), n.role, a));
        }
    }
    let want = if shown.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut border, mut back, mut visibility) in &mut panel {
        if *visibility != want {
            *visibility = want;
        }
        if let Some((_, role, a)) = &shown {
            *border = BorderColor::all(colour(*role).with_alpha(*a));
            back.0 = shade(0).with_alpha(shade(0).alpha() * a);
        }
    }
    for (mut words, mut tint, mut visibility) in &mut text {
        if *visibility != want {
            *visibility = want;
        }
        if let Some((line, role, a)) = &shown {
            if words.0 != *line {
                words.0.clone_from(line);
            }
            tint.0 = colour(*role).with_alpha(*a);
        }
    }
}
