//! Driving the Feathers creation panel headless: controls found by `PanelId`, reports sent as
//! the widgets' own events, and the pointer and keys as a window would send them, through real
//! layout, picking and focus. The window's scale factor is one here, so a node's physical
//! position is also its logical one.

use super::{feathers_app, start_new_game_by_mouse};
use bevy::camera::{NormalizedRenderTarget, RenderTarget};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocus;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui::{CalculatedClip, UiGlobalTransform};
use bevy::ui_widgets::{Activate, ScrollArea, SliderValue, ValueChange};
use bevy::window::{PrimaryWindow, WindowRef, WindowResized};
use omnis_app::creation_panel::{Choice, PanelId};
use omnis_app::feathers_creation::{Control, LabelId, PanelRoot, Shown};
use omnis_app::menus::Screens;

/// A new game by the canvas menus, arriving on party creation in the Feathers skin.
pub fn creating(save: &str) -> App {
    let mut app = feathers_app(save, false);
    start_new_game_by_mouse(&mut app);
    settle(&mut app);
    app
}

pub fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

pub fn control(app: &mut App, id: PanelId) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &Control)>();
    query
        .iter(app.world())
        .find_map(|(entity, control)| (control.0 == id).then_some(entity))
        .unwrap_or_else(|| panic!("{id:?} is not on the panel"))
}

/// Every control on the panel, in `PanelId`'s order.
pub fn controls(app: &mut App) -> Vec<PanelId> {
    let mut query = app.world_mut().query::<&Control>();
    let mut ids: Vec<_> = query.iter(app.world()).map(|control| control.0).collect();
    ids.sort();
    ids
}

pub fn form(app: &App) -> &omnis_app::menu::CreationForm {
    &app.world().resource::<Screens>().creation
}

/// A text the panel writes from the form.
pub fn shown(app: &mut App, id: LabelId) -> String {
    let mut query = app.world_mut().query::<(&Shown, &Text)>();
    query
        .iter(app.world())
        .find_map(|(shown, text)| (shown.0 == id).then(|| text.0.clone()))
        .unwrap_or_else(|| panic!("{id:?} is not on the panel"))
}

// ------------------------------------------------------------------ reports as events

pub fn activate(app: &mut App, id: PanelId) {
    let entity = control(app, id);
    app.world_mut().trigger(Activate { entity });
    settle(app);
}

pub fn change<T: Send + Sync + 'static + Clone>(app: &mut App, id: PanelId, value: T) {
    let source = control(app, id);
    app.world_mut().trigger(ValueChange {
        source,
        value,
        is_final: true,
    });
    settle(app);
}

pub fn type_name(app: &mut App, name: &str) {
    let entity = control(app, PanelId::Name);
    app.world_mut()
        .get_mut::<EditableText>(entity)
        .expect("the name input")
        .queue_edit(TextEdit::Insert(name.into()));
    settle(app);
}

/// The human fighter of `draft_fighter_by_mouse`, short of `Add member`: STR 15, DEX 14,
/// CON 13, INT 12, WIS 10, CHA 8 (number inputs and sliders by turns), Athletics, Perception.
pub fn draft_fighter(app: &mut App) {
    type_name(app, "Brenna");
    activate(app, PanelId::Pick(Choice::Race, 3));
    activate(app, PanelId::Pick(Choice::Class, 1));
    for (ability, score) in [15_u8, 14, 13, 12, 10, 8].into_iter().enumerate() {
        if ability % 2 == 0 {
            change(app, PanelId::Score(ability), i32::from(score));
        } else {
            change(app, PanelId::ScoreSlider(ability), f32::from(score));
        }
    }
    change(app, PanelId::Skill(2), true);
    change(app, PanelId::Skill(6), true);
}

// ------------------------------------------------------------------ pointer and keys

pub fn window(app: &mut App) -> Entity {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>();
    query.single(app.world()).expect("a primary window")
}

fn target(app: &mut App) -> NormalizedRenderTarget {
    let window = window(app);
    RenderTarget::Window(WindowRef::Entity(window))
        .normalize(Some(window))
        .expect("a window target")
}

/// A node's rectangle in window pixels.
pub fn rect(app: &App, entity: Entity) -> Rect {
    let world = app.world();
    let at = world
        .get::<UiGlobalTransform>(entity)
        .expect("a laid-out node");
    let node = world.get::<ComputedNode>(entity).expect("a node");
    Rect::from_center_size(at.translation, node.size())
}

/// One pointer message, and the frame that hears it.
pub fn pointer(app: &mut App, position: Vec2, action: PointerAction) {
    let location = Location {
        target: target(app),
        position,
    };
    app.world_mut()
        .resource_mut::<Messages<PointerInput>>()
        .write(PointerInput::new(PointerId::Mouse, location, action));
    app.update();
}

/// A real click at a point: the pointer moves there, presses, and lets go, a frame each.
pub fn click_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

/// A real click on a node's centre.
pub fn click_node(app: &mut App, entity: Entity) {
    let at = rect(app, entity).center();
    click_at(app, at);
}

/// A real drag: press on the node's centre, move away in four steps, let go.
pub fn drag_node(app: &mut App, entity: Entity, by: Vec2) {
    let from = rect(app, entity).center();
    pointer(app, from, PointerAction::Move { delta: Vec2::ZERO });
    pointer(app, from, PointerAction::Press(PointerButton::Primary));
    for step in 1..=4_u8 {
        let at = from + by * f32::from(step) / 4.0;
        pointer(app, at, PointerAction::Move { delta: by / 4.0 });
    }
    pointer(
        app,
        from + by,
        PointerAction::Release(PointerButton::Primary),
    );
    settle(app);
}

fn key(app: &mut App, key_code: KeyCode, logical_key: Key, text: Option<&str>) {
    let window = window(app);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window,
        });
    app.update();
}

/// Keys as a keyboard sends them: a logical key with its text.
pub fn keys(app: &mut App, text: &str) {
    for c in text.chars() {
        let s = c.to_string();
        key(
            app,
            KeyCode::F24,
            Key::Character(s.as_str().into()),
            Some(&s),
        );
    }
    settle(app);
}

pub fn tab(app: &mut App) {
    key(app, KeyCode::Tab, Key::Tab, None);
    settle(app);
}

pub fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

pub fn name_text(app: &mut App) -> String {
    let name = control(app, PanelId::Name);
    let text = app.world().get::<EditableText>(name).expect("an input");
    text.value().to_string()
}

/// The text input inside an ability's number input.
pub fn number_input(app: &mut App, ability: usize) -> Entity {
    let outer = control(app, PanelId::Score(ability));
    let children = app.world().get::<Children>(outer).expect("children");
    children
        .iter()
        .find(|child| app.world().get::<EditableText>(*child).is_some())
        .expect("a text input in the number input")
}

pub fn slider(app: &mut App, id: PanelId) -> f32 {
    let slider = control(app, id);
    app.world().get::<SliderValue>(slider).expect("a slider").0
}

/// Another window size. The camera's target follows the resize message, and so does the
/// app's `WindowSize`; the resolution alone moves neither.
pub fn resize(app: &mut App, width: f32, height: f32) {
    let window = window(app);
    let mut entity = app.world_mut().entity_mut(window);
    let mut found = entity.get_mut::<Window>().expect("a window");
    found.resolution.set(width, height);
    app.world_mut()
        .resource_mut::<Messages<WindowResized>>()
        .write(WindowResized {
            window,
            width,
            height,
        });
    settle(app);
}

/// The owner's display.
pub fn ultrawide(app: &mut App) {
    resize(app, 5120.0, 1440.0);
}

/// Choose a class as a person does: open the menu, click the item. The panel is rebuilt
/// under the pointer, with the keyboard focus left on an entity that is gone.
pub fn pick_class_by_mouse(app: &mut App, index: usize) {
    let before = control(app, PanelId::Name);
    let menu = control(app, PanelId::Menu(Choice::Class));
    click_node(app, menu);
    let item = control(app, PanelId::Pick(Choice::Class, index));
    click_node(app, item);
    assert_eq!(form(app).class, index);
    assert_ne!(control(app, PanelId::Name), before, "the panel was rebuilt");
}

// ------------------------------------------------------------------ the layout check

/// What is wrong with where a control lies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fault {
    /// It has no size.
    Empty(PanelId),
    /// It is not wholly inside the panel.
    Outside(PanelId),
    /// Two controls share pixels.
    Overlap(PanelId, PanelId),
    /// The skills' scroll pane is not wholly inside the panel.
    PaneOutside,
}

fn inside(outer: Rect, inner: Rect) -> bool {
    outer.contains(inner.min) && outer.contains(inner.max)
}

/// The canvas screen's `assert_laid_out` for the panel: every control has a size and lies
/// inside the panel, and no two share more than half a pixel. A skill counts as far as its
/// scroll pane shows it (one scrolled out of sight counts for nothing) and the pane itself
/// must be inside; menu items are left out, their popups are laid out over the panel.
pub fn layout_faults(app: &mut App) -> Vec<Fault> {
    let mut roots = app.world_mut().query_filtered::<Entity, With<PanelRoot>>();
    let root = roots.single(app.world()).expect("one panel");
    let root = rect(app, root);
    let mut faults = Vec::new();
    let mut panes = app.world_mut().query_filtered::<Entity, With<ScrollArea>>();
    let panes: Vec<_> = panes.iter(app.world()).collect();
    if panes.iter().any(|pane| !inside(root, rect(app, *pane))) {
        faults.push(Fault::PaneOutside);
    }
    let mut query = app
        .world_mut()
        .query::<(Entity, &Control, Option<&CalculatedClip>)>();
    let mut seen: Vec<(PanelId, Rect)> = Vec::new();
    for (entity, control, clip) in query.iter(app.world()) {
        let id = control.0;
        let full = rect(app, entity);
        match id {
            PanelId::Pick(..) | PanelId::FontPick(_) => {}
            _ if full.is_empty() => faults.push(Fault::Empty(id)),
            PanelId::Skill(_) => {
                let visible = clip.map_or(full, |c| full.intersect(c.clip));
                if !visible.is_empty() {
                    seen.push((id, visible));
                }
            }
            _ if !inside(root, full) => faults.push(Fault::Outside(id)),
            _ => seen.push((id, full)),
        }
    }
    seen.sort_by_key(|(id, _)| *id);
    for (index, (a, first)) in seen.iter().enumerate() {
        for (b, second) in &seen[index + 1..] {
            let shared = first.intersect(*second).size();
            if shared.x > 0.5 && shared.y > 0.5 {
                faults.push(Fault::Overlap(*a, *b));
            }
        }
    }
    faults
}

// ------------------------------------------------------------------ the text tree

fn tree_line(world: &World, entity: Entity, depth: usize) -> Option<String> {
    let control = world.get::<Control>(entity).map(|c| format!("{:?}", c.0));
    let label = world.get::<Shown>(entity).map(|s| format!("{:?}", s.0));
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

/// The panel as text, the stand-in for a screen dump (a PPM of the canvas cannot show
/// `bevy_ui`): one line per control, rewritten label, text or input, in tree order, indented
/// by its depth among such lines, with its rectangle in window pixels; what a closed menu
/// holds is marked hidden.
pub fn text_tree(app: &mut App) -> String {
    let mut roots = app.world_mut().query_filtered::<Entity, With<PanelRoot>>();
    let root = roots.single(app.world()).expect("one panel");
    let world = app.world();
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
