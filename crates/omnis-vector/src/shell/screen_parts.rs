//! The small parts a control is built of (`screen_draw`): a choice's or a number's arrows, a
//! number's slider and its thumb, a tick box. Headless-safe.

use super::controls::pressable;
use super::screen_draw::{
    Arrow, SliderOf, Spawn, colour, labelled, pressed_by, shade, shown_label, words,
};
use crate::screen::{Control, ControlId, Key};
use crate::style::{Role, Size, scaled};
use bevy::prelude::*;
use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick};

/// The side of a tick box.
const TICK: f32 = 26.0;

/// A tick box: a small pressable square, an x in it when it is set.
pub(super) fn tick(
    parent: &mut ChildSpawnerCommands,
    keys: &[(Key, ControlId)],
    c: &Control,
    role: Role,
    spawn: Spawn,
) {
    let side = Val::Px(scaled(TICK, spawn.height));
    let mut entity = parent.spawn((
        Node {
            width: side,
            height: side,
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(colour(role)),
        BackgroundColor(shade(0)),
    ));
    pressed_by(&mut entity, spawn.mark, c.id);
    if c.why_not.is_none() {
        entity.insert(pressable());
    }
    entity.with_children(|b| {
        labelled(
            b,
            words(&shown_label(keys, c), role, Size::Body, spawn.height),
            c.id,
            spawn,
        );
    });
}

/// A choice's or a number's arrow: a small pressable box.
pub(super) fn arrow(parent: &mut ChildSpawnerCommands, id: ControlId, right: bool, height: f32) {
    parent
        .spawn((
            Arrow(id, right),
            pressable(),
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

/// The thumb's width, as a share of the window height's scale.
const THUMB: f32 = 12.0;

/// A number's slider: a track, and a thumb the [`thumbs`] system places from the value. The
/// slider reports a move; the router sets the number, and the renderer the slider's value.
#[allow(clippy::cast_precision_loss)]
pub(super) fn slider(
    parent: &mut ChildSpawnerCommands,
    id: ControlId,
    value: i64,
    range: (i64, i64),
    (h, bare): (f32, bool),
) {
    parent
        .spawn((
            SliderOf(id),
            Slider {
                track_click: TrackClick::Snap,
                ..default()
            },
            SliderValue(value as f32),
            SliderRange::new(range.0 as f32, range.1 as f32),
            SliderStep(1.0),
            // In a field's row the track takes what the arrows and the number leave.
            Node {
                width: if bare {
                    Val::Auto
                } else {
                    Val::Px(scaled(140.0, h))
                },
                min_width: Val::Px(scaled(if bare { 36.0 } else { 140.0 }, h)),
                flex_grow: if bare { 1.0 } else { 0.0 },
                flex_basis: if bare { Val::Px(0.0) } else { Val::Auto },
                height: Val::Px(scaled(THUMB, h)),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|s| {
            // The track.
            s.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(scaled(2.0, h).max(1.0)),
                    ..default()
                },
                BackgroundColor(colour(Role::Dim)),
            ));
            // The thumb travels the width less its own.
            s.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(scaled(THUMB, h)),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            })
            .with_children(|travel| {
                travel.spawn((
                    SliderThumb,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(scaled(THUMB, h)),
                        height: Val::Px(scaled(THUMB, h)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(colour(Role::Interface)),
                    BackgroundColor(colour(Role::Interface)),
                ));
            });
        });
}

/// Place each slider's thumb from its value.
pub(super) fn thumbs(
    sliders: Query<(Entity, &SliderValue, &SliderRange), Changed<SliderValue>>,
    children: Query<&Children>,
    mut thumb: Query<&mut Node, With<SliderThumb>>,
) {
    for (entity, value, range) in &sliders {
        for below in children.iter_descendants(entity) {
            if let Ok(mut node) = thumb.get_mut(below) {
                node.left = Val::Percent(range.thumb_position(value.0) * 100.0);
            }
        }
    }
}
