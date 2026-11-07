//! Helpers for the editor's tests (M5.4, M5.5): the shell started in the editor, the layer's
//! controls pressed and its choices stepped, the pointer put over the plan and the mouse's
//! buttons as the window would send them.

use super::app::{config, set, shell};
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use omnis_sim::omnis_core::Facing;
use omnis_vector::editor::EditAct;
use omnis_vector::editor::forms::Placing;
use omnis_vector::editor::stamp;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::editor::{Column, Editing};
use omnis_vector::shell::screen::{Arrow, EditorPress, Pressable, Screens, Scrolls};
use omnis_vector::shell::session::{Config, Packs};
use std::path::PathBuf;

/// The shell started in the editor with `map` open, on the shipped packs.
pub fn in_editor(map: &str) -> App {
    in_editor_on(&[], map)
}

/// The shell started in the editor with `map` open, on the shipped packs and the fixture
/// packs at `extra` (paths from the repository's root).
pub fn in_editor_on(extra: &[&str], map: &str) -> App {
    let mut packs = config().packs;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    packs.extend(extra.iter().map(|p| root.join(p)));
    let mut app = shell(
        Config {
            packs,
            editor: Some(Some(map.to_owned())),
            ..config()
        },
        false,
    );
    app.update();
    app
}

/// The scrolling panel of `column`, and how far it is scrolled.
pub fn panel(app: &mut App, column: Column) -> (Entity, f32) {
    let world = app.world_mut();
    let mut panels = world.query_filtered::<(Entity, &ScrollPosition), With<Scrolls>>();
    let found: Vec<(Entity, f32)> = panels.iter(world).map(|(e, at)| (e, at.0.y)).collect();
    for (panel, y) in found {
        let mut up = panel;
        while let Some(parent) = world.get::<ChildOf>(up) {
            up = parent.0;
            if world.get::<Column>(up) == Some(&column) {
                return (panel, y);
            }
        }
    }
    panic!("the {column:?} column has a scrolling panel")
}

pub fn editing(app: &App) -> &Editing {
    app.world().resource::<Editing>()
}

/// Press a control of the editor's layer, then let it go.
pub fn press_edit(app: &mut App, act: EditAct) {
    let id = ControlId::Edit(act);
    set(app, &EditorPress(id), Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut buttons = world.query::<(&EditorPress, &mut Interaction)>();
    for (p, mut i) in buttons.iter_mut(world) {
        if p.0 == id {
            *i = Interaction::None;
        }
    }
    app.update();
}

/// Step a choice of the editor's layer right (`true`) or left.
pub fn arrow(app: &mut App, act: EditAct, right: bool) {
    let marker = Arrow(ControlId::Edit(act), right);
    set(app, &marker, Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut arrows = world.query::<(&Arrow, &mut Interaction)>();
    for (a, mut i) in arrows.iter_mut(world) {
        if *a == marker {
            *i = Interaction::None;
        }
    }
    app.update();
}

/// The pointer over the plan at `cell`'s middle, or a pixel in from its `side`, as the layout
/// would report it to the pane (headless, the pane keeps the model's size): Bevy's `normalized`
/// runs from (-0.5, -0.5) at the top left to (0.5, 0.5) at the bottom right. The pointer's
/// path through Bevy's own layout and focus is `editor_pointer`'s.
pub fn point_at(app: &mut App, cell: (u16, u16), side: Option<Facing>) {
    let s = editing(app).view.cell_rect(cell).2 as f32;
    let (dx, dy) = match side {
        None => (0.5, 0.5),
        Some(Facing::North) => (0.5, 0.0),
        Some(Facing::East) => ((s - 1.0) / s, 0.5),
        Some(Facing::South) => (0.5, (s - 1.0) / s),
        Some(Facing::West) => (0.0, 0.5),
    };
    point_frac(app, cell, dx, dy);
}

/// The pointer over the plan at the point `fx` across and `fy` down `cell` (0 to 1 of its side).
pub fn point_frac(app: &mut App, cell: (u16, u16), fx: f32, fy: f32) {
    let (pane, view) = {
        let e = editing(app);
        (e.pane, e.view)
    };
    let (left, top, s) = view.cell_rect(cell);
    #[allow(clippy::cast_precision_loss)]
    let at = Vec2::new(
        (left as f32 + s as f32 * fx) / pane.0 as f32,
        (top as f32 + s as f32 * fy) / pane.1 as f32,
    ) - Vec2::splat(0.5);
    over_plan(app, Some(at));
}

/// The pointer off the plan.
pub fn leave_plan(app: &mut App) {
    over_plan(app, None);
}

fn over_plan(app: &mut App, at: Option<Vec2>) {
    let world = app.world_mut();
    let mut panes = world.query::<(&Column, &mut RelativeCursorPosition)>();
    let mut found = false;
    for (column, mut position) in panes.iter_mut(world) {
        if *column == Column::Plan {
            position.cursor_over = at.is_some();
            position.normalized = at;
            found = true;
        }
    }
    assert!(found, "the plan's pane");
}

/// Type `text` into the editor's field `act`, as the field's editing would leave it, then a
/// frame for the layer to read it.
pub fn type_into(app: &mut App, act: EditAct, text: &str) {
    let id = ControlId::Edit(act);
    let world = app.world_mut();
    let mut fields = world.query::<(Entity, &EditorPress, &bevy::text::EditableText)>();
    let field = fields
        .iter(world)
        .find(|(_, p, _)| p.0 == id)
        .map(|(e, _, _)| e)
        .unwrap_or_else(|| panic!("the {id:?} field"));
    world
        .entity_mut(field)
        .insert(bevy::text::EditableText::new(text));
    app.update();
}

/// Type `text` into the field `id` of the screen on top (a modal over the editor), as the
/// field's editing would leave it, then a frame for the router to read it.
pub fn type_ui(app: &mut App, id: ControlId, text: &str) {
    let world = app.world_mut();
    let mut fields = world.query::<(Entity, &Pressable, &bevy::text::EditableText)>();
    let field = fields
        .iter(world)
        .find(|(_, p, _)| p.0 == id)
        .map(|(e, _, _)| e)
        .unwrap_or_else(|| panic!("the {id:?} field on the top screen"));
    world
        .entity_mut(field)
        .insert(bevy::text::EditableText::new(text));
    app.update();
}

/// The screen on top of the editor, if any (the editor runs with no game).
pub fn top_screen(app: &App) -> Option<ScreenId> {
    app.world().resource::<Screens>().nav.top(None)
}

/// Hold `ids` in the stamp of `kind`, as the picker's Finish would with the properties left as
/// they come (the editor's column, step 3b).
pub fn hold(app: &mut App, kind: Placing, ids: &[&str]) {
    let data = app.world().resource::<Packs>().data.clone();
    let def = editing(app).def().expect("a map");
    let chosen: Vec<String> = ids.iter().map(|s| (*s).to_owned()).collect();
    let places = stamp::rebuild(kind, &chosen, &[], &def, &data);
    assert!(!places.is_empty(), "{kind:?} holds {ids:?}");
    let mut editing = app.world_mut().resource_mut::<Editing>();
    editing.stamps.set(kind, places);
    editing.revision = editing.revision.wrapping_add(1);
    app.update();
}

/// The details the stamp of `kind` holds.
pub fn held(app: &App, kind: Placing) -> Vec<String> {
    editing(app).stamps.ids(kind)
}

/// The entity of the editor's field `act`, if it is drawn.
pub fn field_entity(app: &mut App, act: EditAct) -> Option<Entity> {
    let id = ControlId::Edit(act);
    let world = app.world_mut();
    let mut fields = world.query::<(Entity, &EditorPress, &bevy::text::EditableText)>();
    fields
        .iter(world)
        .find(|(_, p, _)| p.0 == id)
        .map(|(e, _, _)| e)
}

/// The index of `form`'s own field `label`, as `EditAct::Field` counts it (a form's own fields
/// come first and stay put while rows come and go).
pub fn field_of(form: &omnis_vector::editor::form::Form, label: &str) -> u8 {
    let i = form
        .fields
        .iter()
        .position(|f| f.slot == omnis_vector::editor::form::Slot::Top && f.label == label)
        .unwrap_or_else(|| panic!("the form has a field {label}"));
    u8::try_from(i).unwrap()
}

/// A scratch copy of the shipped test pack under the target directory, named `name`: what a
/// test that saves or writes notes edits, never `crates/omnis-sim/tests/packs/test` itself.
pub fn scratch_pack(name: &str) -> PathBuf {
    fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(src) else {
            return;
        };
        std::fs::create_dir_all(dst).unwrap();
        for entry in entries.map(Result::unwrap) {
            let to = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &to);
            } else {
                std::fs::copy(entry.path(), to).unwrap();
            }
        }
    }
    let src = config().packs.pop().unwrap();
    let dst = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::copy(src.join("pack.ron"), dst.join("pack.ron")).unwrap();
    for dir in ["data", "text", "notes", "migrations"] {
        copy_dir(&src.join(dir), &dst.join(dir));
    }
    dst
}

/// The shell started in the editor with `map` of a scratch copy of the test pack (`name`) open,
/// and the copy's root.
pub fn in_editor_scratch(name: &str, map: &str) -> (App, PathBuf) {
    let scratch = scratch_pack(name);
    let mut config = config();
    config.packs.pop();
    config.packs.push(scratch.clone());
    config.editor = Some(Some(map.to_owned()));
    let mut app = shell(config, false);
    app.update();
    (app, scratch)
}

/// A mouse button pressed or released, then a frame.
pub fn mouse(app: &mut App, button: MouseButton, state: ButtonState) {
    app.world_mut().write_message(MouseButtonInput {
        button,
        state,
        window: Entity::PLACEHOLDER,
    });
    app.update();
}
