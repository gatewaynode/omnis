//! The fight screen (presentation-ARCHITECTURE.md §9): while monsters are met or fought, a 2D screen
//! replaces the 3D view. `arena` lays it out and draws it, `combat_menu` says what can be done;
//! this module only wires them. `CombatPlugin` is headless-safe: the state, the picture, title,
//! action column, roll log and name labels as UI nodes, and the buttons, number keys, Esc and
//! clicks. `CombatViewPlugin` needs a window or the capture: the 2D camera and the lines.

use super::capture::{CaptureSize, Offscreen};
use super::cinema::Screen;
use super::controls::{Action, Corner, GREEN, SHADES, button};
use super::minimap::Minimap;
use super::render::ViewCamera;
use super::session::Session;
use super::{VectorSet, ViewState};
use crate::arena::{Arena, Rect, Tone, arena, pick};
use crate::cinema::opening;
use crate::combat_menu::{Act, CombatMenu, Pick, Step};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use omnis_sim::combat_view;

/// The window size used when there is neither a window nor a capture: a headless test's.
const HEADLESS: (f32, f32) = (1600.0, 900.0);
/// The roll log's padding.
const PAD: f32 = 10.0;
/// The render layer the fight's camera and lines are on; the 3D view's are on layer 0.
const LAYER: usize = 1;

/// The fight screen's state: the menu's step, where the pointer rests, and the screen as laid
/// out for the world as it stands.
#[derive(Resource, Debug, Default)]
pub struct FightScreen {
    /// The action menu.
    pub menu: CombatMenu,
    /// The clickable target under the pointer.
    pub hover: Option<Pick>,
    /// What a click can choose now.
    pub targets: Vec<Pick>,
    /// The screen as last laid out; `None` outside a fight.
    pub arena: Option<Arena>,
    /// What the screen was laid out for: accepted commands, the menu's step and the size.
    shown: Option<(usize, Step, (u32, u32))>,
}

/// A button of the action column, carrying what it does.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Choose(pub Act);

/// Marks the fight screen's UI; its children are rebuilt when the screen changes.
#[derive(Component)]
pub struct FightRoot;

/// The state, the screen's UI and its input.
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ViewState>()
            .init_resource::<FightScreen>()
            .add_systems(OnEnter(ViewState::Fight), (enter, give_way::<true>))
            .add_systems(OnExit(ViewState::Fight), (leave, give_way::<false>))
            .add_systems(Update, track.in_set(VectorSet::Input))
            .add_systems(
                Update,
                (input, refresh, hover)
                    .chain()
                    .in_set(VectorSet::Input)
                    .run_if(in_state(ViewState::Fight)),
            );
    }
}

/// The view follows the simulation: the fight screen while monsters are met or fought.
fn track(
    session: Res<Session>,
    state: Res<State<ViewState>>,
    mut next: ResMut<NextState<ViewState>>,
) {
    let want = if combat_view(&session.world, &session.data).is_some() {
        ViewState::Fight
    } else {
        ViewState::Explore
    };
    if *state.get() != want {
        next.set(want);
    }
}

fn enter(mut commands: Commands, mut screen: ResMut<FightScreen>) {
    *screen = FightScreen::default();
    commands.spawn((
        FightRoot,
        DespawnOnExit(ViewState::Fight),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    ));
}

/// Back to exploring. The pose is already where the party is: every order follows it
/// (`Session::order`), a retreat's too.
fn leave(mut screen: ResMut<FightScreen>) {
    *screen = FightScreen::default();
}

/// The nodes that give way to the fight screen.
type GivesWay = Or<(With<Corner>, With<Action>, With<Minimap>)>;

/// In a fight the movement pad, the action menu's button and the minimap give way; they come
/// back after it.
fn give_way<const FIGHT: bool>(
    mut nodes: Query<(&mut Node, Option<&Corner>, Option<&Action>), GivesWay>,
) {
    let to = if FIGHT { Display::None } else { Display::Flex };
    for (mut node, corner, action) in &mut nodes {
        let away = match (corner, action) {
            (Some(corner), _) => *corner == Corner::Left,
            (None, Some(action)) => *action == Action::Actions,
            (None, None) => true,
        };
        if away {
            node.display = to;
        }
    }
}

/// The window's size in logical pixels: the window's, the capture's, or a headless default.
fn size(window: Option<&Window>, capture: Option<&CaptureSize>) -> (f32, f32) {
    #[allow(clippy::cast_precision_loss)]
    match (window, capture) {
        (Some(w), _) => (w.width(), w.height()),
        (None, Some(c)) => (c.0 as f32, c.1 as f32),
        (None, None) => HEADLESS,
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

/// A pressed button or its number key carries out its entry; Esc steps back; a left click on a
/// clickable figure chooses it as the target.
fn input(
    buttons: Query<(&Interaction, &Choose), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut session: ResMut<Session>,
    mut screen: ResMut<FightScreen>,
) {
    let screen = &mut *screen;
    let mut act = buttons
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, c)| c.0.clone());
    if act.is_none()
        && let Some(n) = DIGITS.iter().position(|k| keys.just_pressed(*k))
        && let Some(entry) = screen
            .menu
            .entries(&session.world, &session.data)
            .into_iter()
            .nth(n)
        && entry.blocked.is_none()
    {
        act = Some(entry.act);
    }
    let mut command = act.and_then(|a| screen.menu.choose(&a));
    if keys.just_pressed(KeyCode::Escape) {
        screen.menu.back();
    }
    if command.is_none()
        && mouse.just_pressed(MouseButton::Left)
        && let Some(point) = window.single().ok().and_then(Window::cursor_position)
        && let Some(target) = screen.arena.as_ref().and_then(|a| pick(a, point.into()))
    {
        command = screen.menu.pick(&session.world, &session.data, target);
    }
    if let Some(command) = command {
        session.order(command);
    }
}

/// Lay the screen out again and rebuild its UI when a command was accepted, the menu moved or
/// the window changed size.
fn refresh(
    mut commands: Commands,
    session: Res<Session>,
    mut screen: ResMut<FightScreen>,
    window: Query<&Window, With<PrimaryWindow>>,
    capture: Option<Res<CaptureSize>>,
    root: Query<Entity, With<FightRoot>>,
) {
    let (w, h) = size(window.single().ok(), capture.as_deref());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let key = (
        session.binder.log.len(),
        screen.menu.step,
        (w as u32, h as u32),
    );
    if screen.shown == Some(key) {
        return;
    }
    let Ok(root) = root.single() else {
        return;
    };
    screen.shown = Some(key);
    let (world, data) = (&session.world, &session.data);
    screen.targets = screen.menu.clickable(world, data);
    screen.hover = None;
    screen.arena = arena(world, data, (w, h), &screen.targets, None);
    commands.entity(root).despawn_related::<Children>();
    let Some(laid) = &screen.arena else {
        return;
    };
    let l = laid.layout;
    let title = CombatMenu::default().prompt(world, data);
    let prompt = (screen.menu.step != Step::Top).then(|| screen.menu.prompt(world, data));
    let entries = screen.menu.entries(world, data);
    commands.entity(root).with_children(|p| {
        if let Some(scene) = opening(world) {
            p.spawn((Screen(scene), at(&l.picture)));
        }
        p.spawn((at(&l.title), Text::new(title), font(22.0), TextColor(GREEN)));
        p.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                overflow: Overflow::clip(),
                ..at(&l.actions)
            },
            BackgroundColor(SHADES[0]),
        ))
        .with_children(|column| {
            if let Some(prompt) = prompt {
                column.spawn((Text::new(prompt), font(18.0), TextColor(GREEN)));
            }
            for (i, e) in entries.iter().enumerate() {
                let label = match &e.blocked {
                    None => format!("{}  {}", i + 1, e.label),
                    Some(why) => format!("{}: {why}", e.label),
                };
                button(column, &label, Choose(e.act.clone()), e.blocked.is_none());
            }
        });
        p.spawn((
            // The newest line at the bottom; the oldest are clipped off the top.
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(PAD)),
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::clip(),
                ..at(&l.log)
            },
            BorderColor::all(GREEN),
            BackgroundColor(SHADES[0]),
        ))
        .with_children(|panel| {
            for line in &session.fight_log {
                panel.spawn((
                    Text::new(line.as_str()),
                    font(15.0),
                    TextColor(GREEN),
                    // Kept whole: a wrapped line grows rather than being squeezed.
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
            }
        });
        for g in laid.groups() {
            p.spawn(Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..at(&g.label.rect)
            })
            .with_children(|label| {
                label.spawn((
                    Text::new(g.label.text.as_str()),
                    font(15.0),
                    TextColor(GREEN),
                ));
            });
        }
    });
}

/// The clickable target under the pointer, framed brighter than the others.
fn hover(window: Query<&Window, With<PrimaryWindow>>, mut screen: ResMut<FightScreen>) {
    let screen = &mut *screen;
    let Some(laid) = screen.arena.as_mut() else {
        return;
    };
    let under = window
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .and_then(|p| pick(laid, p.into()))
        .filter(|p| screen.targets.contains(p));
    if under != screen.hover {
        screen.hover = under;
        laid.marks.hover = under;
    }
}

/// An absolutely placed node over `r`.
fn at(r: &Rect) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(r.x),
        top: Val::Px(r.y),
        width: Val::Px(r.w),
        height: Val::Px(r.h),
        ..default()
    }
}

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}

/// The fight's lines.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct CombatGizmos;

/// Marks the fight's camera.
#[derive(Component)]
pub struct FightCamera;

/// The fight's 2D camera and its lines. Needs a window or the capture.
pub struct CombatViewPlugin;

impl Plugin for CombatViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<CombatGizmos>()
            .add_systems(Startup, (spawn_camera, line_style))
            .add_systems(OnEnter(ViewState::Fight), cameras::<true>)
            .add_systems(OnExit(ViewState::Fight), cameras::<false>)
            .add_systems(
                Update,
                draw.in_set(VectorSet::Draw)
                    .run_if(in_state(ViewState::Fight)),
            );
    }
}

/// The 2D camera, off until a fight. Like the 3D view's it has HDR and bloom, so the lines
/// glow the same; it sees only the fight's layer.
fn spawn_camera(mut commands: Commands, offscreen: Option<Res<Offscreen>>) {
    let mut camera = commands.spawn((
        FightCamera,
        Camera2d,
        Camera {
            is_active: false,
            ..default()
        },
        Hdr,
        Tonemapping::None,
        Bloom {
            intensity: 0.25,
            ..Bloom::NATURAL
        },
        RenderLayers::layer(LAYER),
    ));
    if let Some(target) = offscreen {
        camera.insert(RenderTarget::Image(target.0.clone().into()));
    }
}

fn line_style(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<CombatGizmos>();
    config.line.width = 2.0;
    config.render_layers = RenderLayers::layer(LAYER);
}

/// The 3D view's camera, apart from the fight's.
type ViewOnly = (With<ViewCamera>, Without<FightCamera>);

/// Swap the cameras: the 2D one on and drawing the UI in a fight, the 3D one otherwise.
fn cameras<const FIGHT: bool>(
    mut commands: Commands,
    mut view: Query<(Entity, &mut Camera), ViewOnly>,
    mut fight: Query<(Entity, &mut Camera), With<FightCamera>>,
) {
    let (Ok((view, mut view_camera)), Ok((fight, mut fight_camera))) =
        (view.single_mut(), fight.single_mut())
    else {
        return;
    };
    view_camera.is_active = !FIGHT;
    fight_camera.is_active = FIGHT;
    let (on, off) = if FIGHT { (fight, view) } else { (view, fight) };
    commands.entity(off).remove::<IsDefaultUiCamera>();
    commands.entity(on).insert(IsDefaultUiCamera);
}

/// The colour of each kind of line: figures bright green, the fallen dim, the dead red, the one
/// acting amber, targets cyan, the hovered target white. Values above 1.0 bloom.
fn colour(tone: Tone) -> LinearRgba {
    let (r, g, b) = match tone {
        Tone::Figure => (0.15, 3.0, 0.5),
        Tone::Down => (0.05, 0.7, 0.15),
        Tone::Dead => (2.6, 0.2, 0.15),
        Tone::Bar => (0.08, 0.9, 0.25),
        Tone::Health => (0.2, 2.2, 0.4),
        Tone::Acting => (3.2, 1.6, 0.2),
        Tone::Target => (0.2, 1.8, 3.0),
        Tone::Hover => (3.0, 3.0, 3.0),
    };
    LinearRgba::rgb(r, g, b)
}

/// Draw the screen's lines. The layout's y runs down from the top left; the camera's runs up
/// from the centre.
fn draw(screen: Res<FightScreen>, mut gizmos: Gizmos<CombatGizmos>) {
    let Some(laid) = &screen.arena else {
        return;
    };
    let (w, h) = (laid.layout.window.w, laid.layout.window.h);
    let to = |(x, y): (f32, f32)| Vec2::new(x - w / 2.0, h / 2.0 - y);
    for s in crate::arena::segments(laid) {
        gizmos.line_2d(to(s.a), to(s.b), colour(s.tone));
    }
}
