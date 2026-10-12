//! The party column's nodes and input (presentation-ARCHITECTURE §6.6): `screen::roster`
//! describes it, this module draws it at the right and routes its buttons. In explore it stands
//! under the minimap; in a fight at the arena's party rectangle. It shows only while a game is
//! played and no screen is up, so it never sits under the pause or a service. Headless-safe.

use super::capture::CaptureSize;
use super::combat::size;
use super::minimap::Minimap;
use super::screen::{Beside, Screens, Showing, Side, colour, ctx, shade};
use super::screen_draw::{Mark, rows};
use super::session::Session;
use crate::arena::{BUTTONS, MARGIN, layout, zoom};
use crate::screen::roster::{self, RosterEffect};
use crate::screen::{Panel, ScreenId};
use crate::style::{Role, scaled};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Marks the node the column is drawn under.
#[derive(Component)]
pub struct RosterRoot;

/// The column as last drawn, and where: `None` while it is hidden.
#[derive(Resource, Debug, Default)]
pub(super) struct Drawn(Option<(Panel, Placement)>);

/// Where the column stands, in interface units (logical pixels over `UiScale`): its top, its
/// width, its foot's distance from the window's bottom and the window's height, and the height
/// its type is scaled for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Placement {
    top: f32,
    width: f32,
    bottom: f32,
    room: f32,
    height: f32,
}

pub(super) fn spawn(mut commands: Commands) {
    commands.spawn((
        RosterRoot,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(MARGIN),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(1.0)),
            overflow: Overflow::clip(),
            display: Display::None,
            ..default()
        },
        BorderColor::all(colour(Role::Interface)),
        BackgroundColor(shade(0)),
    ));
}

/// Whether the column shows: a game is played (not while its party is made) and no screen is
/// up but the column's own and the screens it stands beside (`ScreenId::beside_column`).
fn shown(session: Option<&Session>, screens: &Screens, showing: Showing) -> bool {
    session.is_some_and(|s| {
        let views = Some(&s.views);
        let beside = screens.nav.top(views).is_some_and(ScreenId::beside_column);
        (!showing.0 || beside) && screens.nav.base(views) != Some(ScreenId::CreateParty)
    })
}

/// Whether a screen stands beside the column.
fn beside(session: &Session, screens: &Screens) -> bool {
    screens
        .nav
        .top(Some(&session.views))
        .is_some_and(ScreenId::beside_column)
}

/// A pressed button of the column, applied: a member selected, or their sheet, kit or spells
/// opened.
pub(super) fn route(
    session: Option<Res<Session>>,
    mut screens: ResMut<Screens>,
    showing: Res<Showing>,
    buttons: Query<(&Interaction, &Side), Changed<Interaction>>,
) {
    let Some(session) = session else {
        screens.forms.selected = None;
        return;
    };
    let ctx = ctx(&session);
    let Some(game) = ctx.game else {
        return;
    };
    screens.forms.selected = roster::settle(game, screens.forms.selected);
    if !shown(Some(&session), &screens, *showing) {
        return;
    }
    let Some(id) = buttons
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, s)| s.0)
    else {
        return;
    };
    let mut selected = screens.forms.selected;
    match roster::apply(ctx.data, game, &mut selected, id) {
        Ok(RosterEffect::None) => screens.forms.selected = selected,
        Ok(RosterEffect::Open { tool, member, .. }) => {
            if let Some(screen) = tool.screen() {
                screens.forms.owner = Some(member);
                screens.nav.show(screen, Some(&session.views));
            }
        }
        Err(_) => {}
    }
}

/// Draw the column where it stands now, rebuilt when what it shows or where it stands changed.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint(
    mut commands: Commands,
    session: Option<Res<Session>>,
    screens: Res<Screens>,
    showing: Res<Showing>,
    fight: Option<Res<State<super::ViewState>>>,
    window: Query<&Window, With<PrimaryWindow>>,
    capture: Option<Res<CaptureSize>>,
    minimap: Query<&ComputedNode, With<Minimap>>,
    ui: Option<Res<UiScale>>,
    mut root: Query<(Entity, &mut Node), With<RosterRoot>>,
    mut drawn: ResMut<Drawn>,
    mut room: ResMut<Beside>,
) {
    let Ok((entity, mut node)) = root.single_mut() else {
        return;
    };
    let session = session.as_deref();
    let Some(session) = session.filter(|s| shown(Some(s), &screens, *showing)) else {
        room.set_if_neq(Beside(0.0));
        if drawn.0.take().is_some() {
            commands.entity(entity).despawn_related::<Children>();
            node.display = Display::None;
        }
        return;
    };
    let (w, h) = size(window.single().ok(), capture.as_deref());
    let in_fight = fight.is_some_and(|f| *f.get() == super::ViewState::Fight);
    let scale = ui.map_or(1.0, |u| u.0);
    let place = if in_fight {
        // The arena's units to the interface's (`combat::FightScreen`).
        let z = zoom(w, h, scale);
        let r = z / scale;
        let party = layout(w / z, h / z).party;
        Placement {
            top: party.y * r,
            width: party.w * r,
            bottom: (h / z - party.bottom()) * r,
            room: h / scale,
            height: h * r,
        }
    } else {
        // Under the minimap, whose size is its picture's (its side until it is laid out).
        #[allow(clippy::cast_precision_loss)]
        let side = crate::minimap::SIDE as f32;
        let map = minimap
            .single()
            .map(|c| c.size().y * c.inverse_scale_factor())
            .ok()
            .filter(|h| *h > 0.0)
            .unwrap_or(side);
        Placement {
            top: MARGIN + map + MARGIN,
            width: (w / scale * 0.24).clamp(300.0, 640.0),
            bottom: MARGIN + BUTTONS + MARGIN,
            room: h / scale,
            height: h,
        }
    };
    room.set_if_neq(Beside(if beside(session, &screens) {
        place.width + 2.0 * MARGIN
    } else {
        0.0
    }));
    let ctx = ctx(session);
    let Some(game) = ctx.game else {
        return;
    };
    let panel = roster::content(ctx.data, game, screens.forms.selected);
    if drawn.0.as_ref() == Some(&(panel.clone(), place)) {
        return;
    }
    commands.entity(entity).despawn_related::<Children>();
    node.display = Display::Flex;
    node.top = Val::Px(place.top);
    node.width = Val::Px(place.width);
    node.max_height = Val::Px((place.room - place.top - place.bottom).max(0.0));
    node.padding = UiRect::all(Val::Px(scaled(10.0, place.height)));
    node.row_gap = Val::Px(scaled(6.0, place.height));
    commands
        .entity(entity)
        .with_children(|column| rows(column, 0, &panel, &[], place.height, Mark::Side));
    drawn.0 = Some((panel, place));
}
