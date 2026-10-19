//! The screen renderer and router (presentation-ARCHITECTURE §6.4). `screen::content` says what the
//! top screen holds; this module draws it as `bevy_ui` nodes in the style's roles and sizes,
//! rebuilding the nodes only when the screen's shape changes and rewriting the words in place
//! otherwise, and routes presses, choice arrows, typed text and keys to `screen::apply`, carrying
//! out the `Effect`: commands through the session, the host's own work (new game, save, load,
//! the log), and moving between screens. Esc goes through `Nav::escape`; Space, a right click and
//! Shift open the square's actions. While no game runs, the game's HUD and buttons are hidden.
//! Headless-safe.

use super::capture::CaptureSize;
use super::combat::size;
use super::controls::Corner;
use super::hud::Hud;
use super::minimap::Minimap;
pub use super::screen_draw::{
    Arrow, FightPress, Pressable, Scrolls, Side, SliderOf, colour, shade,
};
use super::screen_draw::{
    LabelOf, Mark, MessageText, SCROLL_FOOT, SpanAt, TitleText, bars, build, shown_label,
    text_field, thumbs, wheel, width,
};
use super::session::{Packs, Session, StartScreen};
use super::settings::{Current, Fitted};
use super::{VectorSet, ViewState};
use crate::combat_menu::Step;
use crate::game_io;
use crate::screen::actions::{Does, default_action};
use crate::screen::load_report::Refusal;
use crate::screen::nav::{Escape, Nav};
use crate::screen::{
    self, Control, ControlId, Ctx, Effect, Forms, Game, HostOp, Key, Kind, Payload, Screen,
    ScreenId, Slot, Span,
};
use crate::sim::Views;
use crate::style::{Role, scaled};
use bevy::ecs::system::SystemParam;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui_widgets::{SliderValue, ValueChange};
use bevy::window::PrimaryWindow;
use omnis_sim::omnis_core::Position;
use omnis_sim::omnis_core::rng::splitmix64;
use omnis_sim::{Command, ServiceCommand};
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

impl Screens {
    /// A new start: the base screen for `game`, no overlay, no member selected.
    pub fn reset(&mut self, game: Option<&Views>) {
        self.nav.reset(game);
        self.forms.selected = None;
        self.forms.fight = crate::screen::fight::FightForm::default();
        self.forms.debug = crate::screen::debug::DebugForm::default();
    }
}

/// Marks the node every screen is drawn under.
#[derive(Component)]
struct ScreenRoot;

/// The screens' layer stands over every other root of the interface.
const SCREEN_Z: i32 = 1;

/// Marks the window-sized layer the screen is centred in.
#[derive(Component)]
pub struct ScreenLayer;

/// The room the party column takes at the right while a screen stands beside it (the inventory,
/// the cast screen), in logical pixels; 0 otherwise. The column's paint writes it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct Beside(pub f32);

/// A slider moved since the router last looked: the number it now asks for.
#[derive(Resource, Debug, Default)]
struct Slid(Option<(ControlId, i64)>);

/// The renderer and the router.
pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screens>()
            .init_resource::<Current>()
            .init_resource::<Fitted>()
            .init_resource::<Showing>()
            .init_resource::<super::roster::Drawn>()
            .init_resource::<Slid>()
            .init_resource::<Beside>()
            .add_observer(slid)
            .add_systems(Startup, (spawn, super::roster::spawn, open_start))
            .add_systems(
                Update,
                (
                    route,
                    super::roster::route,
                    paint,
                    foot,
                    super::roster::paint,
                    make_room,
                    thumbs,
                    bars,
                    wheel,
                    chrome,
                )
                    .chain()
                    .in_set(VectorSet::Screens),
            );
    }
}

/// A slider reports a move: kept, rounded to a whole number, for the router's next look. A
/// value that is not a number is dropped.
fn slid(change: On<ValueChange<f32>>, sliders: Query<&SliderOf>, mut moved: ResMut<Slid>) {
    let Ok(of) = sliders.get(change.source) else {
        return;
    };
    if change.value.is_finite() {
        #[allow(clippy::cast_possible_truncation)]
        let n = change.value.round() as i64;
        moved.0 = Some((of.0, n));
    }
}

/// The window-sized layer the screen is centred in: a gap above it, 22% of the window, that gives
/// way when the screen is taller than the rest, so the screen's foot stays in the window. Clicks
/// pass through the layer and the gap.
fn spawn(mut commands: Commands) {
    commands
        .spawn((
            ScreenLayer,
            // Over the HUD, the column, the buttons and the fight screen, which carry none.
            GlobalZIndex(SCREEN_Z),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(Val::Vh(2.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|layer| {
            layer.spawn((
                Node {
                    height: Val::Percent(22.0),
                    min_height: Val::Vh(2.0),
                    flex_shrink: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
            layer.spawn((
                ScreenRoot,
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(1.0)),
                    display: Display::None,
                    ..default()
                },
                BorderColor::all(colour(Role::Interface)),
                BackgroundColor(shade(0)),
            ));
        });
}

/// Centre the screen in the room the party column leaves when it stands beside it.
fn make_room(beside: Res<Beside>, mut layer: Query<&mut Node, With<ScreenLayer>>) {
    let right = Val::Px(beside.0);
    for mut node in &mut layer {
        if node.padding.right != right {
            node.padding.right = right;
        }
    }
}

/// Open the screen the command line asked for (`--screen`).
fn open_start(
    start: Option<Res<StartScreen>>,
    session: Option<Res<Session>>,
    mut screens: ResMut<Screens>,
) {
    let game = session.as_deref().map(|s| &s.views);
    screens.reset(game);
    match start.as_deref() {
        Some(StartScreen::NewGame) => screens.nav.open(ScreenId::NewGame, game),
        Some(StartScreen::Pause) => screens.nav.open(ScreenId::Pause, game),
        Some(StartScreen::Actions) => screens.nav.open(ScreenId::Actions, game),
        Some(StartScreen::Verbs) => screens.nav.open(ScreenId::Verbs, game),
        Some(StartScreen::Log) => screens.nav.open(ScreenId::Log, game),
        Some(StartScreen::Inventory) => screens.nav.show(ScreenId::Inventory, game),
        Some(StartScreen::Title | StartScreen::CreateParty) | None => {}
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

/// The context of a running game, as its screens read it, with the default settings (the game's
/// screens do not read them; the router passes the applied ones, `ctx_with`).
#[must_use]
pub fn ctx(session: &Session) -> Ctx<'_> {
    Ctx {
        data: &session.data,
        game: Some(Game {
            world: &session.world,
            views: &session.views,
            log: &session.fight_log,
            unreplayable: session.unreplayable.as_deref(),
            history: &session.history,
        }),
        save: slot(&session.config.save),
        prefs: crate::settings::Prefs::default(),
        fits: crate::settings::SCALE_MAX,
    }
}

fn slot(path: &str) -> Slot<'_> {
    Slot {
        path,
        exists: game_io::exists(root(), path),
    }
}

/// The context with or without a game, with the settings as applied.
#[must_use]
pub fn ctx_with<'a>(
    session: Option<&'a Session>,
    packs: &'a Packs,
    prefs: crate::settings::Prefs,
    fits: u16,
) -> Ctx<'a> {
    let ctx = session.map_or_else(
        || Ctx {
            data: &packs.data,
            game: None,
            save: slot(&packs.config.save),
            prefs,
            fits,
        },
        ctx,
    );
    Ctx { prefs, fits, ..ctx }
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
    current: ResMut<'w, Current>,
    fitted: Res<'w, Fitted>,
}

/// The top screen's input, from its nodes and the keys.
#[derive(SystemParam)]
struct Used<'w, 's> {
    buttons: Query<'w, 's, (&'static Interaction, &'static Pressable), Changed<Interaction>>,
    arrows: Query<'w, 's, (&'static Interaction, &'static Arrow), Changed<Interaction>>,
    fields: Query<'w, 's, (Entity, Ref<'static, EditableText>, &'static Pressable)>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    slid: ResMut<'w, Slid>,
}

/// Esc, the square's action keys, a pressed control, a choice's arrow, typed text or a key,
/// routed to the top screen; the effect carried out.
fn route(
    mut used: Used,
    mut host: Host,
    mut session: Option<ResMut<Session>>,
    mut screens: ResMut<Screens>,
) {
    let screens = &mut *screens;
    let mut escaped = None;
    {
        let game = session.as_deref().map(|s| &s.views);
        let here = game.map(|v| v.position);
        let moved = screens.at.is_some() && here.is_some() && screens.at != here;
        screens.at = here;
        screens.nav.settle(game, moved);
        if used.keys.just_pressed(KeyCode::Escape) {
            let at_root = screens.forms.fight.menu.step == Step::Top;
            match screens.nav.escape(game, at_root) {
                Escape::FightBack => screens.forms.fight.menu.back(),
                Escape::Leave => escaped = Some(Effect::ToNewGame),
                Escape::LeaveService => {
                    escaped = Some(Effect::Command {
                        command: Command::Service(ServiceCommand::Leave),
                        close: false,
                    });
                }
                Escape::Closed | Escape::Paused | Escape::Nothing => {}
            }
        } else if used.keys.just_pressed(KeyCode::Backquote) {
            // The debug panel, as the retired client's backtick: opened where it is allowed.
            screens.nav.toggle(ScreenId::Debug, game);
        }
    }
    if let Some(effect) = escaped {
        carry_out(effect, &mut host, &mut session, screens);
        return;
    }
    if let Some(session) = session.as_deref_mut() {
        action_keys(&used.keys, &used.mouse, session, &mut screens.nav);
    }
    {
        let game = session.as_deref().map(|s| &s.views);
        screens
            .forms
            .shown(screens.nav.top(game), screens.nav.base(game));
    }
    let slid = used.slid.0.take();
    let Some(top) = screens.nav.top(session.as_deref().map(|s| &s.views)) else {
        return;
    };
    let effect = {
        let ctx = ctx_with(
            session.as_deref(),
            &host.packs,
            host.current.0,
            host.fitted.0,
        );
        let input = Input { used: &used, slid };
        applied(&input, &ctx, top, &mut screens.forms, &mut host.commands)
    };
    if let Some(effect) = effect {
        carry_out(effect, &mut host, &mut session, screens);
    }
}

/// This frame's input: the nodes and keys, and a slider's move.
struct Input<'a, 'w, 's> {
    used: &'a Used<'w, 's>,
    slid: Option<(ControlId, i64)>,
}

/// The control used this frame on screen `top`, applied: a press, an arrow, a slider, new text
/// in a field, or a number key. Text a field cannot take is put back to what the form holds.
fn applied(
    input: &Input,
    ctx: &Ctx,
    top: ScreenId,
    forms: &mut Forms,
    commands: &mut Commands,
) -> Option<Effect> {
    let used = input.used;
    let screen = screen::content(top, ctx, forms);
    let slid = || input.slid.map(|(id, n)| (id, Payload::Set(n)));
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
    let (id, payload) = pressed
        .or_else(stepped)
        .or_else(slid)
        .or_else(typed)
        .or_else(keyed)?;
    match screen::apply(top, ctx, forms, id, &payload, entropy()) {
        Ok(effect) => Some(effect),
        Err(_) if matches!(payload, Payload::Text(_)) => {
            // Text the form refused: the field shows what the form holds again.
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

/// Space or a right click opens or closes the square's actions while exploring; with Shift it
/// runs the default action, or opens them when there is none. R opens or closes the camp. L opens
/// the full log.
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
    // R opens the camp, or closes it, as its button does.
    if keys.just_pressed(KeyCode::KeyR)
        && matches!(nav.top(Some(&session.views)), None | Some(ScreenId::Camp))
    {
        nav.toggle(ScreenId::Camp, Some(&session.views));
    }
    // L opens the full log where no screen is up; it never closes it, for the filter takes
    // typing (Esc, Close or the button do).
    if keys.just_pressed(KeyCode::KeyL) && nav.top(Some(&session.views)).is_none() {
        nav.open(ScreenId::Log, Some(&session.views));
    }
    if !action || !free || !session.views.exploring() || session.views.combat.is_some() {
        return;
    }
    if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        match default_action(&session.world, &session.data) {
            Some(Does::Command(command)) => {
                session.order(command);
                nav.settle(Some(&session.views), true);
            }
            Some(Does::Choose) => nav.open(ScreenId::Verbs, Some(&session.views)),
            None => nav.open(ScreenId::Actions, Some(&session.views)),
        }
    } else {
        nav.toggle(ScreenId::Actions, Some(&session.views));
    }
}

/// Carry out what a screen asked for. Every effect but `None` clears the last outcome; a
/// command then reports its newest line, or its refusal, and the host's work its own.
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
                match session.ordered(command.clone()) {
                    Ok(line) => {
                        screens.forms.accepted(&command);
                        screens.forms.message = line.map(Span::body);
                    }
                    Err(why) => screens.forms.message = Some(Span::alert(why)),
                }
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
        Effect::ToTitle | Effect::ToNewGame => {
            host.commands.remove_resource::<Session>();
            host.view.set(ViewState::Explore);
            screens.reset(None);
            if effect == Effect::ToNewGame {
                screens.nav.open(ScreenId::NewGame, None);
            }
        }
        Effect::Host(op) => host_op(op, host, session, screens),
        Effect::Settings(prefs) => {
            host.current.0 = prefs;
            let path = &host.packs.config.settings;
            let outcome = crate::settings::write(root(), path, prefs)
                .map(|()| format!("Settings saved to {path}"))
                .map_err(|e| format!("Could not save settings: {e}"));
            report(screens, outcome);
        }
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
                    screens.reset(Some(&fresh.views));
                    host.commands.insert_resource(fresh);
                }
                Err(e) => report(screens, Err(format!("Could not start: {e}"))),
            }
        }
        HostOp::Restart => {
            if let Some(session) = session.as_deref_mut() {
                match session.restart() {
                    Ok(()) => screens.reset(Some(&session.views)),
                    Err(e) => session.say(format!("Could not start again: {e}")),
                }
            }
        }
        HostOp::Save => {
            if let Some(session) = session.as_deref() {
                let outcome = game_io::save(&session.world, &session.data, root(), &save)
                    .map(|_| format!("Saved to {save}"))
                    .map_err(|e| format!("Save failed: {e}"));
                report(screens, outcome);
            }
        }
        HostOp::Load => {
            let data = session
                .as_deref()
                .map_or_else(|| host.packs.data.clone(), |s| s.data.clone());
            // Examined first (M8.5 7h4): a save that loads, loads at once; one that does not
            // opens the report.
            let loaded = match game_io::examine(&data, root(), &save) {
                Ok(opened) if !opened.report().verdict.loads() => {
                    screens.forms.load_report = Some(Refusal {
                        check: opened.into_report(),
                        reload: false,
                    });
                    let views = session.as_deref().map(|s| &s.views);
                    screens.nav.open(ScreenId::LoadReport, views);
                    return;
                }
                Ok(opened) => opened.load(&data),
                Err(e) => Err(e),
            };
            match loaded {
                Ok(loaded) => {
                    let why = format!("the save {save} was loaded");
                    let lines = loaded.lines(&save);
                    if let Some(session) = session.as_deref_mut() {
                        session.replace(loaded.world, &why);
                        for line in lines {
                            session.say(line);
                        }
                        screens.reset(Some(&session.views));
                    } else {
                        let mut fresh = Session::loaded(&host.packs, loaded.world, &why);
                        for line in lines {
                            fresh.say(line);
                        }
                        screens.reset(Some(&fresh.views));
                        host.commands.insert_resource(fresh);
                    }
                }
                Err(e) => report(screens, Err(format!("Load failed: {e}"))),
            }
        }
        HostOp::Begin => screens.nav.begin(),
        HostOp::SaveLog => {
            if let Some(session) = session.as_deref() {
                let outcome = session
                    .save_log()
                    .map(|path| format!("Log saved to {}", path.display()))
                    .map_err(|e| format!("Log not saved: {e}"));
                report(screens, outcome);
            }
        }
        HostOp::Reload => {
            if let Some(session) = session.as_deref_mut() {
                // Refused for a name the fresh packs lack: the Load screen lists them.
                match Packs::load(&session.config).map(|p| session.reload(p.data)) {
                    Ok(Ok(check)) => {
                        let said = game_io::reloaded(&check).swap_remove(0);
                        report(screens, Ok(said));
                    }
                    Ok(Err(refused)) if !refused.check.verdict.loads() => {
                        screens.forms.load_report = Some(Refusal {
                            check: *refused.check,
                            reload: true,
                        });
                        screens.nav.open(ScreenId::LoadReport, Some(&session.views));
                    }
                    Ok(Err(refused)) => {
                        report(screens, Err(format!("Reload refused: {}", refused.error)))
                    }
                    Err(e) => report(screens, Err(format!("Reload failed: {e}"))),
                }
            }
        }
        HostOp::Tidy { delete } => match game_io::tidy(root(), delete) {
            Ok(tidied) => {
                let n = tidied.files.len();
                let files = if n == 1 { "file" } else { "files" };
                let said = if delete {
                    format!("Deleted {n} {files} no save named ({} bytes)", tidied.bytes)
                } else {
                    format!("{n} {files} no save names ({} bytes)", tidied.bytes)
                };
                screens
                    .forms
                    .debug
                    .tidied((!delete).then_some(tidied.files));
                report(screens, Ok(said));
            }
            Err(e) => report(screens, Err(format!("Tidy failed: {e}"))),
        },
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
    current: Res<Current>,
    fitted: Res<Fitted>,
    mut screens: ResMut<Screens>,
    mut showing: ResMut<Showing>,
    window: Query<&Window, With<PrimaryWindow>>,
    capture: Option<Res<CaptureSize>>,
    mut root: Query<(Entity, &mut Node), With<ScreenRoot>>,
    mut placed: Query<Placed>,
    sliders: Query<(Entity, &SliderOf, &SliderValue)>,
    focus: Option<Res<InputFocus>>,
    fields: Query<&Pressable, With<EditableText>>,
) {
    let Ok((entity, mut node)) = root.single_mut() else {
        return;
    };
    let top = screens.nav.top(session.as_deref().map(|s| &s.views));
    showing.0 = top.is_some();
    let room = Val::Vh(if played(session.as_deref(), &screens.nav) {
        PLAYED_ROOM
    } else {
        ALONE_ROOM
    });
    if node.max_height != room {
        node.max_height = room;
    }
    let Some(top) = top else {
        if screens.drawn.take().is_some() {
            commands.entity(entity).despawn_related::<Children>();
            node.display = Display::None;
        }
        return;
    };
    let ctx = ctx_with(session.as_deref(), &packs, current.0, fitted.0);
    let screen = screen::content(top, &ctx, &screens.forms);
    let height = size(window.single().ok(), capture.as_deref()).1;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bucket = height as u32;
    let same_shape = |drawn: &Screen| {
        drawn.shape() == screen.shape() && drawn.message.is_some() == screen.message.is_some()
    };
    match &screens.drawn {
        Some((drawn, h)) if *drawn == screen && *h == bucket => return,
        Some((drawn, h)) if same_shape(drawn) && *h == bucket => {
            sync(&screen, &mut placed);
            slide(&screen, &sliders, &mut commands);
        }
        _ => {
            // The field that has the keyboard keeps it through the rebuild when the screen still
            // has it (a field emptied dims its button, which changes the shape; owner,
            // 2026-10-17); a screen without it gives the keyboard to its own field.
            let keep = focus
                .as_ref()
                .and_then(|f| f.get())
                .and_then(|e| fields.get(e).ok())
                .map(|p| p.0)
                .filter(|id| screen.control(*id).is_some());
            commands.entity(entity).despawn_related::<Children>();
            node.display = Display::Flex;
            // The full log stands wide: most of the window (owner, 2026-10-17).
            let log = screen.id == ScreenId::Log;
            node.width = if log {
                Val::Percent(LOG_WIDE)
            } else {
                Val::Px(width(&screen, height))
            };
            // And as tall as the room allows, however few its lines.
            node.height = if log { room } else { Val::Auto };
            // Never wider than the window, at any interface scale: the panels scroll instead.
            node.max_width = Val::Percent(100.0);
            node.padding = UiRect::all(Val::Px(scaled(14.0, height)));
            node.row_gap = Val::Px(scaled(8.0, height));
            commands
                .entity(entity)
                .with_children(|p| build(p, &screen, (height, Mark::Screen), keep));
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
        let span = screen.panels.get(at.0)?.rows.get(at.1)?.span(at.2)?;
        return Some((span.text.clone(), span.role));
    }
    if let Some(of) = label {
        let c = screen.control(of.0)?;
        let role = if c.why_not.is_some() {
            Role::Dim
        } else {
            Role::Interface
        };
        return Some((shown_label(&screen.keys, c), role));
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

/// Set each slider to the number its control holds, so a slider follows the arrows and a move
/// the form refused springs back.
#[allow(clippy::cast_precision_loss)]
fn slide(
    screen: &Screen,
    sliders: &Query<(Entity, &SliderOf, &SliderValue)>,
    commands: &mut Commands,
) {
    for (entity, of, value) in sliders {
        if let Some(Control {
            kind: Kind::Number { value: n, .. },
            ..
        }) = screen.control(of.0)
        {
            let want = *n as f32;
            // The value is immutable: a new one is inserted, which moves the thumb.
            #[allow(clippy::float_cmp)]
            if value.0 != want {
                commands.entity(entity).insert(SliderValue(want));
            }
        }
    }
}

/// The full log stands at its newest line: when it is drawn anew, and whenever its lines change
/// (a filter typed). Elsewhere the wheel moves it (`screen_draw::wheel`).
fn foot(
    screens: Res<Screens>,
    mut last: Local<Option<u64>>,
    mut panels: Query<&mut ScrollPosition, With<Scrolls>>,
) {
    use std::hash::{Hash, Hasher};
    let Some((screen, _)) = screens
        .drawn
        .as_ref()
        .filter(|(s, _)| s.id == ScreenId::Log)
    else {
        *last = None;
        return;
    };
    let mut h = std::collections::hash_map::DefaultHasher::new();
    screen.panels.hash(&mut h);
    let lines = Some(h.finish());
    let changed = *last != lines;
    *last = lines;
    for mut at in &mut panels {
        if changed || at.is_added() {
            at.0.y = SCROLL_FOOT;
        }
    }
}

/// The full log's width, in percent of the window.
const LOG_WIDE: f32 = 90.0;

/// The most a screen's height takes, in percent of the window's, over a game being played: the
/// room between the HUD's lines above and the button groups below. A taller screen's panels
/// scroll.
const PLAYED_ROOM: f32 = 64.0;

/// The most a screen's height takes with no game chrome: the window less the gaps.
const ALONE_ROOM: f32 = 95.0;

/// Whether a game is being played, and so its HUD, buttons and minimap show: not at the title,
/// nor while its party is made.
fn played(session: Option<&Session>, nav: &Nav) -> bool {
    session.is_some_and(|s| nav.base(Some(&s.views)) != Some(ScreenId::CreateParty))
}

/// The game's own nodes: the HUD (its status and log), the button groups and the minimap.
type GameChrome = Or<(With<Hud>, With<Corner>, With<Minimap>)>;

/// The game's HUD, buttons and minimap show only while a game is played: not at the title, nor
/// while its party is made.
fn chrome(
    session: Option<Res<Session>>,
    screens: Res<Screens>,
    mut nodes: Query<&mut Visibility, GameChrome>,
) {
    let want = if played(session.as_deref(), &screens.nav) {
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
