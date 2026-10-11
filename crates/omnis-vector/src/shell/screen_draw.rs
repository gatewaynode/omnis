//! A screen's nodes (presentation-ARCHITECTURE §6.4): the heading, the panels side by side, each
//! row's text and controls, and the outcome line, drawn in the style's roles and sizes. A button
//! is a bordered box; a choice its name between two arrows; a number its name, arrows and a
//! slider; a text field an underlined editable line. The markers here let the router find what
//! was used and the renderer rewrite words in place. Headless-safe.

use crate::screen::{Control, ControlId, Key, Kind, Screen};
use crate::style::{Role, SHADES, Size, scaled};
use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick};

/// A control's node: pressing it, or typing in it, uses the control.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressable(pub ControlId);

/// One of a choice's or a number's arrows: `true` steps right.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrow(pub ControlId, pub bool);

/// A number's slider: moving it sets the number.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderOf(pub ControlId);

/// The words of control `.0` (a choice's current option, a number's value).
#[derive(Component)]
pub(super) struct LabelOf(pub(super) ControlId);

/// The words of span `(panel, row, span)`; see [`crate::screen::Row::span`].
#[derive(Component)]
pub(super) struct SpanAt(pub(super) usize, pub(super) usize, pub(super) usize);

/// The heading's words.
#[derive(Component)]
pub(super) struct TitleText;

/// The outcome's words, under the panels.
#[derive(Component)]
pub(super) struct MessageText;

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

/// The editable line of a text field holding `value`, taking at most `max` characters.
pub(super) fn text_field(value: &str, max: usize) -> EditableText {
    let mut field = EditableText::new(value);
    field.max_characters = Some(max);
    field
}

/// A text field's cursor and selection, in the interface's colours.
#[must_use]
pub fn cursor_style() -> TextCursorStyle {
    TextCursorStyle {
        color: colour(Role::Interface),
        selection_color: shade(1),
        unfocused_selection_color: shade(0),
        selected_text_color: None,
    }
}

/// A control's words as drawn: its number key first when it has one, its reason after when it
/// is dim; a choice shows its current option, a number its value.
pub(super) fn shown_label(screen: &Screen, control: &Control) -> String {
    match &control.kind {
        Kind::Choice { options, at } => return options.get(*at).cloned().unwrap_or_default(),
        Kind::Number { value, .. } => return value.to_string(),
        Kind::Button | Kind::Text { .. } => {}
    }
    match &control.why_not {
        Some(why) => format!("{}: {why}", control.label),
        None => match screen.keys.iter().find(|(_, c)| *c == control.id) {
            Some((Key::Digit(n), _)) => format!("{n}  {}", control.label),
            None => control.label.clone(),
        },
    }
}

/// The words of a role and size, at window height `height`.
pub(super) fn words(
    text: &str,
    role: Role,
    size: Size,
    height: f32,
) -> (Text, TextFont, TextColor) {
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
            // At least this tall; a long reason wraps inside the box rather than out of it.
            min_height: Val::Px(scaled(34.0, height)),
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

/// A choice's or a number's arrow: a small pressable box.
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

/// The thumb's width, as a share of the window height's scale.
const THUMB: f32 = 12.0;

/// A number's slider: a track, and a thumb the [`thumbs`] system places from the value. The
/// slider reports a move; the router sets the number, and the renderer the slider's value.
#[allow(clippy::cast_precision_loss)]
fn slider(parent: &mut ChildSpawnerCommands, id: ControlId, value: i64, range: (i64, i64), h: f32) {
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
            Node {
                width: Val::Px(scaled(140.0, h)),
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

/// A control's nodes. A button is a bordered box with its words; a dim one has no `Button`, so it
/// takes no clicks, and is drawn in the dim role. A choice is its name, two arrows and the
/// current option between them; a number adds a slider. A text field is its name and an
/// editable line.
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
        Kind::Number { value, min, max } => {
            parent.spawn(boxed(role, height)).with_children(|b| {
                b.spawn(words(&c.label, role, Size::Body, height));
                arrow(b, c.id, false, height);
                slider(b, c.id, *value, (*min, *max), height);
                b.spawn((
                    LabelOf(c.id),
                    words(&shown_label(screen, c), role, Size::Body, height),
                    Node {
                        min_width: Val::Px(scaled(22.0, height)),
                        ..default()
                    },
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
                    // Without a style no cursor is drawn; Bevy's default is too dark to see here.
                    cursor_style(),
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

/// The width a screen takes: one panel's, and more for each panel beside it.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub(super) fn width(screen: &Screen, height: f32) -> f32 {
    let beside = screen.panels.len().saturating_sub(1) as f32;
    scaled(560.0 + 460.0 * beside, height)
}

/// Build the screen's nodes under the root: the heading, the panels side by side, the outcome.
pub(super) fn build(parent: &mut ChildSpawnerCommands, screen: &Screen, height: f32) {
    let t = &screen.title;
    parent.spawn((TitleText, words(&t.text, t.role, t.size, height)));
    let gap = Val::Px(scaled(6.0, height));
    parent
        .spawn(Node {
            column_gap: Val::Px(scaled(24.0, height)),
            ..default()
        })
        .with_children(|columns| {
            for (p, panel) in screen.panels.iter().enumerate() {
                columns
                    // Panels share the width equally, so one panel's long rows never squeeze
                    // the next into wrapping.
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        min_width: Val::Px(0.0),
                        row_gap: Val::Px(scaled(8.0, height)),
                        ..default()
                    })
                    .with_children(|column| {
                        for (r, row) in panel.rows.iter().enumerate() {
                            column
                                .spawn(Node {
                                    flex_wrap: FlexWrap::Wrap,
                                    align_items: AlignItems::Center,
                                    column_gap: gap,
                                    row_gap: gap,
                                    ..default()
                                })
                                .with_children(|line| {
                                    let before = row.spans.len();
                                    for (s, span) in row.spans.iter().enumerate() {
                                        line.spawn((
                                            SpanAt(p, r, s),
                                            words(&span.text, span.role, span.size, height),
                                        ));
                                    }
                                    for c in &row.controls {
                                        control_node(line, screen, c, height);
                                    }
                                    for (s, span) in row.after.iter().enumerate() {
                                        line.spawn((
                                            SpanAt(p, r, before + s),
                                            words(&span.text, span.role, span.size, height),
                                        ));
                                    }
                                });
                        }
                    });
            }
        });
    if let Some(m) = &screen.message {
        parent.spawn((MessageText, words(&m.text, m.role, m.size, height)));
    }
}
