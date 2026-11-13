//! The pointer on a pile (owner, 2026-11-12): while the party explores with no screen up, a
//! pile within its reach can be clicked where it is drawn. The pointer over one is a hand, and
//! a left click there opens the loot window on the ground instead of taking the mouse look;
//! with the mouse look on, the middle of the view is the pointer. The geometry is `crate::aim`.
//! While anything on the ground is in reach, the party's own cell included, a "Pick up..."
//! button stands over the view's bottom and opens the same window (owner, 2026-11-13: a pile
//! underfoot is below the view).

use super::controls::{Control, GREEN, Over, Unpressable, button, over_control, pressable};
use super::editor::Editing;
use super::movement::{Intent, View};
use super::render::FOV;
use super::screen::Screens;
use super::session::Session;
use super::{VectorSet, ViewState};
use crate::aim::{Eye, pile_under};
use crate::ground::{Ground, eye};
use crate::screen::ScreenId;
use crate::screen::loot::Loot;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use omnis_sim::omnis_core::{Level, MapId};

/// The cell of the pile in the party's reach that the pointer is on, if any: a click there is
/// the pile's, not the view's, and the cell's floor is outlined (`render::ViewAim`).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Aimed(pub Option<(u16, u16)>);

/// The floor of the party's level, kept until the map or the level changes.
#[derive(Default)]
pub(super) struct Floor(Option<((MapId, Level), Ground)>);

/// What the aim reads besides the input.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Seen<'w, 's> {
    windows: Query<'w, 's, (Entity, &'static Window), With<PrimaryWindow>>,
    session: Option<Res<'w, Session>>,
    view: Res<'w, View>,
    intent: Res<'w, Intent>,
    state: Option<Res<'w, State<ViewState>>>,
    editing: Option<Res<'w, Editing>>,
    controls: Over<'w, 's>,
}

impl Seen<'_, '_> {
    /// The cell of the pile in reach that the pointer is on, with no screen on `top`.
    fn on_pile(&self, top: Option<ScreenId>, floor: &mut Floor) -> Option<(u16, u16)> {
        let session = self.session.as_deref()?;
        let fighting = self
            .state
            .as_deref()
            .is_some_and(|s| *s.get() == ViewState::Fight);
        if top.is_some() || fighting || self.editing.is_some() || over_control(&self.controls) {
            return None;
        }
        let data = &session.data;
        let pos = session.world.exploring_at(data)?;
        let piles = session.world.piles_in_reach(data);
        let (Some(map), Ok((_, window)), false) = (
            data.maps.get(&pos.map),
            self.windows.single(),
            piles.is_empty(),
        ) else {
            return None;
        };
        let size = (window.width(), window.height());
        let point = if self.intent.looking {
            Some((size.0 / 2.0, size.1 / 2.0))
        } else {
            window.cursor_position().map(|p| (p.x, p.y))
        };
        let point = point?;
        let key = (pos.map, pos.level);
        if floor.0.as_ref().is_none_or(|(k, _)| *k != key) {
            floor.0 = Some((key, Ground::from_map(map, pos.level)));
        }
        let (_, ground) = floor.0.as_ref()?;
        let pose = session.pose;
        let eye = Eye {
            at: [pose.x, eye(ground, pose), pose.z],
            yaw: pose.yaw,
            pitch: self.view.pitch,
            fov: FOV,
        };
        pile_under(point, size, &eye, &piles, ground).map(|i| (piles[i].x, piles[i].y))
    }
}

/// The pointer's shape follows what it is on, and a left click on a pile opens the loot window.
pub(super) fn aim(
    seen: Seen,
    mouse: Res<ButtonInput<MouseButton>>,
    mut screens: Option<ResMut<Screens>>,
    mut floor: Local<Floor>,
    mut aimed: ResMut<Aimed>,
    mut commands: Commands,
) {
    let views = seen.session.as_deref().map(|s| &s.views);
    let top = screens.as_deref().and_then(|s| s.nav.top(views));
    let on = screens
        .is_some()
        .then(|| seen.on_pile(top, &mut floor))
        .flatten();
    let hit = on.is_some();
    if on != aimed.0 {
        aimed.0 = on;
        if let Ok((window, _)) = seen.windows.single() {
            commands.entity(window).insert(CursorIcon::System(if hit {
                SystemCursorIcon::Pointer
            } else {
                SystemCursorIcon::Default
            }));
        }
    }
    if let (true, true, Some(screens)) = (
        hit,
        mouse.just_pressed(MouseButton::Left),
        screens.as_deref_mut(),
    ) {
        open(screens, views);
    }
}

/// The loot window on the ground in reach, as nothing left it.
fn open(screens: &mut Screens, views: Option<&crate::sim::Views>) {
    screens.forms.loot = Loot::default();
    screens.nav.open(ScreenId::Loot, views);
}

/// What an object gave lies on the ground: the loot window opens on it at once, over whatever
/// screen the verb came from, so it is not left behind for want of looking (owner,
/// 2026-11-13). Where the world does not allow the window, the find stays a pile.
fn found(session: Option<ResMut<Session>>, screens: Option<ResMut<Screens>>) {
    let (Some(mut session), Some(mut screens)) = (session, screens) else {
        return;
    };
    if !session.found {
        return;
    }
    session.found = false;
    if !session.views.objects.ground.is_empty() {
        open(&mut screens, Some(&session.views));
    }
}

/// Marks the "Pick up..." button over the view.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickUp;

/// How far above the window's bottom the button stands, in pixels: over the two rows of the
/// movement pad and the side buttons, whatever the window's width.
const PICK_UP_BOTTOM: f32 = 102.0;

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Px(PICK_UP_BOTTOM),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|layer| {
            button(layer, "Pick up...", (PickUp, Visibility::Hidden), false);
        });
}

/// What the button reads of the world.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Reach<'w> {
    session: Option<Res<'w, Session>>,
    screens: Option<Res<'w, Screens>>,
    state: Option<Res<'w, State<ViewState>>>,
    editing: Option<Res<'w, Editing>>,
}

impl Reach<'_> {
    /// Whether something on the ground is in the party's reach while it explores with no
    /// screen up: what the Actions menu's "Pick up..." asks, on the view itself.
    fn offered(&self) -> bool {
        let (Some(session), Some(screens)) = (self.session.as_deref(), self.screens.as_deref())
        else {
            return false;
        };
        let views = &session.views;
        let fighting = self
            .state
            .as_deref()
            .is_some_and(|s| *s.get() == ViewState::Fight);
        views.exploring()
            && !fighting
            && self.editing.is_none()
            && screens.nav.top(Some(views)).is_none()
            && !views.objects.ground.is_empty()
    }
}

/// Show the button and let it be pressed while something is in reach; hide it, and take the
/// press from it, otherwise: a hidden button takes no click.
fn offer(
    mut commands: Commands,
    reach: Reach,
    mut buttons: Query<(Entity, Has<Control>, &mut Visibility, &Children), With<PickUp>>,
    mut borders: Query<&mut BorderColor>,
    mut texts: Query<&mut TextColor>,
) {
    let live = reach.offered();
    for (entity, lit, mut visibility, children) in &mut buttons {
        if lit == live {
            continue;
        }
        if live {
            commands.entity(entity).insert(pressable());
            *visibility = Visibility::Inherited;
            if let Ok(mut border) = borders.get_mut(entity) {
                *border = BorderColor::all(GREEN);
            }
            for &child in children {
                if let Ok(mut text) = texts.get_mut(child) {
                    text.0 = GREEN;
                }
            }
        } else {
            commands.entity(entity).remove::<Unpressable>();
            *visibility = Visibility::Hidden;
        }
    }
}

/// A press on the button opens the loot window.
fn pick_up(
    pressed: Query<Ref<Pressed>, (With<PickUp>, With<Control>)>,
    session: Option<Res<Session>>,
    screens: Option<ResMut<Screens>>,
) {
    let (Some(session), Some(mut screens)) = (session, screens) else {
        return;
    };
    if pressed.iter().any(|p| p.is_added()) {
        open(&mut screens, Some(&session.views));
    }
}

/// Registers the aim ahead of the mouse look's own click.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Aimed>()
        .add_systems(Startup, spawn)
        .add_systems(Update, aim.before(VectorSet::Grab))
        .add_systems(Update, (pick_up, found).in_set(VectorSet::Input))
        .add_systems(Update, offer.in_set(VectorSet::Screens));
}
