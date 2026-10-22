//! The editor's layer (M5.4; `tasks/plans/m5-editor-v1.md` §Client side, presentation-ARCHITECTURE
//! §6): while the `Editing` resource is there, three columns stand in place of the 3D view:
//! the tools (a kit screen), the plan (the Bevy-free raster in one image node, with the cursor
//! as a bordered node over it) and the inspector (a kit screen). Built like the fight screen:
//! the columns are rebuilt only when their screen changed, the plan repainted only when what
//! it shows changed. The editor runs with no `Session`, so nothing of the game moves or draws;
//! its menu is a screen of the kit (`ScreenId::EditorMenu`). Headless-safe: without
//! `Assets<Image>` the plan is not painted, and the rest stands.

use super::VectorSet;
use super::screen::{Arrow, Screens};
use super::screen_draw::{EditorPress, Mark, build, colour, shade};
use super::session::Packs;
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

/// The layer's input: a pressed control or a choice's arrow, a left click on the plan (the
/// cursor goes to the cell), and the keys while no screen stands over the editor and no field
/// has the keyboard: the arrows move the cursor, `[` and `]` zoom, C centres, Q and E turn the
/// facing. Esc is the screens' (`Nav::escape` opens the menu).
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
    packs: Res<Packs>,
    mut editing: ResMut<Editing>,
    mut screens: ResMut<Screens>,
) {
    let covered = screens.nav.top(None).is_some();
    let typing = focus
        .as_ref()
        .and_then(|f| f.get())
        .is_some_and(|e| fields.get(e).is_ok());
    let data = &packs.data;
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
    if !covered && let Some((ControlId::Edit(act), step)) = pressed.or_else(stepped) {
        asked = editing.apply(data, act, step);
    }
    if !covered && !typing {
        let plan = pane.iter().find(|(c, _)| c.cursor_over()).map(|(c, node)| {
            (
                c.normalized.unwrap_or_default(),
                pane_size(Some(node), &editing),
            )
        });
        let over_button = interactions.iter().any(|i| *i != Interaction::None);
        if mouse.just_pressed(MouseButton::Left)
            && !over_button
            && let Some((at, size)) = plan
        {
            #[allow(clippy::cast_precision_loss)]
            let (px, py) = (at.x * size.0 as f32, at.y * size.1 as f32);
            if let Some(cell) = editing.view.hit(editing.size(), px, py) {
                editing.point(cell);
            }
        }
        for (key, facing) in [
            (KeyCode::ArrowUp, Facing::North),
            (KeyCode::ArrowRight, Facing::East),
            (KeyCode::ArrowDown, Facing::South),
            (KeyCode::ArrowLeft, Facing::West),
        ] {
            if keys.just_pressed(key) {
                editing.step(facing);
            }
        }
        for (key, act) in [
            (KeyCode::BracketLeft, EditAct::ZoomOut),
            (KeyCode::BracketRight, EditAct::ZoomIn),
            (KeyCode::KeyC, EditAct::Centre),
            (KeyCode::KeyQ, EditAct::FaceLeft),
            (KeyCode::KeyE, EditAct::FaceRight),
        ] {
            if keys.just_pressed(key) && editing.map_id().is_some() {
                editing.apply(data, act, 0);
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
