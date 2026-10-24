//! The editor's layer (M5.4; `tasks/plans/m5-editor-v1.md` §Client side, presentation-ARCHITECTURE
//! §6): while the `Editing` resource is there, three columns stand in place of the 3D view:
//! the tools (a kit screen), the plan (the Bevy-free raster in one image node, with the cursor
//! as a bordered node over it) and the inspector (a kit screen). Built like the fight screen:
//! the columns are rebuilt only when their screen changed, the plan repainted only when what
//! it shows changed. The pointer paints (M5.5): a left click paints the brush at the cell or
//! the edge under it and a drag is one stroke, a right click picks what is there. The editor
//! runs with no `Session`, so nothing of the game moves or draws; its menu is a screen of the
//! kit (`ScreenId::EditorMenu`). Headless-safe: without `Assets<Image>` the plan is not
//! painted, and the rest stands.

use super::VectorSet;
use super::screen::{Arrow, Screens};
use super::screen_draw::{EditorPress, Mark, build, colour, shade};
use super::session::Packs;
use crate::editor::brush::Target;
use crate::editor::plan::{self, Source};
use crate::editor::{Asked, EditAct, Editor, view};
use crate::screen::{ControlId, Screen, ScreenId};
use crate::style::{Role, scaled};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;
use omnis_sim::omnis_core::Facing;

/// The editor, while it is open. Wraps the Bevy-free model.
#[derive(Resource, Debug)]
pub struct Editing(pub Editor);

impl core::ops::Deref for Editing {
    type Target = Editor;
    fn deref(&self) -> &Editor {
        &self.0
    }
}

impl core::ops::DerefMut for Editing {
    fn deref_mut(&mut self) -> &mut Editor {
        &mut self.0
    }
}

/// Marks the layer's root; everything of the editor is under it.
#[derive(Component)]
pub struct EditorRoot;

/// Which column a node is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    /// The tools, left.
    Tools,
    /// The plan's pane, centre.
    Plan,
    /// The inspector, right.
    Inspector,
}

/// The plan's picture.
#[derive(Component)]
pub struct PlanImage;

/// The cursor's border over the plan.
#[derive(Component)]
pub struct PlanCursor;

/// The plan's picture node: its image and its size.
type Picture = (&'static mut ImageNode, &'static mut Node);
/// Which node the picture is.
type IsPicture = (With<PlanImage>, Without<PlanCursor>);

/// What the layer last drew.
#[derive(Resource, Debug, Default)]
struct Drawn {
    tools: Option<(Screen, u32)>,
    inspector: Option<(Screen, u32)>,
    plan: Option<(u64, (u32, u32))>,
}

/// The tools column's width at the reference height.
const TOOLS: f32 = 330.0;
/// The inspector column's width at the reference height.
const INSPECTOR: f32 = 400.0;

/// The layer, its input and its drawing.
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Drawn>().add_systems(
            Update,
            (
                enter.run_if(resource_added::<Editing>),
                leave.run_if(resource_removed::<Editing>),
                (route, refresh).chain().run_if(resource_exists::<Editing>),
            )
                .chain()
                .in_set(VectorSet::Screens),
        );
    }
}

fn enter(mut commands: Commands, mut drawn: ResMut<Drawn>) {
    *drawn = Drawn::default();
    commands
        .spawn((
            EditorRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(shade(0)),
        ))
        .with_children(|root| {
            root.spawn((
                Column::Tools,
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    width: Val::Px(TOOLS),
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::right(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(colour(Role::Dim)),
            ));
            root.spawn((
                Column::Plan,
                RelativeCursorPosition::default(),
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ))
            .with_children(|pane| {
                pane.spawn((
                    PlanImage,
                    ImageNode::default(),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                ));
                pane.spawn((
                    PlanCursor,
                    Node {
                        position_type: PositionType::Absolute,
                        border: UiRect::all(Val::Px(2.0)),
                        display: Display::None,
                        ..default()
                    },
                    BorderColor::all(colour(Role::Interface)),
                    Pickable::IGNORE,
                ));
            });
            root.spawn((
                Column::Inspector,
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    width: Val::Px(INSPECTOR),
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::left(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(colour(Role::Dim)),
            ));
        });
}

fn leave(mut commands: Commands, roots: Query<Entity, With<EditorRoot>>, mut drawn: ResMut<Drawn>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    *drawn = Drawn::default();
}

/// Use a control of the editor: the act applied to the model and, after a Save, the packs
/// read from disk again (the sealed pack is what a new game walks) and the editor told.
pub fn act(editing: &mut Editing, packs: &mut Packs, act: EditAct, step: i8) -> Asked {
    let asked = editing.apply(&packs.data, act, step);
    if asked == Asked::Saved {
        match Packs::load(&packs.config) {
            Ok(fresh) => {
                packs.data = fresh.data;
                editing.changed(&packs.data);
            }
            Err(e) => editing.message = Some(format!("Saved, but the packs did not reload: {e}")),
        }
    }
    asked
}

/// Whether the tools column offers `act` now: a dim control takes no press.
fn live(editing: &Editing, data: &omnis_sim::omnis_data::Data, act: EditAct) -> bool {
    view::tools(editing, data)
        .control(ControlId::Edit(act))
        .is_none_or(|c| c.why_not.is_none())
}

/// The pane's size in logical pixels once laid out; the model's last size before that.
fn pane_size(node: Option<&ComputedNode>, editing: &Editing) -> (u32, u32) {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    match node {
        Some(n) if n.size().x > 0.0 && n.size().y > 0.0 => (
            (n.size().x * n.inverse_scale_factor()).round() as u32,
            (n.size().y * n.inverse_scale_factor()).round() as u32,
        ),
        _ => editing.pane,
    }
}

/// The digit keys, for the first nine brushes.
const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// The layer's input: a pressed control or a choice's arrow; on the plan, a left click paints
/// the brush at the cell or the edge under the pointer (the cursor goes there) and a drag is
/// one stroke, a right click picks what is there; and the keys while no screen stands over the
/// editor and no field has the keyboard: the arrows and WASD move the cursor, Enter paints at it, the
/// digits choose brushes, `-` and `=` step the height, `[` and `]` zoom, C centres, Q and E
/// turn the facing, and with Cmd or Ctrl, Z undoes, Shift+Z and Y redo, S saves. Esc is the
/// screens' (`Nav::escape` opens the menu).
#[allow(clippy::too_many_arguments)]
fn route(
    buttons: Query<(&Interaction, &EditorPress), Changed<Interaction>>,
    arrows: Query<(&Interaction, &Arrow), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pane: Query<(&RelativeCursorPosition, &ComputedNode), With<Column>>,
    interactions: Query<&Interaction>,
    focus: Option<Res<InputFocus>>,
    fields: Query<(), With<EditableText>>,
    mut packs: ResMut<Packs>,
    mut editing: ResMut<Editing>,
    mut screens: ResMut<Screens>,
) {
    let covered = screens.nav.top(None).is_some();
    let typing = focus
        .as_ref()
        .and_then(|f| f.get())
        .is_some_and(|e| fields.get(e).is_ok());
    let data = packs.data.clone();
    let pressed = buttons
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, p)| (p.0, 0_i8));
    let stepped = || {
        arrows
            .iter()
            .find(|(i, a)| **i == Interaction::Pressed && matches!(a.0, ControlId::Edit(_)))
            .map(|(_, a)| (a.0, if a.1 { 1 } else { -1 }))
    };
    let mut asked = Asked::Nothing;
    if !covered
        && let Some((ControlId::Edit(a), step)) = pressed.or_else(stepped)
        && live(&editing, &data, a)
    {
        asked = act(&mut editing, &mut packs, a, step);
    }
    if !covered && !typing {
        let plan = pane.iter().find(|(c, _)| c.cursor_over()).map(|(c, node)| {
            (
                c.normalized.unwrap_or_default(),
                pane_size(Some(node), &editing),
            )
        });
        let over_button = interactions.iter().any(|i| *i != Interaction::None);
        // Bevy's `normalized` runs from (-0.5, -0.5) at the pane's top left to (0.5, 0.5) at its
        // bottom right; the plan's pixels run from its top left.
        let target = plan.filter(|_| !over_button).and_then(|(at, size)| {
            #[allow(clippy::cast_precision_loss)]
            let (px, py) = ((at.x + 0.5) * size.0 as f32, (at.y + 0.5) * size.1 as f32);
            editing.view.target(editing.size(), px, py)
        });
        if let Some(target) = target {
            if mouse.just_pressed(MouseButton::Right) {
                editing.pick(target);
            } else if mouse.just_pressed(MouseButton::Left) {
                editing.paint(&data, target, false);
            } else if mouse.pressed(MouseButton::Left)
                && editing.stroke.is_some_and(|last| last != target)
            {
                editing.paint(&data, target, true);
            }
        }
        if !mouse.pressed(MouseButton::Left) {
            editing.end_stroke();
        }
        let held = |list: [KeyCode; 2]| keys.any_pressed(list);
        let command = held([KeyCode::SuperLeft, KeyCode::SuperRight])
            || held([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let shift = held([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let open = editing.map_id().is_some();
        if command {
            let key_act = if keys.just_pressed(KeyCode::KeyZ) {
                Some(if shift { EditAct::Redo } else { EditAct::Undo })
            } else if keys.just_pressed(KeyCode::KeyY) {
                Some(EditAct::Redo)
            } else if keys.just_pressed(KeyCode::KeyS) {
                Some(EditAct::Save)
            } else {
                None
            };
            if let Some(a) = key_act
                && open
            {
                act(&mut editing, &mut packs, a, 0);
            }
        } else {
            for (keys_for, facing) in [
                ([KeyCode::ArrowUp, KeyCode::KeyW], Facing::North),
                ([KeyCode::ArrowRight, KeyCode::KeyD], Facing::East),
                ([KeyCode::ArrowDown, KeyCode::KeyS], Facing::South),
                ([KeyCode::ArrowLeft, KeyCode::KeyA], Facing::West),
            ] {
                if keys.any_just_pressed(keys_for) {
                    editing.step(facing);
                }
            }
            if open && keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
                let cell = editing.cursor;
                editing.paint(&data, Target { cell, edge: None }, false);
                editing.end_stroke();
            }
            let digit = DIGITS.iter().position(|k| keys.just_pressed(*k));
            let stepped = [
                (KeyCode::BracketLeft, EditAct::ZoomOut, 0),
                (KeyCode::BracketRight, EditAct::ZoomIn, 0),
                (KeyCode::KeyC, EditAct::Centre, 0),
                (KeyCode::KeyQ, EditAct::FaceLeft, 0),
                (KeyCode::KeyE, EditAct::FaceRight, 0),
                (KeyCode::Minus, EditAct::Eighths, -1),
                (KeyCode::Equal, EditAct::Eighths, 1),
            ]
            .into_iter()
            .find(|(key, _, _)| keys.just_pressed(*key))
            .map(|(_, a, step)| (a, step))
            .or_else(|| digit.map(|i| (EditAct::Brush(u8::try_from(i).unwrap_or(0)), 0)));
            if let Some((a, step)) = stepped
                && open
            {
                editing.apply(&data, a, step);
            }
        }
    }
    if asked == Asked::Menu {
        screens.nav.open(ScreenId::EditorMenu, None);
    }
}

/// Rebuild a column whose screen changed, and repaint the plan when what it shows changed.
#[allow(clippy::too_many_arguments)]
fn refresh(
    mut commands: Commands,
    packs: Res<Packs>,
    mut editing: ResMut<Editing>,
    mut drawn: ResMut<Drawn>,
    window: Query<&Window, With<PrimaryWindow>>,
    capture: Option<Res<super::capture::CaptureSize>>,
    columns: Query<(Entity, &Column, Option<&ComputedNode>)>,
    images: Option<ResMut<Assets<Image>>>,
    mut picture: Query<Picture, IsPicture>,
    mut cursor: Query<&mut Node, With<PlanCursor>>,
) {
    let height = super::combat::size(window.single().ok(), capture.as_deref()).1;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bucket = height as u32;
    let data = &packs.data;
    let pane_node = columns
        .iter()
        .find(|(_, c, _)| **c == Column::Plan)
        .and_then(|(_, _, n)| n);
    let pane = pane_size(pane_node, &editing);
    editing.laid_out(pane);
    let tools = view::tools(&editing, data);
    let inspector = view::inspector(&editing, data);
    for (entity, column, _) in &columns {
        let (want, last) = match column {
            Column::Tools => (&tools, &mut drawn.tools),
            Column::Inspector => (&inspector, &mut drawn.inspector),
            Column::Plan => continue,
        };
        if last
            .as_ref()
            .is_some_and(|(s, h)| s == want && *h == bucket)
        {
            continue;
        }
        *last = Some((want.clone(), bucket));
        commands.entity(entity).despawn_related::<Children>();
        commands
            .entity(entity)
            .with_children(|p| build(p, want, (height, Mark::Editor), None));
    }
    let key = (editing.revision, pane);
    if drawn.plan == Some(key) {
        return;
    }
    drawn.plan = Some(key);
    let (def, grid) = (editing.def(), editing.grid());
    let Some((def, grid)) = def.zip(grid) else {
        for mut node in &mut cursor {
            node.display = Display::None;
        }
        return;
    };
    let start = editing
        .level
        .is_ground()
        .then_some((def.start.0, def.start.1));
    let raster = plan::paint(
        Source {
            grid,
            terrains: &def.terrains,
            size: (def.width, def.height),
            level: editing.level,
            resolved: editing.resolved.as_ref(),
            start,
        },
        editing.view,
        pane,
    );
    if let Some(mut images) = images
        && let Ok((mut image, mut node)) = picture.single_mut()
    {
        #[allow(clippy::cast_precision_loss)]
        {
            node.width = Val::Px(raster.width as f32);
            node.height = Val::Px(raster.height as f32);
        }
        image.image = images.add(super::minimap::image(&raster));
    }
    let (left, top, side) = editing.view.cell_rect(editing.cursor);
    for mut node in &mut cursor {
        #[allow(clippy::cast_precision_loss)]
        {
            node.left = Val::Px(left as f32);
            node.top = Val::Px(top as f32);
            node.width = Val::Px(side as f32);
            node.height = Val::Px(side as f32);
        }
        node.display = Display::Flex;
        node.border = UiRect::all(Val::Px(scaled(2.0, height).max(1.0)));
    }
}
