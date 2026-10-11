//! The screen renderer and router (presentation-ARCHITECTURE §6.4). `screen::content` says what the
//! top screen holds; this module draws it as `bevy_ui` nodes in the style's roles and sizes,
//! rebuilding the nodes only when the screen's shape changes and rewriting the words in place
//! otherwise, and routes presses, choice arrows, typed text and keys to `screen::apply`, carrying
//! out the `Effect`: commands through the session, the host's own work (new game, save, load,
//! the log), and moving between screens. Esc goes through `Nav::escape`; Space, a right click and
//! Shift open the square's actions. While no game runs, the game's HUD and buttons are hidden.
//! Headless-safe.

use super::capture::CaptureSize;
use super::combat::{FightScreen, size};
use super::controls::Corner;
use super::hud::StatusText;
use super::minimap::Minimap;
use super::session::{Packs, Session, StartScreen};
use super::{VectorSet, ViewState};
use crate::combat_menu::Step;
use crate::game_io;
use crate::screen::actions::default_action;
use crate::screen::nav::{Escape, Nav};
use crate::screen::{
    self, Control, ControlId, Ctx, Effect, Forms, Game, HostOp, Key, Kind, Payload, Refused,
    Screen, ScreenId, Slot, Span,
};
use crate::style::{Role, SHADES, Size, scaled};
use bevy::ecs::system::SystemParam;
use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::window::PrimaryWindow;
use omnis_sim::omnis_core::Position;
use omnis_sim::omnis_core::rng::splitmix64;
use std::path::Path;

/// Whether a screen is up; the cursor is freed while it is.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Showing(pub bool);

/// The screens' state: the overlays, the forms, and what was last drawn.
#[derive(Resource, Debug, Default)]
pub struct Screens {
    /// The overlays over the world's base screen.
    pub nav: Nav,
    /// The forms being filled and the last outcome to report.
    pub forms: Forms,
    /// The screen as last drawn, and the window height it was drawn for.
    drawn: Option<(Screen, u32)>,
    /// Where the party stood last frame, to close the square's actions when it moves.
    at: Option<Position>,
}

/// Marks the node every screen is drawn under.
#[derive(Component)]
struct ScreenRoot;

/// A control's node: pressing it, or typing in it, uses the control.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressable(pub ControlId);

/// One of a choice's arrows: `true` steps right.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrow(pub ControlId, pub bool);

/// The words of control `.0` (a choice's current option).
#[derive(Component)]
struct LabelOf(ControlId);

/// The words of span `(panel, row, span)`.
#[derive(Component)]
struct SpanAt(usize, usize, usize);

/// The heading's words.
#[derive(Component)]
struct TitleText;

/// The outcome's words, under the panels.
#[derive(Component)]
struct MessageText;

/// The renderer and the router.
pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screens>()
            .init_resource::<Showing>()
            .add_systems(Startup, (spawn, open_start))
            .add_systems(
                Update,
                (route, paint, chrome).chain().in_set(VectorSet::Screens),
            );
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

/// Open the screen the command line asked for (`--screen`).
fn open_start(
    start: Option<Res<StartScreen>>,
    session: Option<Res<Session>>,
    mut screens: ResMut<Screens>,
) {
    let game = session.as_deref().map(|s| &s.views);
    match start.as_deref() {
        Some(StartScreen::NewGame) => screens.nav.open(ScreenId::NewGame, game),
        Some(StartScreen::Pause) => screens.nav.open(ScreenId::Pause, game),
        Some(StartScreen::Actions) => screens.nav.open(ScreenId::Actions, game),
        Some(StartScreen::Title) | None => {}
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

/// Where saves are read and written: the working directory.
fn root() -> &'static Path {
    Path::new(".")
}

/// The context of a running game, as its screens read it.
#[must_use]
pub fn ctx(session: &Session) -> Ctx<'_> {
    Ctx {
        data: &session.data,
        game: Some(Game {
            world: &session.world,
            views: &session.views,
            log: &session.fight_log,
            unreplayable: session.unreplayable.as_deref(),
        }),
        save: slot(&session.config.save),
    }
}

fn slot(path: &str) -> Slot<'_> {
    Slot {
        path,
        exists: game_io::exists(root(), path),
    }
}

/// The context with or without a game.
#[must_use]
pub fn ctx_with<'a>(session: Option<&'a Session>, packs: &'a Packs) -> Ctx<'a> {
    session.map_or_else(
        || Ctx {
            data: &packs.data,
            game: None,
            save: slot(&packs.config.save),
        },
        ctx,
    )
}

/// A seed for a game whose seed was left blank: the clock's nanoseconds, mixed.
fn entropy() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    #[allow(clippy::cast_possible_truncation)]
    splitmix64(nanos as u64 ^ (nanos >> 64) as u64)
}

/// What the router may change besides the screens and the session.
#[derive(SystemParam)]
struct Host<'w, 's> {
    commands: Commands<'w, 's>,
    packs: Res<'w, Packs>,
    exit: MessageWriter<'w, AppExit>,
    view: ResMut<'w, NextState<ViewState>>,
}

/// The top screen's input, from its nodes and the keys.
#[derive(SystemParam)]
struct Used<'w, 's> {
    buttons: Query<'w, 's, (&'static Interaction, &'static Pressable), Changed<Interaction>>,
    arrows: Query<'w, 's, (&'static Interaction, &'static Arrow), Changed<Interaction>>,
    fields: Query<'w, 's, (Entity, Ref<'static, EditableText>, &'static Pressable)>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
}

/// Esc, the square's action keys, a pressed control, a choice's arrow, typed text or a key,
/// routed to the top screen; the effect carried out.
fn route(
    used: Used,
    mut host: Host,
    mut session: Option<ResMut<Session>>,
    mut screens: ResMut<Screens>,
    fight: Option<ResMut<FightScreen>>,
) {
    let screens = &mut *screens;
    {
        let game = session.as_deref().map(|s| &s.views);
        let here = game.map(|v| v.position);
        let moved = screens.at.is_some() && here.is_some() && screens.at != here;
        screens.at = here;
        screens.nav.settle(game, moved);
        if used.keys.just_pressed(KeyCode::Escape) {
            let at_root = fight.as_ref().is_none_or(|f| f.menu.step == Step::Top);
            match screens.nav.escape(game, at_root) {
                Escape::FightBack => {
                    if let Some(mut fight) = fight {
                        fight.menu.back();
                    }
                }
                Escape::Closed | Escape::Paused => screens.forms.message = None,
                Escape::Nothing => {}
            }
        }
    }
    if let Some(session) = session.as_deref_mut() {
        action_keys(&used.keys, &used.mouse, session, &mut screens.nav);
    }
    let Some(top) = screens.nav.top(session.as_deref().map(|s| &s.views)) else {
        return;
    };
    let effect = {
        let ctx = ctx_with(session.as_deref(), &host.packs);
        applied(&used, &ctx, top, &mut screens.forms, &mut host.commands)
    };
    if let Some(effect) = effect {
        carry_out(effect, &mut host, &mut session, screens);
    }
}

/// The control used this frame on screen `top`, applied: a press, a choice's arrow, new text
/// in a field, or a number key. Text a field cannot take is put back to what the form holds.
fn applied(
    used: &Used,
    ctx: &Ctx,
    top: ScreenId,
    forms: &mut Forms,
    commands: &mut Commands,
) -> Option<Effect> {
    let screen = screen::content(top, ctx, forms);
    let pressed = used
        .buttons
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, p)| (p.0, Payload::Press));
    let stepped = || {
        used.arrows
            .iter()
            .find(|(i, _)| **i == Interaction::Pressed)
            .map(|(_, a)| (a.0, if a.1 { Payload::Right } else { Payload::Left }))
    };
    let typed = || {
        used.fields.iter().find_map(|(_, field, p)| {
            let Some(Control {
                kind: Kind::Text { value, .. },
                ..
            }) = screen.control(p.0)
            else {
                return None;
            };
            let text = field.value().to_string();
            (field.is_changed() && text != *value).then_some((p.0, Payload::Text(text)))
        })
    };
    let keyed = || {
        let n = DIGITS.iter().position(|k| used.keys.just_pressed(*k))?;
        let digit = u8::try_from(n + 1).ok()?;
        screen.key(Key::Digit(digit)).map(|id| (id, Payload::Press))
    };
    let (id, payload) = pressed.or_else(stepped).or_else(typed).or_else(keyed)?;
    match screen::apply(top, ctx, forms, id, &payload, entropy()) {
        Ok(effect) => Some(effect),
        Err(Refused::TooLong(_)) => {
            // The field shows what the form holds again.
            if let Some(Control {
                kind: Kind::Text { value, max },
                ..
            }) = screen.control(id)
            {
                for (entity, _, p) in &used.fields {
                    if p.0 == id {
                        commands.entity(entity).insert(text_field(value, *max));
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

fn text_field(value: &str, max: usize) -> EditableText {
    let mut field = EditableText::new(value);
    field.max_characters = Some(max);
    field
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
    let free = matches!(
        nav.top(Some(&session.views)),
        None | Some(ScreenId::Actions)
    );
    if !action || !free || !session.views.exploring() || session.views.combat.is_some() {
        return;
    }
    if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        match default_action(&session.world, &session.data) {
            Some(command) => {
                session.order(command);
                nav.settle(Some(&session.views), true);
            }
            None => nav.open(ScreenId::Actions, Some(&session.views)),
        }
    } else {
        nav.toggle(ScreenId::Actions, Some(&session.views));
    }
}

/// Carry out what a screen asked for. Every effect but `None` clears the last outcome; the
/// host's work then reports its own.
fn carry_out(
    effect: Effect,
    host: &mut Host,
    session: &mut Option<ResMut<Session>>,
    screens: &mut Screens,
) {
    if effect != Effect::None {
        screens.forms.message = None;
    }
    match effect {
        Effect::None => {}
        Effect::Command { command, close } => {
            if let Some(session) = session.as_deref_mut() {
                session.order(command);
            }
            if close {
                screens.nav.back();
            }
        }
        Effect::Back => {
            screens.nav.back();
        }
        Effect::Open(id) => screens.nav.open(id, session.as_deref().map(|s| &s.views)),
        Effect::Quit => {
            host.exit.write(AppExit::Success);
        }
        Effect::ToTitle => {
            host.commands.remove_resource::<Session>();
            host.view.set(ViewState::Explore);
            screens.nav.reset();
        }
        Effect::Host(op) => host_op(op, host, session, screens),
    }
}

/// Report an outcome on the screen: words in the interface role, or a failure in the alert role.
fn report(screens: &mut Screens, outcome: Result<String, String>) {
    screens.forms.message = Some(match outcome {
        Ok(text) => Span::body(text),
        Err(text) => Span::alert(text),
    });
}

/// The client's own work.
fn host_op(
    op: HostOp,
    host: &mut Host,
    session: &mut Option<ResMut<Session>>,
    screens: &mut Screens,
) {
    let save = host.packs.config.save.clone();
    match op {
        HostOp::NewGame { seed, settings } => {
            match Session::new_game(&host.packs, seed, settings) {
                Ok(fresh) => {
                    host.commands.insert_resource(fresh);
                    screens.nav.reset();
                }
                Err(e) => report(screens, Err(format!("Could not start: {e}"))),
            }
        }
        HostOp::Restart => {
            if let Some(session) = session.as_deref_mut() {
                match session.restart() {
                    Ok(()) => screens.nav.reset(),
                    Err(e) => session.say(format!("Could not start again: {e}")),
                }
            }
        }
        HostOp::Save => {
            if let Some(session) = session.as_deref() {
                let outcome = game_io::save(&session.world, root(), &save)
                    .map(|_| format!("Saved to {save}"))
                    .map_err(|e| format!("Save failed: {e}"));
                report(screens, outcome);
            }
        }
        HostOp::Load => {
            let data = session
                .as_deref()
                .map_or_else(|| host.packs.data.clone(), |s| s.data.clone());
            match game_io::load(&data, root(), &save, false) {
                Ok(world) => {
                    let why = format!("the save {save} was loaded");
                    if let Some(session) = session.as_deref_mut() {
                        session.replace(world, &why);
                        session.say(format!("Loaded {save}"));
                    } else {
                        let mut fresh = Session::loaded(&host.packs, world, &why);
                        fresh.say(format!("Loaded {save}"));
                        host.commands.insert_resource(fresh);
                    }
                    screens.nav.reset();
                }
                Err(e) => report(screens, Err(format!("Load failed: {e}"))),
            }
        }
        HostOp::SaveLog => {
            if let Some(session) = session.as_deref() {
                let outcome = session
                    .save_log()
                    .map(|path| format!("Log saved to {}", path.display()))
                    .map_err(|e| format!("Log not saved: {e}"));
                report(screens, outcome);
            }
        }
    }
}

/// A control's words as drawn: its number key first when it has one, its reason after when it
/// is dim; a choice shows its current option.
fn shown_label(screen: &Screen, control: &Control) -> String {
    if let Kind::Choice { options, at } = &control.kind {
        return options.get(*at).cloned().unwrap_or_default();
    }
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

/// A bordered box, as every control is drawn.
fn boxed(role: Role, height: f32) -> impl Bundle {
    (
        Node {
            min_width: Val::Px(scaled(150.0, height)),
            height: Val::Px(scaled(34.0, height)),
            padding: UiRect::horizontal(Val::Px(scaled(10.0, height))),
            border: UiRect::all(Val::Px(1.0)),
            column_gap: Val::Px(scaled(10.0, height)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(colour(role)),
        BackgroundColor(shade(0)),
    )
}

/// A choice's arrow: a small pressable box.
fn arrow(parent: &mut ChildSpawnerCommands, id: ControlId, right: bool, height: f32) {
    parent
        .spawn((
            Arrow(id, right),
            Button,
            Node {
                width: Val::Px(scaled(28.0, height)),
                height: Val::Px(scaled(26.0, height)),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(colour(Role::Interface)),
            BackgroundColor(shade(0)),
        ))
        .with_children(|a| {
            let glyph = if right { ">" } else { "<" };
            a.spawn(words(glyph, Role::Interface, Size::Body, height));
        });
}

/// A control's nodes. A button is a bordered box with its words; a dim one has no `Button`, so it
/// takes no clicks, and is drawn in the dim role. A choice is its name, two arrows and the
/// current option between them. A text field is its name and an editable line.
fn control_node(parent: &mut ChildSpawnerCommands, screen: &Screen, c: &Control, height: f32) {
    let role = if c.why_not.is_some() {
        Role::Dim
    } else {
        Role::Interface
    };
    match &c.kind {
        Kind::Button => {
            let mut entity = parent.spawn((Pressable(c.id), boxed(role, height)));
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
        Kind::Choice { .. } => {
            parent.spawn(boxed(role, height)).with_children(|b| {
                b.spawn(words(&c.label, role, Size::Body, height));
                arrow(b, c.id, false, height);
                b.spawn((
                    LabelOf(c.id),
                    words(&shown_label(screen, c), role, Size::Body, height),
                ));
                arrow(b, c.id, true, height);
            });
        }
        Kind::Text { value, max } => {
            parent.spawn(boxed(role, height)).with_children(|b| {
                b.spawn(words(&c.label, role, Size::Body, height));
                b.spawn((
                    Pressable(c.id),
                    text_field(value, *max),
                    AutoFocus,
                    TextFont {
                        font_size: FontSize::Px(Size::Body.px(height)),
                        ..default()
                    },
                    TextColor(colour(Role::Interface)),
                    // Underlined, so an empty field still shows where to type.
                    Node {
                        min_width: Val::Px(scaled(260.0, height)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(colour(Role::Dim)),
                ));
            });
        }
    }
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
    if let Some(m) = &screen.message {
        parent.spawn((MessageText, words(&m.text, m.role, m.size, height)));
    }
}

/// A screen's words on screen: the heading, a span, a control's label or the outcome, by marker.
type Placed = (
    &'static mut Text,
    &'static mut TextColor,
    Option<&'static TitleText>,
    Option<&'static SpanAt>,
    Option<&'static LabelOf>,
    Option<&'static MessageText>,
);

/// Draw the top screen: rebuilt when its shape, its outcome line or the window height changed,
/// its words rewritten in place when only they did.
#[allow(clippy::too_many_arguments)]
fn paint(
    mut commands: Commands,
    session: Option<Res<Session>>,
    packs: Res<Packs>,
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
    let top = screens.nav.top(session.as_deref().map(|s| &s.views));
    showing.0 = top.is_some();
    let Some(top) = top else {
        if screens.drawn.take().is_some() {
            commands.entity(entity).despawn_related::<Children>();
            node.display = Display::None;
        }
        return;
    };
    let ctx = ctx_with(session.as_deref(), &packs);
    let screen = screen::content(top, &ctx, &screens.forms);
    let height = size(window.single().ok(), capture.as_deref()).1;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bucket = height as u32;
    let same_shape = |drawn: &Screen| {
        drawn.shape() == screen.shape() && drawn.message.is_some() == screen.message.is_some()
    };
    match &screens.drawn {
        Some((drawn, h)) if *drawn == screen && *h == bucket => return,
        Some((drawn, h)) if same_shape(drawn) && *h == bucket => sync(&screen, &mut placed),
        _ => {
            commands.entity(entity).despawn_related::<Children>();
            node.display = Display::Flex;
            node.width = Val::Px(scaled(560.0, height));
            node.margin = UiRect::left(Val::Px(-scaled(280.0, height)));
            node.padding = UiRect::all(Val::Px(scaled(14.0, height)));
            node.row_gap = Val::Px(scaled(8.0, height));
            commands
                .entity(entity)
                .with_children(|p| build(p, &screen, height));
        }
    }
    screens.drawn = Some((screen, bucket));
}

/// The words and role a placed text should show now, or `None` when the screen has no such text.
fn wanted(
    screen: &Screen,
    title: Option<&TitleText>,
    span: Option<&SpanAt>,
    label: Option<&LabelOf>,
    message: Option<&MessageText>,
) -> Option<(String, Role)> {
    if title.is_some() {
        return Some((screen.title.text.clone(), screen.title.role));
    }
    if let Some(at) = span {
        let span = screen.panels.get(at.0)?.rows.get(at.1)?.spans.get(at.2)?;
        return Some((span.text.clone(), span.role));
    }
    if let Some(of) = label {
        let c = screen.control(of.0)?;
        let role = if c.why_not.is_some() {
            Role::Dim
        } else {
            Role::Interface
        };
        return Some((shown_label(screen, c), role));
    }
    message?;
    screen.message.as_ref().map(|m| (m.text.clone(), m.role))
}

/// Rewrite the words of a screen whose shape is unchanged.
fn sync(screen: &Screen, placed: &mut Query<Placed>) {
    for (mut text, mut tint, title, span, label, message) in placed.iter_mut() {
        if let Some((words, role)) = wanted(screen, title, span, label, message) {
            if text.0 != words {
                text.0 = words;
            }
            tint.0 = colour(role);
        }
    }
}

/// The game's own nodes: the HUD's status, the button groups and the minimap.
type GameChrome = Or<(With<StatusText>, With<Corner>, With<Minimap>)>;

/// The game's HUD, buttons and minimap show only while a game runs.
fn chrome(session: Option<Res<Session>>, mut nodes: Query<&mut Visibility, GameChrome>) {
    let want = if session.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut nodes {
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Whether the game moves this frame: a game runs and no screen holds it (`Nav::paused`).
#[must_use]
pub fn playing(session: Option<Res<Session>>, screens: Option<Res<Screens>>) -> bool {
    let Some(session) = session else {
        return false;
    };
    screens.is_none_or(|s| !s.nav.paused(Some(&session.views)))
}
