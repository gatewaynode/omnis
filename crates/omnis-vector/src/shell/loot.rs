//! What opens the loot window besides a click on a pile (`shell/aim.rs`): a find, laid on the
//! ground by the engine (owner, 2026-11-13: it is not left behind for want of looking); the
//! "Pick up..." button that stands over the view's bottom while anything on the ground is in
//! reach, the party's own cell included, where a pile is below the view; and Shift+Space, the
//! default action. All of them go through [`open`].

use super::controls::{Control, Unpressable, button, pressable};
use super::editor::Editing;
use super::screen::Screens;
use super::session::Session;
use super::theme::{Edge, Ink};
use super::{VectorSet, ViewState};
use crate::screen::ScreenId;
use crate::screen::actions::{Does, default_action};
use crate::screen::loot::Loot;
use crate::sim::Views;
use crate::style::Role;
use bevy::prelude::*;
use bevy::ui::Pressed;
use omnis_sim::ItemPlace;

/// The loot window on `from`, as nothing left it.
pub(super) fn open(screens: &mut Screens, views: Option<&Views>, from: ItemPlace) {
    screens.forms.loot = Loot {
        from,
        ..Loot::default()
    };
    screens.nav.open(ScreenId::Loot, views);
}

/// What an object gave lies on the ground: the loot window opens on it at once, over whatever
/// screen the verb came from, so it is not left behind for want of looking (owner,
/// 2026-11-13). Where the world does not allow the window, the find stays a pile.
fn found(
    session: Option<ResMut<Session>>,
    screens: Option<ResMut<Screens>>,
    state: Option<Res<State<ViewState>>>,
) {
    let (Some(mut session), Some(mut screens)) = (session, screens) else {
        return;
    };
    // A won fight's screen is up for one frame more: the spoils wait for the map's view.
    if state.is_some_and(|s| *s.get() == ViewState::Fight) || session.offer.is_none() {
        return;
    }
    let Some(from) = session.offer.take() else {
        return;
    };
    // A find taken by the same command that made it leaves nothing to show.
    if from != ItemPlace::Ground || !session.views.objects.ground.is_empty() {
        open(&mut screens, Some(&session.views), from);
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
    mut borders: Query<&mut Edge>,
    mut texts: Query<&mut Ink>,
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
                border.0 = Role::Interface;
            }
            for &child in children {
                if let Ok(mut text) = texts.get_mut(child) {
                    text.0 = Role::Interface;
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
        open(&mut screens, Some(&session.views), ItemPlace::Ground);
    }
}

/// Space or a right click opens or closes the square's actions while exploring; with Shift it
/// runs the default action, or opens them when there is none. R opens or closes the camp. L opens
/// the full log.
pub(super) fn action_keys(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    session: &mut Session,
    screens: &mut Screens,
) {
    let nav = &mut screens.nav;
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
            Some(Does::Loot(from)) => open(screens, Some(&session.views), from),
            // The default is never an attack: that is asked for by name.
            Some(Does::Attack) | None => nav.open(ScreenId::Actions, Some(&session.views)),
        }
    } else {
        nav.toggle(ScreenId::Actions, Some(&session.views));
    }
}

/// Registers the window's openers.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn)
        .add_systems(Update, (pick_up, found).in_set(VectorSet::Input))
        .add_systems(Update, offer.in_set(VectorSet::Screens));
}
