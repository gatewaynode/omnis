//! A screen's nodes (presentation-ARCHITECTURE §6.4): the heading, the panels side by side, each
//! row's text and controls, and the outcome line, drawn in the style's roles and sizes. A button
//! is a bordered box; a choice its name between two arrows; a number its name, arrows and a
//! slider; a text field an underlined editable line; an icon a square box with its image
//! (`shell/icons.rs` draws and keeps the images). The markers here let the router find what
//! was used and the renderer rewrite words in place. Headless-safe.

use super::controls::pressable;
use super::icons::{self, IconOf, NeedsIcon};
pub(super) use super::screen_parts::thumbs;
use super::screen_parts::{arrow, slider, tick};
use super::theme::{Ink, edge, fill, ink, solid};
use super::tips::Tip;
use crate::screen::{Control, ControlId, Key, Kind, Panel, Screen, ScreenId};
use crate::style::{Role, SHADES, Size, scaled};
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::input_focus::{AutoFocus, FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui::RelativeCursorPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb, TextInput};

/// A control's node: pressing it, or typing in it, uses the control.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressable(pub ControlId);

/// One of a choice's or a number's arrows: `true` steps right.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrow(pub ControlId, pub bool);

/// A button of the party column: pressing it uses the column's control, never the top screen's.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Side(pub ControlId);

/// A control of the fight's screen: pressing it uses the fight's control, never the top
/// screen's.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightPress(pub ControlId);

/// A control of the editor's layer (M5.4): pressing it uses the editor's control, never the
/// top screen's.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorPress(pub ControlId);

/// Which router a panel's controls go to, and so how its nodes are marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mark {
    /// The top screen's: `Pressable`, and the markers its words are rewritten in place by.
    Screen,
    /// The party column's: `Side`, redrawn whole when it changes.
    Side,
    /// The fight's: `FightPress`, redrawn whole when it changes.
    Fight,
    /// The editor's columns: `EditorPress` (its text fields too), and the markers their words
    /// are rewritten in place by (M5.6: a form's field keeps its caret while the words around
    /// it change).
    Editor,
}

impl Mark {
    /// Whether the words drawn under this mark carry the markers to be rewritten in place.
    const fn rewrites(self) -> bool {
        matches!(self, Mark::Screen | Mark::Editor)
    }
}

/// How a screen's nodes are spawned: for which screen (none for rows that are no screen's, the
/// party column's), at what window height, marked for which router, and the field that keeps
/// the keyboard.
#[derive(Debug, Clone, Copy)]
pub(super) struct Spawn {
    pub(super) id: Option<ScreenId>,
    pub(super) height: f32,
    pub(super) mark: Mark,
    pub(super) keep: Option<ControlId>,
}

impl Spawn {
    /// The screen marker of a word rewritten in place, when this mark rewrites.
    fn rewriting(self) -> Option<OfScreen> {
        self.id.filter(|_| self.mark.rewrites()).map(OfScreen)
    }
}

/// The words of control `id` (a choice's option, a number's value, a button's label), marked to
/// be rewritten in place when the mark rewrites.
pub(super) fn labelled<B: Bundle>(
    b: &mut ChildSpawnerCommands,
    bundle: B,
    id: ControlId,
    spawn: Spawn,
) {
    let mut current = b.spawn(bundle);
    if let Some(of) = spawn.rewriting() {
        current.insert((LabelOf(id), of));
    }
}

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

/// Which screen a marked word belongs to: `sync` rewrites one screen's words and leaves the
/// others' (the editor's columns stand beside each other, each a screen of its own).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OfScreen(pub(super) ScreenId);

/// A screen's words on screen: the heading, a span, a control's label or the outcome, by marker.
pub(super) type Placed = (
    &'static mut Text,
    &'static mut Ink,
    Option<&'static TitleText>,
    Option<&'static SpanAt>,
    Option<&'static LabelOf>,
    Option<&'static MessageText>,
    Option<&'static OfScreen>,
);

/// A screen's panel: its rows scroll when the screen is taller than the window.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scrolls;

/// How far one line of a mouse wheel scrolls a panel, in logical pixels.
pub(super) const WHEEL_LINE: f32 = 40.0;

/// `role`'s colour in the default theme. A node names its role (`theme::ink`, `edge`) and is
/// painted from the theme of the moment; this is for what has no theme yet.
#[must_use]
pub fn colour(role: Role) -> Color {
    let [r, g, b] = role.rgb();
    Color::srgb(r, g, b)
}

/// A control's background in the default theme: at rest, hovered or pressed.
#[must_use]
pub fn shade(i: usize) -> Color {
    let [r, g, b] = SHADES[i.min(2)];
    Color::srgb(r, g, b)
}

/// The editable text of a field holding `value`, taking at most `max` characters, on `lines`
/// lines: one is a line, more take Enter as a line break and show that many lines.
pub(super) fn text_field(value: &str, max: usize, lines: u8) -> EditableText {
    let mut field = EditableText::new(value);
    field.max_characters = Some(max);
    if lines > 1 {
        field.allow_newlines = true;
        field.visible_lines = Some(f32::from(lines));
    }
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
        selection_radius: 0.0,
    }
}

/// A control's words as drawn: its number key first when it has one, its reason after when it
/// is dim; a choice shows its current option, a number its value.
pub(super) fn shown_label(keys: &[(Key, ControlId)], control: &Control) -> String {
    match &control.kind {
        Kind::Choice { options, at } => return options.get(*at).cloned().unwrap_or_default(),
        Kind::Number { value, .. } => return value.to_string(),
        Kind::Icon { name, .. } => return (*name).to_owned(),
        Kind::Swatch { glyph, .. } => return glyph.to_string(),
        Kind::Flag(on) => return if *on { "x" } else { "" }.to_owned(),
        Kind::Button | Kind::Text { .. } | Kind::Document { .. } => {}
    }
    match &control.why_not {
        Some(why) => format!("{}: {why}", control.label),
        None => match keys.iter().find(|(_, c)| *c == control.id) {
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
) -> (Text, TextFont, (TextColor, Ink)) {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size.px(height)),
            ..default()
        },
        ink(role),
    )
}

/// The least width of a control's box: 150 for every screen, and 120 under the editor's mark,
/// the one exception: its tools column is 330 px and holds its buttons two to a row (the
/// owner's column, 2026-10-25).
fn least_width(mark: Mark) -> f32 {
    match mark {
        Mark::Editor => 120.0,
        Mark::Screen | Mark::Side | Mark::Fight => 150.0,
    }
}

/// A control's box node.
fn boxed_node(height: f32, mark: Mark) -> Node {
    Node {
        min_width: Val::Px(scaled(least_width(mark), height)),
        // At least this tall; a long reason wraps inside the box rather than out of it.
        min_height: Val::Px(scaled(34.0, height)),
        padding: UiRect::horizontal(Val::Px(scaled(10.0, height))),
        border: UiRect::all(Val::Px(1.0)),
        column_gap: Val::Px(scaled(10.0, height)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// A bordered box, as every control is drawn; in a field's row (`bare`) it fills what the
/// labels' column leaves of the row, its parts spread across it.
fn boxed(role: Role, height: f32, mark: Mark, bare: bool) -> impl Bundle {
    let node = if bare {
        Node {
            min_width: Val::Px(0.0),
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            padding: UiRect::horizontal(Val::Px(scaled(8.0, height))),
            column_gap: Val::Px(scaled(8.0, height)),
            justify_content: JustifyContent::SpaceBetween,
            ..boxed_node(height, mark)
        }
    } else {
        boxed_node(height, mark)
    };
    (node, edge(role), fill(0))
}

/// The width of the labels' column of a field's row, at the reference height: the longest of
/// the editor's labels ("chance per mille") at the body size.
const LABELS: f32 = 158.0;

/// The marker of `mark`'s router on a pressable node.
pub(super) fn pressed_by(entity: &mut EntityCommands, mark: Mark, id: ControlId) {
    match mark {
        // A target's row is pointed at even when dim: the view marks what it names.
        Mark::Screen if matches!(id, ControlId::Target(_)) => {
            entity.insert((Pressable(id), bevy::picking::hover::Hovered::default()))
        }
        Mark::Screen => entity.insert(Pressable(id)),
        Mark::Side => entity.insert(Side(id)),
        Mark::Fight => entity.insert(FightPress(id)),
        Mark::Editor => entity.insert(EditorPress(id)),
    };
}

/// The square box of an icon or a swatch: framed twice as thick when chosen, pressable unless
/// dim (no `Button` then).
fn square<'a>(
    parent: &'a mut ChildSpawnerCommands,
    c: &Control,
    chosen: bool,
    role: Role,
    spawn: Spawn,
) -> EntityCommands<'a> {
    let side = scaled(44.0, spawn.height);
    let mut entity = parent.spawn((
        Node {
            width: Val::Px(side),
            height: Val::Px(side),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(if chosen { 2.0 } else { 1.0 })),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        edge(role),
        fill(0),
    ));
    pressed_by(&mut entity, spawn.mark, c.id);
    if c.why_not.is_none() {
        entity.insert(pressable());
    }
    entity
}

/// A swatch control's nodes: the square box holding a filled square of the colour and the glyph
/// over it, black on a light colour and white on a dark one (the text readers print the glyph
/// under its `[button]`).
fn swatch_node(
    parent: &mut ChildSpawnerCommands,
    c: &Control,
    (r, g, b): (u8, u8, u8),
    glyph: char,
    chosen: bool,
    role: Role,
    spawn: Spawn,
) {
    let height = spawn.height;
    let inner = scaled(28.0, height);
    let light = u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 > 128 * 1000;
    let ink = if light { Color::BLACK } else { Color::WHITE };
    square(parent, c, chosen, role, spawn).with_children(|box_| {
        box_.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(inner),
                height: Val::Px(inner),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(r, g, b)),
        ));
        let words = words(&glyph.to_string(), Role::Interface, Size::Small, height);
        // The glyph is drawn in the swatch's own contrast colour, not a role's.
        box_.spawn((words.0, words.1, TextColor(ink)));
    });
}

/// An icon control's nodes: the square box holding the icon's image (asked for through
/// `NeedsIcon`, white when chosen, else the role's colour) named by `IconOf` for the text
/// readers.
fn icon_node(
    parent: &mut ChildSpawnerCommands,
    c: &Control,
    name: &'static str,
    chosen: bool,
    role: Role,
    spawn: Spawn,
) {
    let height = spawn.height;
    let mut entity = square(parent, c, chosen, role, spawn);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = scaled(28.0, height).round().max(1.0) as u32;
    let colour = if chosen {
        icons::CHOSEN
    } else {
        icons::tint(role)
    };
    entity.with_children(|b| {
        #[allow(clippy::cast_precision_loss)]
        b.spawn((
            Node {
                width: Val::Px(px as f32),
                height: Val::Px(px as f32),
                ..default()
            },
            IconOf(name),
            NeedsIcon { name, px, colour },
        ));
    });
}

/// A control's nodes. A button is a bordered box with its words; a dim one has no `Button`, so it
/// takes no clicks, and is drawn in the dim role. A choice is its name, two arrows and the
/// current option between them; a number adds a slider. A text field is its name and an
/// editable line, which takes the keyboard when drawn if `keep` names it, or names nothing and
/// the field is the top screen's: the field that had it when the screen was drawn anew keeps it.
/// In a field's row (`bare`) a control is drawn without its name, which stands in the labels'
/// column, and its box fills the rest of the row.
fn control_node(
    parent: &mut ChildSpawnerCommands,
    keys: &[(Key, ControlId)],
    c: &Control,
    (tint, bare): (Option<Role>, bool),
    spawn: Spawn,
) {
    let Spawn {
        height, mark, keep, ..
    } = spawn;
    let role = if c.why_not.is_some() {
        Role::Dim
    } else {
        tint.unwrap_or(Role::Interface)
    };
    match &c.kind {
        Kind::Icon { name, chosen } => icon_node(parent, c, name, *chosen, role, spawn),
        Kind::Swatch { rgb, glyph, chosen } => {
            swatch_node(parent, c, *rgb, *glyph, *chosen, role, spawn);
        }
        Kind::Flag(_) if bare => tick(parent, keys, c, role, spawn),
        Kind::Flag(_) => {
            parent
                .spawn(boxed(role, height, mark, false))
                .with_children(|b| {
                    b.spawn(words(&c.label, role, Size::Body, height));
                    tick(b, keys, c, role, spawn);
                });
        }
        Kind::Button => {
            let mut entity = parent.spawn(boxed(role, height, mark, false));
            pressed_by(&mut entity, mark, c.id);
            if c.why_not.is_none() {
                entity.insert(pressable());
            }
            entity.with_children(|b| {
                labelled(
                    b,
                    words(&shown_label(keys, c), role, Size::Body, height),
                    c.id,
                    spawn,
                );
            });
        }
        Kind::Choice { .. } => {
            parent
                .spawn(boxed(role, height, mark, bare))
                .with_children(|b| {
                    if !bare {
                        b.spawn(words(&c.label, role, Size::Body, height));
                    }
                    arrow(b, c.id, false, height);
                    labelled(
                        b,
                        words(&shown_label(keys, c), role, Size::Body, height),
                        c.id,
                        spawn,
                    );
                    arrow(b, c.id, true, height);
                });
        }
        Kind::Number { value, min, max } => {
            parent
                .spawn(boxed(role, height, mark, bare))
                .with_children(|b| {
                    if !bare {
                        b.spawn(words(&c.label, role, Size::Body, height));
                    }
                    arrow(b, c.id, false, height);
                    slider(b, c.id, *value, (*min, *max), (height, bare));
                    labelled(
                        b,
                        (
                            words(&shown_label(keys, c), role, Size::Body, height),
                            Node {
                                min_width: Val::Px(scaled(22.0, height)),
                                ..default()
                            },
                        ),
                        c.id,
                        spawn,
                    );
                    arrow(b, c.id, true, height);
                });
        }
        Kind::Text { value, max, lines } => {
            let mut boxed = parent.spawn(boxed(role, height, mark, bare));
            if *lines > 1 {
                boxed.insert(Node {
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    align_items: AlignItems::FlexStart,
                    ..boxed_node(height, mark)
                });
            }
            boxed.with_children(|b| {
                if !c.label.is_empty() && !bare {
                    b.spawn(words(&c.label, role, Size::Body, height));
                }
                let mut field = b.spawn((
                    TextInput,
                    super::focus::field(),
                    text_field(value, *max, *lines),
                    TextFont {
                        font_size: FontSize::Px(Size::Body.px(height)),
                        ..default()
                    },
                    ink(Role::Interface),
                    // Without a style no cursor is drawn; Bevy's default is too dark to see here.
                    cursor_style(),
                    // Underlined, so an empty field still shows where to type; a field of
                    // several lines takes its row's width (a note across the inspector, a
                    // file's text across the centre, M5.9).
                    Node {
                        min_width: Val::Px(if bare { 0.0 } else { scaled(260.0, height) }),
                        width: if *lines > 1 {
                            Val::Percent(100.0)
                        } else {
                            Val::Auto
                        },
                        flex_grow: if *lines > 1 || bare { 1.0 } else { 0.0 },
                        flex_basis: if bare { Val::Px(0.0) } else { Val::Auto },
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..default()
                    },
                    edge(Role::Dim),
                ));
                // Typed text goes to the router the control does.
                match mark {
                    Mark::Editor => field.insert(EditorPress(c.id)),
                    Mark::Screen | Mark::Side | Mark::Fight => field.insert(Pressable(c.id)),
                };
                // The top screen's own field takes the keyboard when none is named; the
                // editor's columns give it only to the field named (a rebuild for any other
                // reason leaves it where it was, which may be nowhere).
                let takes = match keep {
                    Some(kept) => kept == c.id,
                    None => mark != Mark::Editor,
                };
                if takes {
                    field.insert(AutoFocus);
                }
            });
        }
        Kind::Document { value, max } => {
            let takes = match keep {
                Some(kept) => kept == c.id,
                None => mark != Mark::Editor,
            };
            super::document::spawn(parent, value, *max, height, |field| {
                match mark {
                    Mark::Editor => field.insert(EditorPress(c.id)),
                    Mark::Screen | Mark::Side | Mark::Fight => field.insert(Pressable(c.id)),
                };
                if takes {
                    field.insert(AutoFocus);
                }
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

/// A panel's rows under `column`, one line each: its text, its controls, the text after them.
/// Under a mark that rewrites, the spans carry `SpanAt` so their words can be rewritten in place.
pub(super) fn rows(
    column: &mut ChildSpawnerCommands,
    p: usize,
    panel: &Panel,
    keys: &[(Key, ControlId)],
    spawn: Spawn,
) {
    let gap = Val::Px(scaled(6.0, spawn.height));
    for (r, row) in panel.rows.iter().enumerate() {
        // A document's row takes the panel's remaining height (the document scrolls inside it)
        // and so is the one row allowed to be shorter than its content; every other row keeps
        // its content's height and a crowded panel scrolls (a zero minimum on all rows let them
        // shrink into one another; the owner, 2026-10-25).
        let fills = row
            .controls
            .iter()
            .any(|c| matches!(c.kind, Kind::Document { .. }));
        let mut line = column.spawn(Node {
            flex_wrap: if fills || row.aligned {
                FlexWrap::NoWrap
            } else {
                FlexWrap::Wrap
            },
            align_items: if fills {
                AlignItems::Stretch
            } else {
                AlignItems::Center
            },
            flex_grow: if fills { 1.0 } else { 0.0 },
            min_height: if fills { Val::Px(0.0) } else { Val::Auto },
            column_gap: gap,
            row_gap: gap,
            ..default()
        });
        // A field's row explains its field wherever the pointer is on it, its name included.
        if let Some(text) = row.controls.first().and_then(|c| c.tip.clone())
            && row.aligned
        {
            line.insert(Tip { text, beside: true });
        }
        line.with_children(|line| {
            let before = row.spans.len();
            if row.aligned {
                // The labels' column: the same width on every such row.
                line.spawn(Node {
                    width: Val::Px(scaled(LABELS, spawn.height)),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|cell| {
                    for (s, span) in row.spans.iter().enumerate() {
                        span_node(cell, (p, r, s), span, spawn);
                    }
                });
            } else {
                for (s, span) in row.spans.iter().enumerate() {
                    span_node(line, (p, r, s), span, spawn);
                }
            }
            for c in &row.controls {
                match &c.tip {
                    // Any other control with a tip stands in a node that carries it.
                    Some(text) if !row.aligned => {
                        line.spawn((
                            Node::default(),
                            Tip {
                                text: text.clone(),
                                beside: false,
                            },
                        ))
                        .with_children(|held| {
                            control_node(held, keys, c, (row.tint, false), spawn);
                        });
                    }
                    _ => control_node(line, keys, c, (row.tint, row.aligned), spawn),
                }
            }
            for (s, span) in row.after.iter().enumerate() {
                span_node(line, (p, r, before + s), span, spawn);
            }
        });
    }
}

/// A span's words; under a mark that rewrites they carry where they are, to be rewritten in
/// place.
fn span_node(
    line: &mut ChildSpawnerCommands,
    (p, r, s): (usize, usize, usize),
    span: &crate::screen::Span,
    spawn: Spawn,
) {
    let text = words(&span.text, span.role, span.size, spawn.height);
    let mut node = line.spawn(text);
    if let Some(of) = spawn.rewriting() {
        node.insert((SpanAt(p, r, s), of));
    }
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

/// Rewrite the words of `screen`, whose shape is unchanged, in place; the words of other
/// screens standing beside it are left alone.
pub(super) fn sync(screen: &Screen, placed: &mut Query<Placed>) {
    for (mut text, mut tint, title, span, label, message, of) in placed.iter_mut() {
        if of.is_some_and(|of| of.0 != screen.id) {
            continue;
        }
        if let Some((words, role)) = wanted(screen, title, span, label, message) {
            if text.0 != words {
                text.0 = words;
            }
            if tint.0 != role {
                tint.0 = role;
            }
        }
    }
}

/// Where a wheel's `dy` takes a panel scrolled to `at` whose rows run `max` past its foot: from
/// the foot when `at` is past it (a panel set to its foot, `SCROLL_FOOT`), never above the top or
/// below the foot.
#[must_use]
pub fn wheeled(at: f32, dy: f32, max: f32) -> f32 {
    (at.min(max) - dy).clamp(0.0, max)
}

/// A scroll position past any foot: the layout shows the panel's foot (the full log's lines).
pub const SCROLL_FOOT: f32 = f32::MAX;

/// The mouse wheel scrolls the panel under the cursor, inside its rows once it is laid out; its
/// sideways motion, or Shift with it, scrolls a document across (M5.9c).
pub(super) fn wheel(
    scroll: Res<AccumulatedMouseScroll>,
    keys: Res<ButtonInput<KeyCode>>,
    mut panels: Query<(&RelativeCursorPosition, &ComputedNode, &mut ScrollPosition), With<Scrolls>>,
) {
    let unit = match scroll.unit {
        MouseScrollUnit::Line => WHEEL_LINE,
        MouseScrollUnit::Pixel => 1.0,
    };
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let (dx, dy) = if shift {
        (scroll.delta.y * unit, 0.0)
    } else {
        (scroll.delta.x * unit, scroll.delta.y * unit)
    };
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    for (cursor, node, mut at) in &mut panels {
        if cursor.cursor_over() {
            // How far the content runs past the area, in logical pixels; unbounded until laid out.
            let past = |content: f32, size: f32| {
                if size > 0.0 {
                    ((content - size) * node.inverse_scale_factor).max(0.0)
                } else {
                    f32::INFINITY
                }
            };
            // Wheel up (a positive delta) shows what is above; wheel left what is to the left.
            if dy != 0.0 {
                at.0.y = wheeled(at.0.y, dy, past(node.content_size.y, node.size.y));
            }
            if dx != 0.0 {
                at.0.x = wheeled(at.0.x, dx, past(node.content_size.x, node.size.x));
            }
        }
    }
}

/// The scrollbar's width, at the reference height.
const BAR: f32 = 6.0;

/// A panel's scrollbar: a dim track beside the rows and a thumb whose length is the share of
/// them in view. Drawn only while the rows run past the panel's foot ([`bars`]); its gutter is
/// kept either way, so showing it never moves the rows.
fn scrollbar(pane: &mut ChildSpawnerCommands, rows_at: Entity, height: f32) {
    pane.spawn((
        Scrollbar::new(rows_at, ControlOrientation::Vertical, scaled(24.0, height)),
        Node {
            width: Val::Px(scaled(BAR, height).max(3.0)),
            flex_shrink: 0.0,
            ..default()
        },
        fill(1),
        Visibility::Hidden,
    ))
    .with_children(|bar| {
        bar.spawn((ScrollbarThumb::default(), solid(Role::Interface)));
    });
}

/// Show each panel's scrollbar while its rows run past its foot, hide it when they fit.
pub(super) fn bars(
    areas: Query<&ComputedNode, With<Scrolls>>,
    mut scrollbars: Query<(&Scrollbar, &mut Visibility)>,
) {
    for (bar, mut shown) in &mut scrollbars {
        let Ok(area) = areas.get(bar.target) else {
            continue;
        };
        // Half a pixel of slack, so rounding never shows a bar with nothing to scroll.
        let over = match bar.orientation {
            ControlOrientation::Vertical => area.content_size().y > area.size().y + 0.5,
            ControlOrientation::Horizontal => area.content_size().x > area.size().x + 0.5,
        };
        shown.set_if_neq(if over {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
}

/// A panel's share of the screen's width and where it starts from: equal shares, but the full
/// log's filter keeps its own width and its lines take the rest.
fn grow(screen: &Screen, panel: usize) -> (f32, Val) {
    if screen.id == crate::screen::ScreenId::Log && panel == 0 {
        (0.0, Val::Auto)
    } else {
        (1.0, Val::Px(0.0))
    }
}

/// Build the screen's nodes under the root: the heading, the panels side by side, the outcome.
/// Under a mark that rewrites, the words carry the markers they are rewritten in place by
/// (`sync`). `keep` is the field that had the keyboard, which keeps it.
pub(super) fn build(
    parent: &mut ChildSpawnerCommands,
    screen: &Screen,
    (height, mark): (f32, Mark),
    keep: Option<ControlId>,
) {
    // With no field named, a screen's first field takes the keyboard: one field, never each of
    // them (the owner's report, 2026-11-15: the resize's four fields all asked for it and the
    // last one drawn won).
    let first = || {
        screen
            .controls()
            .find(|c| matches!(c.kind, Kind::Text { .. } | Kind::Document { .. }))
            .map(|c| c.id)
    };
    let keep = keep.or_else(|| (mark != Mark::Editor).then(first).flatten());
    let spawn = Spawn {
        id: Some(screen.id),
        height,
        mark,
        keep,
    };
    let t = &screen.title;
    let mut title = parent.spawn(words(&t.text, t.role, t.size, height));
    if let Some(of) = spawn.rewriting() {
        title.insert((TitleText, of));
    }
    parent
        .spawn(Node {
            column_gap: Val::Px(scaled(24.0, height)),
            // Gives way to the root's height, so the panels scroll rather than overflow.
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|columns| {
            for (p, panel) in screen.panels.iter().enumerate() {
                columns
                    // Panels share the width equally, so one panel's long rows never squeeze
                    // the next into wrapping.
                    .spawn(Node {
                        flex_grow: grow(screen, p).0,
                        flex_basis: grow(screen, p).1,
                        min_width: Val::Px(0.0),
                        min_height: Val::Px(0.0),
                        column_gap: Val::Px(scaled(6.0, height)),
                        ..default()
                    })
                    .with_children(|pane| {
                        let rows_at = pane
                            .spawn((
                                Scrolls,
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_grow: 1.0,
                                    min_width: Val::Px(0.0),
                                    min_height: Val::Px(0.0),
                                    row_gap: Val::Px(scaled(8.0, height)),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                                ScrollPosition::default(),
                                RelativeCursorPosition::default(),
                            ))
                            .with_children(|column| {
                                rows(column, p, panel, &screen.keys, spawn);
                            })
                            .id();
                        scrollbar(pane, rows_at, height);
                    });
            }
        });
    if let Some(m) = &screen.message {
        let mut message = parent.spawn(words(&m.text, m.role, m.size, height));
        if let Some(of) = spawn.rewriting() {
            message.insert((MessageText, of));
        }
    }
}

/// Escape leaves a text field the keyboard. Bevy's text input gives the keyboard up on Escape
/// (0.20) and passes the key on; here Escape is the menu's or the screen's, and a field that
/// still stands under it goes on taking what is typed, as it did before: without this the
/// letters typed after a menu closed over the editor's form chose tools. Runs once the key has
/// been dispatched, before any router reads who holds the keyboard.
pub fn keep_keyboard(
    focus: Option<ResMut<InputFocus>>,
    keys: Res<ButtonInput<KeyCode>>,
    fields: Query<(), With<EditableText>>,
    mut held: Local<Option<Entity>>,
) {
    let Some(mut focus) = focus else {
        return;
    };
    match focus.get() {
        Some(entity) => *held = fields.contains(entity).then_some(entity),
        None => {
            // Only the Escape that took it gives it back: a click elsewhere gave it up.
            if let Some(field) = held.take()
                && keys.just_pressed(KeyCode::Escape)
                && fields.contains(field)
            {
                focus.set(field, FocusCause::Auto);
                *held = Some(field);
            }
        }
    }
}
