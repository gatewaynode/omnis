//! The editor's layer (M5.4; `tasks/plans/m5-editor-v1.md` §Client side, presentation-ARCHITECTURE
//! §6): while the `Editing` resource is there, three columns stand in place of the 3D view:
//! the tools (a kit screen), the plan (the Bevy-free raster in one image node, with the cursor
//! as a bordered node over it, and since M5.6 the attribute bar over it, a kit screen) and the
//! inspector (a kit screen). Built like the fight screen: a column is rebuilt only when its
//! screen's shape changed and its words are rewritten in place when only they did (so a
//! field keeps its caret), the plan repainted only when what it shows changed. The pointer paints (M5.5): a left click paints the brush at what the
//! pointer aims at (`Editor::aim`, shown as a hover over the plan before the click) and a drag
//! is one stroke, a right click picks what is there. The editor
//! runs with no `Session`, so nothing of the game moves or draws; its menu is a screen of the
//! kit (`ScreenId::EditorMenu`). Headless-safe: without `Assets<Image>` the plan is not
//! painted, and the rest stands.

use super::screen::{Arrow, Screens, Slid};
use super::screen_draw::{EditorPress, Mark, Placed, Scrolls, build, colour, shade, sync};
use super::session::{Packs, Session};
use super::{VectorSet, ViewState};
use crate::editor::brush::Target;
use crate::editor::plan::{self, Source};
use crate::editor::{Asked, EditAct, Editor, playtest, view};
use crate::pose::Pose;
use crate::screen::{ControlId, Kind, Screen, ScreenId};
use crate::sim::game_settings;
use crate::style::{Role, scaled};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;
use omnis_sim::omnis_core::Facing;
use omnis_sim::{Command, Settings};

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

/// The editor parked while its playtest runs (M5.8): the whole model, untouched, so Back to
/// editor brings it back as it was left. Removing `Editing` takes the layer and the preview
/// down through their own hooks; inserting it again rebuilds them from this.
#[derive(Resource, Debug)]
pub struct Playtesting(pub Editor);

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
    /// The attribute bar over the plan (M5.6).
    Bar,
    /// The preview's picture over the inspector (M5.7); only with a renderer.
    Preview,
    /// The inspector, right.
    Inspector,
}

/// The plan's picture.
#[derive(Component)]
pub struct PlanImage;

/// The cursor's border over the plan.
#[derive(Component)]
pub struct PlanCursor;

/// The aim under the pointer, over the plan: a cell's outline, or a bar along the side an edge
/// brush would paint.
#[derive(Component)]
pub struct PlanHover;

/// The plan's picture node: its image and its size.
type Picture = (&'static mut ImageNode, &'static mut Node);
/// Which node the picture is.
type IsPicture = (With<PlanImage>, Without<PlanCursor>, Without<PlanHover>);
/// Which node the cursor is.
type IsCursor = (With<PlanCursor>, Without<PlanHover>);
/// The hover node: its place and its fill.
type Hover = (&'static mut Node, &'static mut BackgroundColor);

/// What the layer last drew.
#[derive(Resource, Debug, Default)]
struct Drawn {
    tools: Option<(Screen, u32)>,
    bar: Option<(Screen, u32)>,
    inspector: Option<(Screen, u32)>,
    plan: Option<(u64, (u32, u32))>,
    hover: Option<(Option<Target>, u64, (u32, u32))>,
    /// The scroll of a column rebuilt last frame, for its new panel: a column is rebuilt where
    /// it stood, so stepping the height never sends the brushes back to the top.
    scrolls: Vec<(Column, f32)>,
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

fn enter(
    mut commands: Commands,
    mut drawn: ResMut<Drawn>,
    picture: Option<Res<super::preview::PreviewTarget>>,
) {
    *drawn = Drawn::default();
    let picture = picture.map(|p| p.0.clone());
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
            // The centre: the bar over the plan.
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                ..default()
            })
            .with_children(|centre| {
                centre.spawn((
                    Column::Bar,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 0.0,
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        row_gap: Val::Px(6.0),
                        border: UiRect::bottom(Val::Px(1.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BorderColor::all(colour(Role::Dim)),
                ));
                centre
                    .spawn((
                        Column::Plan,
                        RelativeCursorPosition::default(),
                        Node {
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            min_height: Val::Px(0.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::BLACK),
                    ))
                    .with_children(plan_nodes);
            });
            // The right: the preview's picture (with a renderer) over the inspector.
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    width: Val::Px(INSPECTOR),
                    min_height: Val::Px(0.0),
                    border: UiRect::left(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(colour(Role::Dim)),
            ))
            .with_children(|right| {
                if let Some(image) = picture {
                    let width = INSPECTOR - 24.0;
                    right.spawn((
                        Column::Preview,
                        ImageNode::new(image),
                        Node {
                            width: Val::Px(width),
                            height: Val::Px(super::preview::picture_height(width)),
                            margin: UiRect::new(
                                Val::Px(12.0),
                                Val::Px(12.0),
                                Val::Px(12.0),
                                Val::Px(0.0),
                            ),
                            flex_shrink: 0.0,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(colour(Role::Dim)),
                    ));
                }
                right.spawn((
                    Column::Inspector,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(8.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ));
            });
        });
}

/// The plan's nodes under its pane: the picture, the cursor and the hover.
fn plan_nodes(pane: &mut ChildSpawnerCommands) {
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
    pane.spawn((
        PlanHover,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        BorderColor::all(colour(Role::Caution)),
        BackgroundColor(Color::NONE),
        Pickable::IGNORE,
    ));
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

/// A game on the edited map as the editor holds it, the fixed party standing on the cursor's
/// cell facing as the cursor does (M5.8). The packs are copied with the map put in, never
/// changed.
///
/// # Errors
/// What stops it (`playtest::blocked`), a game that will not start, or a teleport refused.
pub fn start_playtest(editing: &Editor, packs: &Packs) -> Result<Session, String> {
    if let Some(why) = playtest::blocked(editing) {
        return Err(why);
    }
    let data = playtest::data_for(editing, &packs.data).ok_or("the map has not loaded")?;
    let temp = Packs {
        data,
        config: packs.config.clone(),
    };
    let mut session = Session::with_party(
        &temp,
        packs.config.seed,
        game_settings(Settings::default()),
        &crate::party::fixed(),
    )?;
    let step = playtest::teleport(editing).ok_or("open a map first")?;
    session.ordered(Command::Dev(step))?;
    session.pose = Pose::at(session.views.position);
    Ok(session)
}

/// Whether the columns offer `act` now: a dim control takes no press.
fn live(editing: &Editing, data: &omnis_sim::omnis_data::Data, act: EditAct) -> bool {
    let id = ControlId::Edit(act);
    [
        view::tools(editing, data),
        view::bar(editing, data),
        view::inspector(editing, data),
    ]
    .iter()
    .find_map(|s| s.control(id).map(|c| c.why_not.is_none()))
    .unwrap_or(true)
}

/// The first text field of `screen`, which takes the keyboard when the screen is drawn anew.
fn first_field(screen: &Screen) -> Option<ControlId> {
    screen
        .controls()
        .find(|c| matches!(c.kind, Kind::Text { .. }))
        .map(|c| c.id)
}

/// What a column needs this frame: nothing, its words rewritten, or its nodes built anew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Need {
    Nothing,
    Words,
    Nodes,
}

/// Whether the column drawn as `last` shows `want` already, needs only its words rewritten
/// (the same shape and height bucket, an outcome line in both or neither) or is built anew.
fn need(last: Option<&(Screen, u32)>, want: &Screen, bucket: u32) -> Need {
    match last {
        Some((s, h)) if *h == bucket && s == want => Need::Nothing,
        Some((s, h))
            if *h == bucket
                && s.shape() == want.shape()
                && s.message.is_some() == want.message.is_some() =>
        {
            Need::Words
        }
        _ => Need::Nodes,
    }
}

/// The hover node over `aim`: a cell's rectangle as a thin outline, an edge as a filled bar
/// along that side, a sixth of the side thick; hidden with no aim.
fn place_hover(
    node: &mut Node,
    fill: &mut BackgroundColor,
    aim: Option<((i64, i64, u32), Option<Facing>)>,
) {
    let Some(((left, top, side), edge)) = aim else {
        node.display = Display::None;
        return;
    };
    let side = i64::from(side);
    let thick = (side / 6).max(2);
    let (x, y, w, h) = match edge {
        None => (left, top, side, side),
        Some(Facing::North) => (left, top - thick / 2, side, thick),
        Some(Facing::South) => (left, top + side - thick / 2, side, thick),
        Some(Facing::West) => (left - thick / 2, top, thick, side),
        Some(Facing::East) => (left + side - thick / 2, top, thick, side),
    };
    #[allow(clippy::cast_precision_loss)]
    {
        node.left = Val::Px(x as f32);
        node.top = Val::Px(y as f32);
        node.width = Val::Px(w as f32);
        node.height = Val::Px(h as f32);
    }
    node.border = UiRect::all(Val::Px(if edge.is_some() { 0.0 } else { 1.0 }));
    fill.0 = if edge.is_some() {
        colour(Role::Caution)
    } else {
        Color::NONE
    };
    node.display = Display::Flex;
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

/// The layer's input: a pressed control or a choice's arrow; on the plan, the pointer's aim
/// (`Editor::aim`: the cell under it, or with an edge brush its nearest side) is kept as the
/// hover, a left click paints the brush there (the cursor goes to the cell) and a drag is one
/// stroke, a right click picks what is there; and the keys while no screen stands over the
/// editor and no field has the keyboard: the arrows and WASD move the cursor, Enter paints at it, the
/// digits choose brushes, `-` and `=` step the height, `[` and `]` zoom, C centres, Q and E
/// turn the facing, and with Cmd or Ctrl, Z undoes, Shift+Z and Y redo, S saves. Esc is the
/// screens' (`Nav::escape` opens the menu). A click on the plan while a field has the keyboard
/// takes it from the field: clicking elsewhere is the way out of a field.
#[allow(clippy::too_many_arguments)]
fn route(
    buttons: Query<(&Interaction, &EditorPress), Changed<Interaction>>,
    arrows: Query<(&Interaction, &Arrow), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pane: Query<(&RelativeCursorPosition, &ComputedNode), With<Column>>,
    interactions: Query<&Interaction>,
    focus: Option<ResMut<InputFocus>>,
    fields: Query<(Ref<EditableText>, &EditorPress)>,
    mut slid: ResMut<Slid>,
    mut packs: ResMut<Packs>,
    mut editing: ResMut<Editing>,
    mut screens: ResMut<Screens>,
    mut commands: Commands,
    mut view: ResMut<NextState<ViewState>>,
) {
    let covered = screens.nav.top(None).is_some();
    let typing = focus
        .as_ref()
        .and_then(|f| f.get())
        .is_some_and(|e| fields.get(e).is_ok());
    let data = packs.data.clone();
    if !covered {
        // Typed text goes to the open form's field; the words on screen are the field's own.
        for (field, press) in &fields {
            if let ControlId::Edit(EditAct::Field(i)) = press.0
                && field.is_changed()
            {
                let text = field.value().to_string();
                if editing
                    .form
                    .as_ref()
                    .is_some_and(|f| f.text(usize::from(i)) != text)
                {
                    editing.typed(i, text);
                }
            }
        }
        if let Some((ControlId::Edit(EditAct::Field(i)), n)) = slid.0.take() {
            editing.set_field(i, n);
        }
    }
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
    if !covered {
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
            editing.aim(px, py)
        });
        if editing.hover != target {
            editing.hover = target;
        }
        if let Some(target) = target {
            let clicked =
                mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right);
            if clicked
                && typing
                && let Some(mut focus) = focus
            {
                focus.clear();
            }
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
    }
    if !covered && !typing {
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
    if asked == Asked::Playtest {
        match start_playtest(&editing, &packs) {
            Ok(session) => {
                // The editor is parked whole; the game takes its place until Back to editor.
                commands.queue(|world: &mut World| {
                    if let Some(parked) = world.remove_resource::<Editing>() {
                        world.insert_resource(Playtesting(parked.0));
                    }
                });
                screens.reset(Some(&session.views));
                screens.nav.edit(false);
                view.set(ViewState::Explore);
                commands.insert_resource(session);
            }
            Err(why) => editing.message = Some(format!("No playtest: {why}")),
        }
    }
}

/// Redraw a column whose screen changed (its words in place when its shape stands, its nodes
/// anew otherwise; the field with the keyboard keeps it), and repaint the plan when what it
/// shows changed.
#[allow(clippy::too_many_arguments)]
fn refresh(
    mut commands: Commands,
    packs: Res<Packs>,
    editing: Option<ResMut<Editing>>,
    mut drawn: ResMut<Drawn>,
    window: Query<&Window, With<PrimaryWindow>>,
    capture: Option<Res<super::capture::CaptureSize>>,
    columns: Query<(Entity, &Column, Option<&ComputedNode>)>,
    images: Option<ResMut<Assets<Image>>>,
    mut picture: Query<Picture, IsPicture>,
    mut cursor: Query<&mut Node, IsCursor>,
    mut hovers: Query<Hover, With<PlanHover>>,
    mut panels: Query<(Entity, &mut ScrollPosition), With<Scrolls>>,
    parents: Query<&ChildOf>,
    mut placed: Query<Placed>,
    focus: Option<Res<InputFocus>>,
    fields: Query<&EditorPress, With<EditableText>>,
) {
    // `route` runs before this under one condition; when it starts a playtest it takes the
    // editor away in between (M5.8), and there is nothing to draw this frame.
    let Some(mut editing) = editing else {
        return;
    };
    let column_of = |panel: Entity| {
        parents
            .iter_ancestors(panel)
            .find_map(|a| columns.get(a).ok().map(|(_, c, _)| *c))
    };
    // A panel built last frame takes the scroll of the one it replaced.
    for (panel, mut at) in &mut panels {
        if at.is_added()
            && let Some(column) = column_of(panel)
            && let Some(i) = drawn.scrolls.iter().position(|(c, _)| *c == column)
        {
            at.0.y = drawn.scrolls.swap_remove(i).1;
        }
    }
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
    let bar = view::bar(&editing, data);
    let inspector = view::inspector(&editing, data);
    let focused = focus
        .as_ref()
        .and_then(|f| f.get())
        .and_then(|e| fields.get(e).ok())
        .map(|p| p.0);
    for (entity, column, _) in &columns {
        let (want, last) = match column {
            Column::Tools => (&tools, &mut drawn.tools),
            Column::Bar => (&bar, &mut drawn.bar),
            Column::Inspector => (&inspector, &mut drawn.inspector),
            Column::Plan | Column::Preview => continue,
        };
        let need = need(last.as_ref(), want, bucket);
        if need == Need::Nothing {
            continue;
        }
        *last = Some((want.clone(), bucket));
        if need == Need::Words {
            sync(want, &mut placed);
            continue;
        }
        // The field that has the keyboard keeps it through the rebuild when the column still
        // has it; a form just opened gives it to its first field. (A rebuild for any other
        // reason leaves the keyboard where the owner put it, which may be nowhere.)
        let fresh = editing.form.as_ref().is_some_and(|f| f.fresh);
        let keep = focused
            .filter(|id| want.control(*id).is_some())
            .or_else(|| fresh.then(|| first_field(want)).flatten());
        if *column == Column::Inspector
            && let Some(form) = &mut editing.form
        {
            form.fresh = false;
        }
        if let Some(y) = panels
            .iter()
            .find(|(p, _)| column_of(*p) == Some(*column))
            .map(|(_, at)| at.0.y)
        {
            drawn.scrolls.retain(|(c, _)| c != column);
            drawn.scrolls.push((*column, y));
        }
        commands.entity(entity).despawn_related::<Children>();
        commands
            .entity(entity)
            .with_children(|p| build(p, want, (height, Mark::Editor), keep));
    }
    let hover_key = (editing.hover, editing.revision, pane);
    if drawn.hover != Some(hover_key) {
        drawn.hover = Some(hover_key);
        let aim = editing.hover.filter(|_| editing.grid().is_some());
        for (mut node, mut fill) in &mut hovers {
            place_hover(
                &mut node,
                &mut fill,
                aim.map(|t| (editing.view.cell_rect(t.cell), t.edge)),
            );
        }
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
    let marks = plan::marks(&def, data, editing.level);
    let raster = plan::paint(
        Source {
            grid,
            terrains: &def.terrains,
            size: (def.width, def.height),
            level: editing.level,
            resolved: editing.resolved.as_ref(),
            start,
            marks: &marks,
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
