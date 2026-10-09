//! The camp as a `bevy_ui` panel over the map (M7 step 8b): one row per member with its hit
//! points and hit dice and, for a member who may spend some, a slider; the short rest, the long
//! rest with its food and why it would be refused, the last refusal and Close. Everything shown
//! comes from `omnis_sim::rest_view` through `camp_panel.rs`, which also answers every report.
//! The entity tree is spawned again when its shape changes (a slider's range); counts, notes
//! and refusals are rewritten in place. An ambush takes the game into the fight
//! (`combat::follow_mode`), and the panel goes with the screen.

use crate::camp_panel::{self as model, CampAsk, CampForm, CampLabelId, CampPanelId};
use crate::feathers_ui::HoldsKeyboard;
use crate::menus::{Active, Where};
use crate::sim::{CommandRefused, PlayState, PlayerCommand, SimEvent, Views};
use crate::ui_kit::{
    Control, PanelRoot, Shown, UiId, UiLabel, UiReport, UiScreen, button, message_line,
    panel as panel_root, row, set_text, title,
};
use bevy::feathers::containers::flex_spacer;
use bevy::feathers::controls::{ButtonVariant, FeathersSlider};
use bevy::feathers::display::{label, label_dim};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{SliderPrecision, SliderStep, SliderValue};
use omnis_sim::{Command, Event, RestView};

/// The camp as the panel last read it, and what the panel keeps.
#[derive(Resource, Debug, Default)]
pub struct CampShown {
    /// The rests where the party stands, while the panel is up.
    pub view: Option<RestView>,
    /// The dice chosen and the last refusal.
    pub form: CampForm,
    /// Whether the widgets show all of the above.
    synced: bool,
}

// ------------------------------------------------------------------ scenes

/// A member: the line, and a slider with its count, or why none may be spent.
fn member_row(slot: usize, spendable: u8) -> impl Scene {
    let picker: Vec<_> = (spendable > 0)
        .then(|| {
            bsn! {
                @FeathersSlider { @min: 0.0, @max: {f32::from(spendable)}, @value: 0.0 }
                Control({UiId::Camp(CampPanelId::Dice(slot))})
                SliderStep(1.)
                SliderPrecision(0)
                Node { width: px(160), flex_shrink: 0.0 }
            }
        })
        .into_iter()
        .collect();
    bsn! {
        row()
        Children [
            (
                Node { flex_grow: 1.0, flex_basis: px(0) }
                Children [ (label("") Shown({UiLabel::Camp(CampLabelId::Member(slot))})) ]
            ),
            {picker},
            (
                Node { width: px(120), flex_shrink: 0.0 }
                Children [ (label_dim("") Shown({UiLabel::Camp(CampLabelId::Dice(slot))})) ]
            ),
        ]
    }
}

/// The whole panel.
fn camp_panel(view: &RestView) -> impl Scene {
    let members: Vec<_> = view
        .members
        .iter()
        .enumerate()
        .map(|(slot, member)| member_row(slot, member.spendable))
        .collect();
    bsn! {
        panel_root()
        Children [
            (
                row()
                Children [
                    title("Camp"),
                    flex_spacer(),
                    (label("") Shown({UiLabel::Camp(CampLabelId::Food)})),
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                }
                Children [ {members} ]
            ),
            (
                row()
                Children [
                    button(CampPanelId::Short.into(), "Short rest", ButtonVariant::Normal),
                    label_dim("An hour; each member spends the dice chosen"),
                ]
            ),
            (
                row()
                Children [
                    button(CampPanelId::Long.into(), "Long rest", ButtonVariant::Normal),
                    (label_dim("") Shown({UiLabel::Camp(CampLabelId::Long)})),
                ]
            ),
            message_line(CampLabelId::Message.into()),
            flex_spacer(),
            (
                row()
                Children [
                    flex_spacer(),
                    button(CampPanelId::Close.into(), "Close", ButtonVariant::Primary),
                ]
            ),
        ]
    }
}

// ------------------------------------------------------------------ systems

/// Read the camp again whenever the world changed while the panel is up; forget it, and the
/// choices, when the panel is down.
pub fn look(at: Where, views: Option<Res<Views>>, mut shown: ResMut<CampShown>) {
    let Some(views) = views else {
        return;
    };
    if at.screen() != Active::Camp {
        if shown.view.is_some() {
            *shown = CampShown::default();
        }
        return;
    }
    if shown.view.is_none() || views.is_changed() {
        let view = views.rest.clone();
        shown.form.fit(&view);
        shown.view = Some(view);
        shown.synced = false;
    }
}

/// A rest taken clears the choices; anything the rules did clears the message; a refusal while
/// the panel is up is the message.
pub fn refusals(
    at: Where,
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    mut shown: ResMut<CampShown>,
) {
    for SimEvent(event) in events.read() {
        if matches!(event, Event::Rested { .. }) {
            shown.form.dice.iter_mut().for_each(|d| *d = 0);
        }
        shown.form.message.clear();
        shown.synced = false;
    }
    for CommandRefused(rejection) in refused.read() {
        if at.screen() == Active::Camp {
            shown.form.message = rejection.to_string();
            shown.synced = false;
        }
    }
}

/// Spawn, despawn or spawn again, so a panel exists exactly while the camp is open, and always
/// for the current shape.
pub fn reconcile(
    mut commands: Commands,
    mut shown: ResMut<CampShown>,
    roots: Query<(Entity, &PanelRoot)>,
) {
    let shape = shown.view.as_ref().map(model::shape);
    let mut standing = false;
    for (entity, root) in &roots {
        if root.screen != UiScreen::Camp {
            continue;
        }
        if Some(root.shape) == shape && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let (Some(view), Some(shape), false) = (&shown.view, shape, standing) {
        commands.spawn_scene(camp_panel(view)).insert(PanelRoot {
            screen: UiScreen::Camp,
            shape,
        });
        shown.synced = false;
    }
}

/// Write the view into the panel: the lines, the counts, the reasons, the sliders, and the
/// rests that would be refused dimmed.
pub fn sync(
    mut commands: Commands,
    views: Option<Res<Views>>,
    mut shown: ResMut<CampShown>,
    controls: Query<(
        Entity,
        &Control,
        Has<InteractionDisabled>,
        Option<&SliderValue>,
    )>,
    mut labels: Query<(&Shown, &mut Text)>,
) {
    let (Some(view), Some(views)) = (shown.view.as_ref(), views) else {
        return;
    };
    let ours = controls
        .iter()
        .any(|(_, c, _, _)| matches!(c.0, UiId::Camp(_)));
    if shown.synced || !ours {
        return;
    }
    for (shown_id, mut text) in &mut labels {
        let UiLabel::Camp(id) = shown_id.0 else {
            continue;
        };
        let wanted = match id {
            CampLabelId::Food => model::food_line(view),
            CampLabelId::Member(slot) => model::member_line(view, &views.party, slot),
            CampLabelId::Dice(slot) => model::dice_note(view, &shown.form, slot),
            CampLabelId::Long => model::long_note(view),
            CampLabelId::Message => shown.form.message.clone(),
        };
        set_text(&mut text, &wanted);
    }
    for (entity, control, disabled, slider) in &controls {
        let dim = match control.0 {
            UiId::Camp(CampPanelId::Short) => view.refusal.is_some() || !shown.form.spends(),
            UiId::Camp(CampPanelId::Long) => view.long.is_some(),
            UiId::Camp(CampPanelId::Dice(slot)) => {
                let wanted = f32::from(shown.form.dice.get(slot).copied().unwrap_or(0));
                if slider.map(|s| s.0) != Some(wanted) {
                    commands.entity(entity).insert(SliderValue(wanted));
                }
                continue;
            }
            _ => continue,
        };
        if dim && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !dim && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    shown.synced = true;
}

/// Answer the panel's reports: each goes through `camp_panel::apply`; a rest goes to the
/// simulation, Close back to the map.
pub(crate) fn reports(
    mut reports: MessageReader<UiReport>,
    mut shown: ResMut<CampShown>,
    views: Option<Res<Views>>,
    mut out: MessageWriter<PlayerCommand>,
    mut next: ResMut<NextState<PlayState>>,
) {
    for report in reports.read() {
        let UiId::Camp(id) = report.id else {
            continue;
        };
        let CampShown { view, form, synced } = &mut *shown;
        let Some(view) = view.as_ref() else {
            continue;
        };
        *synced = false;
        let ids: Vec<_> = views
            .as_deref()
            .map_or_else(Vec::new, |v| v.party.members.iter().map(|m| m.id).collect());
        match model::apply(id, &report.payload, view, &ids, form) {
            Some(CampAsk::Rest(rest)) => {
                out.write(PlayerCommand(Command::Rest(rest)));
            }
            Some(CampAsk::Close) => close(views.as_deref(), &mut next),
            None => {}
        }
    }
}

fn close(views: Option<&Views>, next: &mut NextState<PlayState>) {
    next.set(views.map_or(PlayState::Explore, |v| PlayState::for_kind(v.here.mode)));
}

/// Escape closes the camp, unless a control holds the keyboard.
pub(crate) fn escape_closes(
    mut keys: MessageReader<KeyboardInput>,
    at: Where,
    focus: Res<InputFocus>,
    holders: Query<(), HoldsKeyboard>,
    views: Option<Res<Views>>,
    mut next: ResMut<NextState<PlayState>>,
) {
    let escaped = keys
        .read()
        .any(|k| k.state == ButtonState::Pressed && k.logical_key == Key::Escape);
    let held = focus.get().is_some_and(|entity| holders.contains(entity));
    if escaped && at.screen() == Active::Camp && !held {
        close(views.as_deref(), &mut next);
    }
}
