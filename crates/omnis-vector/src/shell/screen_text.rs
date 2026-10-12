//! What the interface shows, as text: the dev socket's `screen.text` (ARCHITECTURE.md §9.3) and
//! a check for the headless tests. One line per text on screen, in tree order under each root
//! node, indented by its depth among such lines: its rectangle in window pixels once layout has
//! run, `[button]` when a click on it does something, `[field]` for a text field, and the text;
//! a slider is `[slider]` with its value and range. What is not shown (a node
//! with `Display::None`, or hidden) is left out, so the answer is the screen, for any screen,
//! with no code per screen.

use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::UiGlobalTransform;
use bevy::ui_widgets::{SliderRange, SliderValue};

/// What `screen.text` answers when no text is shown.
pub const NOTHING: &str = "no text is shown";

/// Every text on screen, root by root in the order they were made.
#[must_use]
pub fn screen_text(world: &World) -> String {
    let Some(mut roots) = world.try_query_filtered::<Entity, (With<Node>, Without<ChildOf>)>()
    else {
        return NOTHING.to_owned();
    };
    let mut roots: Vec<Entity> = roots.iter(world).collect();
    roots.sort();
    let lines: Vec<String> = roots
        .into_iter()
        .flat_map(|root| lines_below(world, root))
        .collect();
    if lines.is_empty() {
        NOTHING.to_owned()
    } else {
        lines.join("\n")
    }
}

/// The lines for `root` and everything shown below it.
fn lines_below(world: &World, root: Entity) -> Vec<String> {
    let mut lines = Vec::new();
    let mut stack = vec![(root, 0_usize)];
    while let Some((entity, depth)) = stack.pop() {
        if !shown(world, entity) {
            continue;
        }
        let line = line(world, entity, depth);
        let below = depth + usize::from(line.is_some());
        lines.extend(line);
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev().map(|child| (child, below)));
        }
    }
    lines
}

/// Whether the node takes part in the screen: laid out and not hidden.
fn shown(world: &World, entity: Entity) -> bool {
    let laid_out = world
        .get::<Node>(entity)
        .is_none_or(|node| node.display != Display::None);
    let visible = world
        .get::<Visibility>(entity)
        .is_none_or(|v| *v != Visibility::Hidden);
    laid_out && visible
}

fn line(world: &World, entity: Entity, depth: usize) -> Option<String> {
    if let (Some(value), Some(range)) = (
        world.get::<SliderValue>(entity),
        world.get::<SliderRange>(entity),
    ) {
        return Some(format!(
            "{}[slider] {} ({}..{})",
            "  ".repeat(depth),
            value.0,
            range.start(),
            range.end()
        ));
    }
    let (field, text) = match (world.get::<Text>(entity), world.get::<EditableText>(entity)) {
        (Some(text), _) => (false, text.0.clone()),
        (None, Some(field)) => (true, field.value().to_string()),
        (None, None) => return None,
    };
    let mut line = "  ".repeat(depth);
    if let (Some(node), Some(at)) = (
        world.get::<ComputedNode>(entity),
        world.get::<UiGlobalTransform>(entity),
    ) {
        let rect = Rect::from_center_size(at.translation, node.size());
        line.push_str(&format!(
            "{:.0},{:.0} {:.0}x{:.0} ",
            rect.min.x,
            rect.min.y,
            rect.width(),
            rect.height()
        ));
    }
    let pressable = world
        .get::<ChildOf>(entity)
        .is_some_and(|parent| world.get::<Button>(parent.parent()).is_some());
    if pressable {
        line.push_str("[button] ");
    }
    if field {
        line.push_str("[field] ");
    }
    // Debug quoting keeps a text of several lines on one line.
    line.push_str(&format!("{text:?}"));
    Some(line)
}
