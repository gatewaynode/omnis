//! The document control (M5.9c, owner 2026-10-24): a whole file as text with line numbers down
//! its left, a vertical and a horizontal scrollbar, and the view following the caret. Bevy's text
//! input scrolls inside its own box (`TextScroll`), which no scrollbar can show, so the document
//! scrolls in a container instead: the field is laid out whole (as many lines as the text has, as
//! wide as its longest line, no wrapping) beside a gutter of numbers, inside a node that scrolls on
//! both axes, so the numbers and the lines move together with no system between them.

use super::screen_draw::Scrolls;
use crate::style::{Role, Size, scaled};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextGeneration, LineBreak, TextLayout, TextLayoutInfo};
use bevy::ui::RelativeCursorPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

/// A document's field: the text the gutter and the view follow.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentField;

/// A document's gutter: the line numbers of the field it names.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gutter(pub Entity);

/// The scrollbar's thickness, at the reference height.
const BAR: f32 = 6.0;

/// How many lines a text has, as the gutter counts them: one more than its newlines.
#[must_use]
pub fn lines(text: &str) -> u32 {
    u32::try_from(text.matches('\n').count() + 1).unwrap_or(u32::MAX)
}

/// The longest line of a text, in characters (a tab counts four), never under one.
#[must_use]
pub fn longest_line(text: &str) -> u32 {
    text.lines()
        .map(|l| {
            l.chars()
                .map(|c| if c == '\t' { 4 } else { 1 })
                .sum::<usize>()
        })
        .max()
        .map_or(1, |n| u32::try_from(n).unwrap_or(u32::MAX).max(1))
}

/// The gutter's words for `n` lines: `1` to `n`, one a line.
#[must_use]
pub fn numbers(n: u32) -> String {
    let mut out = String::new();
    for i in 1..=n.max(1) {
        if i > 1 {
            out.push('\n');
        }
        out.push_str(&i.to_string());
    }
    out
}

/// Where a view of `view` size scrolled to `scroll` goes so that `caret` (in the content's
/// coordinates) lies inside it: moved the least that does it, never above or left of the start.
#[must_use]
pub fn follow(view: Vec2, caret: Rect, scroll: Vec2) -> Vec2 {
    let axis = |at: f32, size: f32, lo: f32, hi: f32| {
        if lo < at {
            lo
        } else if hi > at + size {
            hi - size
        } else {
            at
        }
        .max(0.0)
    };
    Vec2::new(
        axis(scroll.x, view.x, caret.min.x, caret.max.x),
        axis(scroll.y, view.y, caret.min.y, caret.max.y),
    )
}

/// The document's field: every line visible, as wide as the longest, newlines allowed, no wrap.
#[must_use]
pub fn document_field(value: &str, max: usize) -> EditableText {
    let mut field = EditableText::new(value);
    field.max_characters = Some(max);
    field.allow_newlines = true;
    size_field(&mut field, value);
    field
}

/// Set the field's visible lines and width from its text.
fn size_field(field: &mut EditableText, text: &str) {
    #[allow(clippy::cast_precision_loss)]
    {
        field.visible_lines = Some(lines(text) as f32);
        field.visible_width = Some(longest_line(text) as f32 + 1.0);
    }
}

/// A document's nodes under `parent`: the scrolling area (the gutter and the field side by
/// side) with its vertical bar beside it and its horizontal bar under both. The field is
/// returned for the caller's markers (its press, its focus).
pub(super) fn spawn(
    parent: &mut ChildSpawnerCommands,
    value: &str,
    max: usize,
    height: f32,
    with_field: impl FnOnce(&mut EntityCommands),
) {
    let gap = scaled(12.0, height);
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            width: Val::Percent(100.0),
            min_height: Val::Px(0.0),
            min_width: Val::Px(0.0),
            row_gap: Val::Px(scaled(4.0, height)),
            ..default()
        })
        .with_children(|outer| {
            let mut area_id = Entity::PLACEHOLDER;
            outer
                .spawn(Node {
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    min_width: Val::Px(0.0),
                    column_gap: Val::Px(scaled(4.0, height)),
                    ..default()
                })
                .with_children(|top| {
                    area_id = top
                        .spawn((
                            Scrolls,
                            Node {
                                flex_grow: 1.0,
                                min_width: Val::Px(0.0),
                                min_height: Val::Px(0.0),
                                align_items: AlignItems::FlexStart,
                                column_gap: Val::Px(gap),
                                overflow: Overflow::scroll(),
                                ..default()
                            },
                            ScrollPosition::default(),
                            RelativeCursorPosition::default(),
                        ))
                        .with_children(|area| {
                            let gutter = area
                                .spawn((
                                    super::screen_draw::words(
                                        &numbers(lines(value)),
                                        Role::Dim,
                                        Size::Body,
                                        height,
                                    ),
                                    TextLayout {
                                        justify: Justify::Right,
                                        linebreak: LineBreak::NoWrap,
                                    },
                                    Node {
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                ))
                                .id();
                            let mut field = area.spawn((
                                DocumentField,
                                document_field(value, max),
                                TextFont {
                                    font_size: FontSize::Px(Size::Body.px(height)),
                                    ..default()
                                },
                                TextColor(super::screen_draw::colour(Role::Interface)),
                                super::screen_draw::cursor_style(),
                                TextLayout {
                                    linebreak: LineBreak::NoWrap,
                                    ..default()
                                },
                                Node {
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                            ));
                            with_field(&mut field);
                            let field = field.id();
                            area.commands().entity(gutter).insert(Gutter(field));
                        })
                        .id();
                    bar(top, area_id, ControlOrientation::Vertical, height);
                });
            bar(outer, area_id, ControlOrientation::Horizontal, height);
        });
}

/// A scrollbar for `target` along one axis, shown by `screen_draw::bars` while the content runs
/// past the area that way.
fn bar(parent: &mut ChildSpawnerCommands, target: Entity, along: ControlOrientation, height: f32) {
    let thick = Val::Px(scaled(BAR, height).max(3.0));
    let node = match along {
        ControlOrientation::Vertical => Node {
            width: thick,
            flex_shrink: 0.0,
            ..default()
        },
        ControlOrientation::Horizontal => Node {
            height: thick,
            flex_shrink: 0.0,
            ..default()
        },
    };
    parent
        .spawn((
            Scrollbar::new(target, along, scaled(24.0, height)),
            node,
            BackgroundColor(super::screen_draw::shade(1)),
            Visibility::Hidden,
        ))
        .with_children(|b| {
            b.spawn((
                ScrollbarThumb::default(),
                BackgroundColor(super::screen_draw::colour(Role::Interface)),
            ));
        });
}

/// Keep every document's field as tall and wide as its text, and its gutter numbered to its
/// lines. The field reads as changed every frame in the window (Bevy's layout takes it mutably),
/// so nothing is written unless a size differs.
pub(super) fn size_documents(
    mut fields: Query<Mut<EditableText>, With<DocumentField>>,
    mut gutters: Query<(&Gutter, &mut Text)>,
) {
    for (gutter, mut text) in &mut gutters {
        let Ok(mut field) = fields.get_mut(gutter.0) else {
            continue;
        };
        if !field.is_changed() {
            continue;
        }
        let value = field.value().to_string();
        let n = lines(&value);
        #[allow(clippy::cast_precision_loss)]
        let want = (Some(n as f32), Some(longest_line(&value) as f32 + 1.0));
        if (field.visible_lines, field.visible_width) != want {
            size_field(&mut field, &value);
        }
        let words = numbers(n);
        if text.0 != words {
            text.0 = words;
        }
    }
}

/// What `follow_caret` reads of a field: the editor's generation (changed by an edit or a move,
/// not by the caret's blink), the layout with the caret, the node, its place and its area.
type Caret = (
    Ref<'static, EditableTextGeneration>,
    &'static TextLayoutInfo,
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static ChildOf,
);

/// Scroll the document's area so the focused field's caret stays in view after an edit or a
/// move (the editor's generation changes then, not on the caret's blink).
pub(super) fn follow_caret(
    focus: Option<Res<InputFocus>>,
    fields: Query<Caret, With<DocumentField>>,
    mut areas: Query<(&ComputedNode, &UiGlobalTransform, &mut ScrollPosition), With<Scrolls>>,
) {
    let Some(focused) = focus.as_ref().and_then(|f| f.get()) else {
        return;
    };
    let Ok((generation, info, node, transform, parent)) = fields.get(focused) else {
        return;
    };
    if !generation.is_changed() {
        return;
    }
    let Some((_, caret)) = info.cursor else {
        return;
    };
    let Ok((area, area_transform, mut scroll)) = areas.get_mut(parent.parent()) else {
        return;
    };
    let k = area.inverse_scale_factor();
    // The transforms give the nodes' centres in physical pixels; the field is drawn where the
    // area's scroll put it, so the scroll is added back to place the caret in the content.
    let field_min = transform.translation - node.size() / 2.0;
    let area_min = area_transform.translation - area.size() / 2.0;
    let offset = (field_min - area_min) * k + scroll.0;
    let caret = Rect {
        min: caret.min * k + offset,
        max: caret.max * k + offset,
    };
    let next = follow(area.size() * k, caret, scroll.0);
    if next != scroll.0 {
        scroll.0 = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_the_longest_line_count_as_the_gutter_does() {
        assert_eq!(lines(""), 1);
        assert_eq!(lines("a"), 1);
        assert_eq!(lines("a\nb"), 2);
        assert_eq!(
            lines("a\nb\n"),
            3,
            "a trailing newline opens a last, empty line"
        );
        assert_eq!(longest_line(""), 1);
        assert_eq!(longest_line("ab\ncdef\ng"), 4);
        assert_eq!(longest_line("\tx"), 5, "a tab counts four");
        assert_eq!(numbers(3), "1\n2\n3");
        assert_eq!(numbers(0), "1");
    }

    #[test]
    fn the_view_follows_the_caret_the_least_it_must() {
        let view = Vec2::new(100.0, 50.0);
        let inside = Rect::new(10.0, 10.0, 12.0, 20.0);
        assert_eq!(
            follow(view, inside, Vec2::ZERO),
            Vec2::ZERO,
            "in view: unmoved"
        );
        let below = Rect::new(10.0, 120.0, 12.0, 130.0);
        assert_eq!(follow(view, below, Vec2::ZERO), Vec2::new(0.0, 80.0));
        let right = Rect::new(250.0, 10.0, 252.0, 20.0);
        assert_eq!(follow(view, right, Vec2::ZERO), Vec2::new(152.0, 0.0));
        let above_left = Rect::new(5.0, 5.0, 7.0, 15.0);
        assert_eq!(
            follow(view, above_left, Vec2::new(40.0, 60.0)),
            Vec2::new(5.0, 5.0),
            "back to the caret's start"
        );
        assert_eq!(
            follow(view, Rect::new(-3.0, -3.0, 1.0, 1.0), Vec2::new(10.0, 10.0)),
            Vec2::ZERO,
            "never before the start"
        );
    }
}
