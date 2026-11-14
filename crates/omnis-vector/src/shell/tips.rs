//! Tooltips (owner, 2026-11-13): a control that carries a tip (`screen::Control::tip`) shows it
//! in a small box beside itself once the pointer has rested on it, or while it holds the
//! keyboard. The box is a child of the control's node placed by Bevy's `ui_widgets::Popover`,
//! which keeps it inside the window; it is ordinary text, so `screen.text` reads it while it
//! shows. Headless-safe.

use super::screen_draw::words;
use super::theme::{edge, fill};
use crate::style::{REFERENCE_HEIGHT, Role, Size, scaled};
use bevy::input_focus::InputFocus;
use bevy::picking::Pickable;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{OverrideClip, Pressed};
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};

/// What a node is for, shown beside it. On a field's row the box goes to the row's side
/// (`beside`), off the fields under it; elsewhere under or over the control.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
#[require(Hovered)]
pub struct Tip {
    /// The words.
    pub text: String,
    /// Whether the box goes to the node's left or right before under or over it.
    pub beside: bool,
}

/// A tip's box, a child of the node it explains.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TipBox;

/// How long the pointer rests on a node before its tip shows, in seconds.
pub const REST: f32 = 0.4;

/// Above every screen and the editor's columns.
const TIP_Z: i32 = 50;

/// What the pointer rests on, and since when.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rest<T> {
    on: Option<T>,
    since: f32,
}

impl<T> Default for Rest<T> {
    fn default() -> Rest<T> {
        Rest {
            on: None,
            since: 0.0,
        }
    }
}

impl<T: Copy + PartialEq> Rest<T> {
    /// The pointer is over `over` at `now` seconds: what it has rested on for [`REST`], if
    /// anything. Moving to another node, or off every one, starts the wait again.
    pub fn due(&mut self, over: Option<T>, now: f32) -> Option<T> {
        if self.on != over {
            self.on = over;
            self.since = now;
        }
        self.on.filter(|_| now - self.since >= REST)
    }

    /// Something was pressed at `now`: the wait starts again where the pointer is.
    pub fn pressed(&mut self, now: f32) {
        self.since = now;
    }
}

/// The node with a tip that `entity` is or lies in.
fn holder(
    entity: Entity,
    tips: &Query<(Entity, &Tip, &Hovered)>,
    up: &Query<&ChildOf>,
) -> Option<Entity> {
    core::iter::once(entity)
        .chain(up.iter_ancestors(entity))
        .find(|e| tips.contains(*e))
}

/// Show the tip of what the pointer has rested on, else of what holds the keyboard; take the
/// box away when neither holds.
fn show(
    (time, focus): (Res<Time<Real>>, Option<Res<InputFocus>>),
    tips: Query<(Entity, &Tip, &Hovered)>,
    pressed: Query<(), With<Pressed>>,
    up: Query<&ChildOf>,
    boxes: Query<(Entity, &ChildOf), With<TipBox>>,
    mut rest: Local<Rest<Entity>>,
    mut commands: Commands,
) {
    let now = time.elapsed_secs();
    let over = tips.iter().find(|(_, _, h)| h.get()).map(|(e, _, _)| e);
    // A press starts the pointer's wait again, which takes its tip away.
    if !pressed.is_empty() {
        rest.pressed(now);
    }
    let rested = rest.due(over, now);
    let focused = focus
        .as_ref()
        .and_then(|f| f.get())
        .and_then(|e| holder(e, &tips, &up));
    // While the pointer is on another node its wait is not cut short by the keyboard's tip
    // staying up: the keyboard's shows until the pointer's is due.
    let wanted = rested.or(focused);
    let mut shown = false;
    for (entity, parent) in &boxes {
        if Some(parent.parent()) == wanted {
            shown = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if shown {
        return;
    }
    let Some((target, tip, _)) = wanted.and_then(|e| tips.get(e).ok()) else {
        return;
    };
    let box_ = commands
        .spawn((
            TipBox,
            popover(tip.beside),
            OverrideClip,
            GlobalZIndex(TIP_Z),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                max_width: Val::Px(scaled(300.0, REFERENCE_HEIGHT)),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            edge(Role::Title),
            fill(0),
            children![(
                words(&tip.text, Role::Title, Size::Small, REFERENCE_HEIGHT),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands.entity(target).add_child(box_);
}

/// Where a tip's box may go, the first that fits the window: beside a field's row, or under
/// and over any other control.
fn popover(beside: bool) -> Popover {
    let at = |side| PopoverPlacement {
        side,
        align: PopoverAlign::Start,
        gap: 8.0,
    };
    let (left, right) = (at(PopoverSide::Left), at(PopoverSide::Right));
    let (under, over) = (at(PopoverSide::Bottom), at(PopoverSide::Top));
    Popover {
        positions: if beside {
            vec![left, right, under, over]
        } else {
            vec![under, over, right, left]
        },
        window_margin: 8.0,
    }
}

/// The tips, and Bevy's placing of their boxes.
pub struct TipsPlugin;

impl Plugin for TipsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::ui_widgets::popover::PopoverPlugin>() {
            app.add_plugins(bevy::ui_widgets::popover::PopoverPlugin);
        }
        app.add_systems(Update, show);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tip_is_due_after_the_pointer_has_rested_and_the_wait_starts_again_on_a_move() {
        let mut rest = Rest::default();
        assert_eq!(rest.due(Some(1), 0.0), None);
        assert_eq!(rest.due(Some(1), REST - 0.01), None, "not yet");
        assert_eq!(rest.due(Some(1), REST), Some(1));
        assert_eq!(
            rest.due(Some(2), REST + 0.1),
            None,
            "another node waits anew"
        );
        assert_eq!(rest.due(Some(2), 2.0 * REST + 0.1), Some(2));
        assert_eq!(rest.due(None, 3.0), None, "off every node");
        assert_eq!(rest.due(Some(2), 3.1), None, "back on it waits anew");
        rest.pressed(3.4);
        assert_eq!(
            rest.due(Some(2), 3.6),
            None,
            "a press starts the wait again"
        );
        assert_eq!(rest.due(Some(2), 3.4 + REST), Some(2));
    }
}
