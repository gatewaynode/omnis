//! The panel in the middle of the screen (alt-ARCHITECTURE.md §8, §9): the fight notice while
//! monsters are met or the party has fallen, otherwise the action menu when it is open. Its
//! buttons and the number keys carry out the choices. The action key is Space or a right click.
//! Headless-safe.

use super::VectorSet;
use super::actions::{Menu, default_action, menu};
use super::controls::{GREEN, SHADES, button};
use super::fight::{fallen, notice};
use super::notice::{Notice, Order};
use super::session::Session;
use bevy::prelude::*;
use omnis_sim::Mode;
use omnis_sim::omnis_core::Position;

/// Whether the panel is up; the cursor is freed while it is.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Showing(pub bool);

/// Marks the panel.
#[derive(Component)]
struct Panel;

/// The panel, the action menu's keys, and the choices.
pub struct PanelPlugin;

impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Menu>()
            .init_resource::<Showing>()
            .add_systems(Startup, spawn)
            .add_systems(
                Update,
                (keys, choose, refresh).chain().in_set(VectorSet::Input),
            );
    }
}

/// What the panel shows now: the fight notice first, then the open action menu.
#[must_use]
pub fn current(session: &Session, menu_open: bool) -> Option<Notice> {
    notice(&session.world, &session.data).or_else(|| {
        (menu_open && matches!(session.world.mode, Mode::Explore))
            .then(|| menu(&session.world, &session.data))
    })
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Panel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(22.0),
            left: Val::Percent(50.0),
            // Centred on its own width.
            margin: UiRect::left(Val::Px(-260.0)),
            width: Val::Px(520.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            padding: UiRect::all(Val::Px(14.0)),
            border: UiRect::all(Val::Px(1.0)),
            display: Display::None,
            ..default()
        },
        BorderColor::all(GREEN),
        BackgroundColor(SHADES[0]),
    ));
}

/// The action key, Space or a right click, toggles the action menu; with Shift it runs the
/// default action, or opens the menu when there is none; Esc closes it. The menu also closes
/// when the party moves or stops exploring.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut session: ResMut<Session>,
    mut menu: ResMut<Menu>,
    mut last: Local<Option<Position>>,
) {
    let here = session.world.position;
    let exploring =
        matches!(session.world.mode, Mode::Explore) && !fallen(&session.world, &session.data);
    if last.is_some_and(|p| p != here) || !exploring {
        menu.open = false;
    }
    *last = Some(here);
    if keys.just_pressed(KeyCode::Escape) {
        menu.open = false;
    }
    let action = keys.just_pressed(KeyCode::Space) || mouse.just_pressed(MouseButton::Right);
    if !exploring || !action {
        return;
    }
    if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        match default_action(&session.world, &session.data) {
            Some(command) => {
                session.order(command);
                menu.open = false;
            }
            None => menu.open = true,
        }
    } else {
        menu.open = !menu.open;
    }
}

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// A pressed panel button, or its number key, carries out its order.
fn choose(
    buttons: Query<(&Interaction, &Order), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
    mut menu: ResMut<Menu>,
) {
    let mut order = buttons
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, o)| o.clone());
    if order.is_none()
        && let Some(n) = DIGITS.iter().position(|k| keys.just_pressed(*k))
        && let Some(shown) = current(&session, menu.open)
        && let Some(c) = shown.choices.get(n)
        && c.blocked.is_none()
    {
        order = Some(c.order.clone());
    }
    match order {
        Some(Order::Command(command)) => {
            session.order(command);
            menu.open = false;
        }
        Some(Order::Restart) => {
            if let Err(e) = session.restart() {
                session.say(format!("Could not start again: {e}"));
            }
        }
        Some(Order::Close) => menu.open = false,
        None => {}
    }
}

fn text(parent: &mut ChildSpawnerCommands, line: &str, size: f32) {
    parent.spawn((
        Text::new(line),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(GREEN),
    ));
}

/// Rebuild the panel when what it shows may have changed: a command was accepted, the party
/// fell or moved, or the menu opened or closed.
fn refresh(
    mut commands: Commands,
    session: Res<Session>,
    menu: Res<Menu>,
    mut showing: ResMut<Showing>,
    mut panel: Query<(Entity, &mut Node), With<Panel>>,
    mut last: Local<Option<(usize, bool, bool, Position)>>,
) {
    let key = (
        session.binder.log.len(),
        fallen(&session.world, &session.data),
        menu.open,
        session.world.position,
    );
    if *last == Some(key) {
        return;
    }
    *last = Some(key);
    let Ok((entity, mut node)) = panel.single_mut() else {
        return;
    };
    commands.entity(entity).despawn_related::<Children>();
    let shown = current(&session, menu.open);
    showing.0 = shown.is_some();
    let Some(shown) = shown else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    commands.entity(entity).with_children(|p| {
        text(p, &shown.title, 22.0);
        for line in &shown.lines {
            text(p, line, 16.0);
        }
        p.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            for (i, c) in shown.choices.iter().enumerate() {
                let label = match &c.blocked {
                    None => format!("{}  {}", i + 1, c.label),
                    Some(why) => format!("{}: {why}", c.label),
                };
                button(row, &label, c.order.clone(), c.blocked.is_none());
            }
        });
    });
}
