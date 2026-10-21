//! The outcome notice drawn (owner, 2026-10-16; presentation-ARCHITECTURE §6.6): the words of the
//! last command the player started, in a bordered panel centred where the screens stand, while
//! the party explores with no screen up (a screen reports its own outcome, a fight keeps its own
//! log). It fades with the distance from where the command was done (`crate::notice::alpha`) and
//! is dropped from the session once out of range. Only its close button, top right, takes a
//! click: it drops the notice (owner, 2026-10-17); the log keeps the words. Headless-safe.

use super::screen::{colour, shade};
use super::session::Session;
use crate::notice::alpha;
use bevy::prelude::*;

/// The widest the notice stands, in percent of the window, centred; the HUD's log keeps left of
/// it (`hud::log_width`).
pub const NOTICE_WIDTH: f32 = 60.0;

/// Marks the notice's panel.
#[derive(Component, Debug)]
pub struct NoticePanel;

/// Marks the notice's text.
#[derive(Component, Debug)]
pub struct NoticeText;

/// The close button's side, in pixels.
const CLOSE_SIZE: f32 = 22.0;

/// The panel's padding left and right: the close button and a margin.
const CLOSE_ROOM: f32 = CLOSE_SIZE + 12.0;

/// Marks the notice's close button, its top right corner.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoticeClose;

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
                        max_width: Val::Percent(NOTICE_WIDTH),
                        border: UiRect::all(Val::Px(1.0)),
                        // Wide enough each side that the centred words clear the close button.
                        padding: UiRect::axes(Val::Px(CLOSE_ROOM), Val::Px(12.0)),
                        ..default()
                    },
                    BorderColor::all(Color::NONE),
                    BackgroundColor(Color::NONE),
                    Visibility::Hidden,
                    Pickable::IGNORE,
                ))
                .with_children(|panel| {
                    panel
                        .spawn((
                            NoticeClose,
                            Button,
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(4.0),
                                right: Val::Px(4.0),
                                width: Val::Px(CLOSE_SIZE),
                                height: Val::Px(CLOSE_SIZE),
                                border: UiRect::all(Val::Px(1.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor::all(Color::NONE),
                            BackgroundColor(Color::NONE),
                        ))
                        .with_children(|close| {
                            close.spawn((
                                Text::new("x"),
                                TextFont {
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::NONE),
                                Pickable::IGNORE,
                            ));
                        });
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

/// The close button drops the notice.
pub(super) fn close(
    buttons: Query<&Interaction, (Changed<Interaction>, With<NoticeClose>)>,
    session: Option<ResMut<Session>>,
) {
    if let Some(mut session) = session
        && buttons.iter().any(|i| *i == Interaction::Pressed)
    {
        session.notice = None;
    }
}

/// The close button's border and back, and its letter.
type Close = (
    &'static mut BorderColor,
    &'static mut BackgroundColor,
    &'static Children,
);

/// Fade the notice by the party's distance from where it was done, drop it once out of range,
/// and show it while the party explores with no screen up.
pub(super) fn paint(
    session: Option<ResMut<Session>>,
    screens: Res<super::screen::Screens>,
    mut panel: Query<(&mut BorderColor, &mut BackgroundColor, &mut Visibility), With<NoticePanel>>,
    mut text: Query<Words, (With<NoticeText>, Without<NoticePanel>)>,
    mut close: Query<Close, (With<NoticeClose>, Without<NoticePanel>)>,
    mut letters: Query<&mut TextColor, Without<NoticeText>>,
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
    if let Some((_, role, a)) = &shown {
        for (mut border, mut back, children) in &mut close {
            *border = BorderColor::all(colour(*role).with_alpha(*a));
            // The hover shade (`controls::shade`) sets the colour; the fade sets its alpha.
            back.0 = back.0.with_alpha(*a);
            for &child in children {
                if let Ok(mut tint) = letters.get_mut(child) {
                    tint.0 = colour(*role).with_alpha(*a);
                }
            }
        }
    }
}
