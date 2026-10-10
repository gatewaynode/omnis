//! The screen renderer and router (presentation-ARCHITECTURE §6.4). `screen::content` says what the
//! top screen holds; this module draws it as `bevy_ui` nodes in the style's roles and sizes,
//! rebuilding the nodes only when the screen's shape changes and rewriting the words in place
//! otherwise, and routes presses and keys to `screen::apply`, carrying out the `Effect`.
//! Esc goes through `Nav::escape`; Space, a right click and Shift open the square's actions.
//! Headless-safe.

use super::VectorSet;
use super::capture::CaptureSize;
use super::combat::{FightScreen, size};
use super::session::Session;
use crate::screen::actions::default_action;
use crate::screen::nav::{Escape, Nav};
use crate::screen::{
    self, Control, ControlId, Ctx, Effect, Game, HostOp, Key, Payload, Screen, ScreenId,
};
use crate::style::{Role, SHADES, Size, scaled};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use omnis_sim::omnis_core::Position;

/// Whether a screen is up; the cursor is freed while it is.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Showing(pub bool);

/// The screens' state: the overlays, and what was last drawn.
#[derive(Resource, Debug, Default)]
pub struct Screens {
    /// The overlays over the world's base screen.
    pub nav: Nav,
    /// The screen as last drawn, and the window height it was drawn for.
    drawn: Option<(Screen, u32)>,
    /// Where the party stood last frame, to close the square's actions when it moves.
    at: Option<Position>,
}

/// Marks the node every screen is drawn under.
#[derive(Component)]
struct ScreenRoot;

/// A control's node: pressing it uses the control.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressable(pub ControlId);

/// The words of control `.0`.
#[derive(Component)]
struct LabelOf(ControlId);

/// The words of span `(panel, row, span)`.
#[derive(Component)]
struct SpanAt(usize, usize, usize);

/// The heading's words.
#[derive(Component)]
struct TitleText;

/// The renderer and the router.
pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screens>()
            .init_resource::<Showing>()
            .add_systems(Startup, spawn)
            .add_systems(Update, (route, paint).chain().in_set(VectorSet::Screens));
    }
}

/// `role`'s colour.
#[must_use]
pub fn colour(role: Role) -> Color {
    let [r, g, b] = role.rgb();
    Color::srgb(r, g, b)
}

/// A control's background: at rest, hovered or pressed.
#[must_use]
pub fn shade(i: usize) -> Color {
    let [r, g, b] = SHADES[i.min(2)];
    Color::srgb(r, g, b)
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        ScreenRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(22.0),
            left: Val::Percent(50.0),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(1.0)),
            display: Display::None,
            ..default()
        },
        BorderColor::all(colour(Role::Interface)),
        BackgroundColor(shade(0)),
    ));
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

/// The context screens read.
pub fn ctx(session: &Session) -> Ctx<'_> {
    Ctx {
        data: &session.data,
        game: Game {
            world: &session.world,
            views: &session.views,
        },
    }
}

/// Esc, the square's action keys, a pressed control or its key, routed to the top screen; the
/// effect carried out.
fn route(
    buttons: Query<(&Interaction, &Pressable), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut session: ResMut<Session>,
    mut screens: ResMut<Screens>,
    fight: Option<ResMut<FightScreen>>,
) {
    let screens = &mut *screens;
    let here = session.views.position;
    let moved = screens.at.is_some_and(|p| p != here);
    screens.at = Some(here);
    screens.nav.settle(&session.views, moved);
    if keys.just_pressed(KeyCode::Escape)
        && screens.nav.escape(&session.views) == Escape::FightBack
        && let Some(mut fight) = fight
    {
        fight.menu.back();
    }
    action_keys(&keys, &mouse, &mut session, &mut screens.nav);
    let Some(top) = screens.nav.top(&session.views) else {
        return;
    };
    let pressed = buttons
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, p)| p.0);
    let effect = {
        let ctx = ctx(&session);
        let control = pressed.or_else(|| {
            let n = DIGITS.iter().position(|k| keys.just_pressed(*k))?;
            let digit = u8::try_from(n + 1).ok()?;
            screen::content(top, &ctx).key(Key::Digit(digit))
        });
        control.and_then(|c| screen::apply(top, &ctx, c, &Payload::Press).ok())
    };
    if let Some(effect) = effect {
        carry_out(effect, &mut session, &mut screens.nav);
    }
}

/// Space or a right click opens or closes the square's actions while exploring; with Shift it
/// runs the default action, or opens them when there is none.
fn action_keys(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    session: &mut Session,
    nav: &mut Nav,
) {
    let action = keys.just_pressed(KeyCode::Space) || mouse.just_pressed(MouseButton::Right);
    let free = matches!(nav.top(&session.views), None | Some(ScreenId::Actions));
    if !action || !free || !session.views.exploring() || session.views.combat.is_some() {
        return;
    }
    if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        match default_action(&session.world, &session.data) {
            Some(command) => {
                session.order(command);
                nav.settle(&session.views, true);
            }
            None => nav.open(ScreenId::Actions, &session.views),
        }
    } else {
        nav.toggle(ScreenId::Actions, &session.views);
    }
}

/// Carry out what a screen asked for.
fn carry_out(effect: Effect, session: &mut Session, nav: &mut Nav) {
    match effect {
        Effect::None => {}
        Effect::Command { command, close } => {
            session.order(command);
            if close {
                nav.back();
            }
        }
        Effect::Host(HostOp::Restart) => {
            if let Err(e) = session.restart() {
                session.say(format!("Could not start again: {e}"));
            }
        }
        Effect::Back => {
            nav.back();
        }
    }
}

/// A control's words as drawn: its number key first when it has one, its reason after when it
/// is dim.
fn shown_label(screen: &Screen, control: &Control) -> String {
    match &control.why_not {
        Some(why) => format!("{}: {why}", control.label),
        None => match screen.keys.iter().find(|(_, c)| *c == control.id) {
            Some((Key::Digit(n), _)) => format!("{n}  {}", control.label),
            None => control.label.clone(),
        },
    }
}

fn words(text: &str, role: Role, size: Size, height: f32) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size.px(height)),
            ..default()
        },
        TextColor(colour(role)),
    )
}

/// A control's node: a bordered box with its words; a dim one has no `Button`, so it takes no
/// clicks, and is drawn in the dim role.
fn control_node(parent: &mut ChildSpawnerCommands, screen: &Screen, c: &Control, height: f32) {
    let role = if c.why_not.is_some() {
        Role::Dim
    } else {
        Role::Interface
    };
    let mut entity = parent.spawn((
        Pressable(c.id),
        Node {
            min_width: Val::Px(scaled(150.0, height)),
            height: Val::Px(scaled(34.0, height)),
            padding: UiRect::horizontal(Val::Px(scaled(10.0, height))),
            border: UiRect::all(Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(colour(role)),
        BackgroundColor(shade(0)),
    ));
    if c.why_not.is_none() {
        entity.insert(Button);
    }
    entity.with_children(|b| {
        b.spawn((
            LabelOf(c.id),
            words(&shown_label(screen, c), role, Size::Body, height),
        ));
    });
}

/// Build the screen's nodes under the root.
fn build(parent: &mut ChildSpawnerCommands, screen: &Screen, height: f32) {
    let t = &screen.title;
    parent.spawn((TitleText, words(&t.text, t.role, t.size, height)));
    for (p, panel) in screen.panels.iter().enumerate() {
        for (r, row) in panel.rows.iter().enumerate() {
            parent
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(scaled(6.0, height)),
                    row_gap: Val::Px(scaled(6.0, height)),
                    ..default()
                })
                .with_children(|line| {
                    for (s, span) in row.spans.iter().enumerate() {
                        line.spawn((
                            SpanAt(p, r, s),
                            words(&span.text, span.role, span.size, height),
                        ));
                    }
                    for c in &row.controls {
                        control_node(line, screen, c, height);
                    }
                });
        }
    }
}

/// A screen's words on screen: the heading, a span or a control's label, by its marker.
type Placed = (
    &'static mut Text,
    &'static mut TextColor,
    Option<&'static TitleText>,
    Option<&'static SpanAt>,
    Option<&'static LabelOf>,
);

/// Draw the top screen: rebuilt when its shape or the window height changed, its words
/// rewritten in place when only they did.
#[allow(clippy::too_many_arguments)]
fn paint(
    mut commands: Commands,
    session: Res<Session>,
    mut screens: ResMut<Screens>,
    mut showing: ResMut<Showing>,
    window: Query<&Window, With<PrimaryWindow>>,
    capture: Option<Res<CaptureSize>>,
    mut root: Query<(Entity, &mut Node), With<ScreenRoot>>,
    mut placed: Query<Placed>,
) {
    let Ok((entity, mut node)) = root.single_mut() else {
        return;
    };
    let top = screens.nav.top(&session.views);
    showing.0 = top.is_some();
    let Some(top) = top else {
        if screens.drawn.take().is_some() {
            commands.entity(entity).despawn_related::<Children>();
            node.display = Display::None;
        }
        return;
    };
    let screen = screen::content(top, &ctx(&session));
    let height = size(window.single().ok(), capture.as_deref()).1;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bucket = height as u32;
    match &screens.drawn {
        Some((drawn, h)) if *drawn == screen && *h == bucket => return,
        Some((drawn, h)) if drawn.shape() == screen.shape() && *h == bucket => {
            sync(&screen, &mut placed);
        }
        _ => {
            commands.entity(entity).despawn_related::<Children>();
            node.display = Display::Flex;
            node.width = Val::Px(scaled(520.0, height));
            node.margin = UiRect::left(Val::Px(-scaled(260.0, height)));
            node.padding = UiRect::all(Val::Px(scaled(14.0, height)));
            node.row_gap = Val::Px(scaled(8.0, height));
            commands
                .entity(entity)
                .with_children(|p| build(p, &screen, height));
        }
    }
    screens.drawn = Some((screen, bucket));
}

/// Rewrite the words of a screen whose shape is unchanged.
fn sync(screen: &Screen, placed: &mut Query<Placed>) {
    for (mut text, mut tint, title, span, label) in placed.iter_mut() {
        if title.is_some() {
            text.0.clone_from(&screen.title.text);
            tint.0 = colour(screen.title.role);
        } else if let Some(at) = span {
            let span = screen
                .panels
                .get(at.0)
                .and_then(|p| p.rows.get(at.1))
                .and_then(|r| r.spans.get(at.2));
            if let Some(span) = span {
                text.0.clone_from(&span.text);
                tint.0 = colour(span.role);
            }
        } else if let Some(c) = label.and_then(|of| screen.control(of.0)) {
            text.0 = shown_label(screen, c);
        }
    }
}
