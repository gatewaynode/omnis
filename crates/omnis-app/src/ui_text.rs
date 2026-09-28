//! The open `bevy_ui` panels as text, the stand-in for a screen dump (a PPM of the canvas
//! cannot show `bevy_ui`, and an agent-launched window captures black): one line per control,
//! rewritten label, text or input, in tree order, indented by its depth among such lines, with
//! its rectangle in window pixels; what a closed menu holds is marked hidden. The tests' text
//! tree and the dev socket's `screen.text` both read it.

use crate::ui_kit::{Control, PanelRoot, Shown};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::UiGlobalTransform;

/// What `screen.text` answers when no panel is open.
pub const NO_PANEL: &str = "no panel is open (the canvas screens are not panels; use screenshot)";

/// Every open panel, each under a line naming its screen, in screen order; [`NO_PANEL`] when
/// none is open.
#[must_use]
pub fn screen_text(world: &World) -> String {
    let Some(mut roots) = world.try_query::<(Entity, &PanelRoot)>() else {
        return NO_PANEL.to_owned();
    };
    let mut panels: Vec<_> = roots.iter(world).map(|(e, r)| (r.screen, e)).collect();
    panels.sort();
    if panels.is_empty() {
        return NO_PANEL.to_owned();
    }
    panels
        .into_iter()
        .map(|(screen, root)| format!("{screen:?} panel\n{}", panel_text(world, root)))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// One panel below `root`, without a heading.
#[must_use]
pub fn panel_text(world: &World, root: Entity) -> String {
    let mut lines = Vec::new();
    let mut stack = vec![(root, 0_usize)];
    while let Some((entity, depth)) = stack.pop() {
        let line = tree_line(world, entity, depth);
        let below = depth + usize::from(line.is_some());
        lines.extend(line);
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev().map(|child| (child, below)));
        }
    }
    lines.join("\n")
}

fn tree_line(world: &World, entity: Entity, depth: usize) -> Option<String> {
    let control = world.get::<Control>(entity).map(|c| c.0.name());
    let label = world.get::<Shown>(entity).map(|s| s.0.name());
    let text = world.get::<Text>(entity).map(|t| t.0.clone());
    let typed = world
        .get::<EditableText>(entity)
        .map(|t| t.value().to_string());
    if control.is_none() && label.is_none() && text.is_none() && typed.is_none() {
        return None;
    }
    let node = world.get::<ComputedNode>(entity)?;
    let at = world.get::<UiGlobalTransform>(entity)?;
    let rect = Rect::from_center_size(at.translation, node.size());
    let mut line = format!(
        "{}{:.0},{:.0} {:.0}x{:.0}",
        "  ".repeat(depth),
        rect.min.x,
        rect.min.y,
        rect.width(),
        rect.height()
    );
    for tag in [control, label].into_iter().flatten() {
        line.push_str(&format!(" [{tag}]"));
    }
    if let Some(text) = text.or(typed) {
        line.push_str(&format!(" \"{text}\""));
    }
    if world
        .get::<InheritedVisibility>(entity)
        .is_some_and(|v| !v.get())
    {
        line.push_str(" (hidden)");
    }
    Some(line)
}
