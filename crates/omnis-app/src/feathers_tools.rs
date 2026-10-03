//! The tool bar as a `bevy_ui` scene (M7 step 8a; PRD D26: the canvas's widgets move to
//! Feathers): seven buttons in a four-column grid over the canvas's tool strip, where the
//! canvas painted its six. `tool_bar.rs` is the model: which buttons are live, and the gate a
//! press passes. The bar stands while the tool states show it (a world, while playing) and is
//! never a panel, so the modal screens are untouched by it.

use crate::tool_bar::{ToolButton, ToolPressed, ToolStates};
use crate::ui_kit::{Control, ToolBar, UiId, UiReport, button};
use crate::ui_model::Payload;
use crate::widget::PadState;
use bevy::feathers::controls::ButtonVariant;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

/// Columns of the bar's grid: the first row holds four, the second three.
const COLUMNS: u16 = 4;

/// So many equal grid tracks.
fn tracks(count: u16) -> Vec<RepeatedGridTrack> {
    RepeatedGridTrack::flex(count, 1.0)
}

fn tool_button(tool: ToolButton) -> impl Scene {
    bsn! {
        button(UiId::Tool(tool), tool.label(), ButtonVariant::Normal)
        Node { width: percent(100), height: percent(100) }
    }
}

/// The bar: a see-through grid over the tool strip (`ui_kit::place` sizes it).
fn tool_bar() -> impl Scene {
    let buttons: Vec<_> = ToolButton::ALL.into_iter().map(tool_button).collect();
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            display: Display::Grid,
            grid_template_columns: {tracks(COLUMNS)},
            grid_template_rows: {tracks(2)},
            column_gap: px(6),
            row_gap: px(6),
        }
        Children [ {buttons} ]
    }
}

/// Spawn the bar while the states show it, and despawn it when they hide it.
pub fn reconcile(
    mut commands: Commands,
    states: Res<ToolStates>,
    bars: Query<Entity, With<ToolBar>>,
) {
    let wanted = states.shown();
    let mut standing = false;
    for entity in &bars {
        if wanted && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if wanted && !standing {
        commands.spawn_scene(tool_bar()).insert(ToolBar);
    }
}

/// Dim each button its state does not make live.
pub fn sync(
    mut commands: Commands,
    states: Res<ToolStates>,
    buttons: Query<(Entity, &Control, Has<InteractionDisabled>)>,
) {
    for (entity, control, dim) in &buttons {
        let UiId::Tool(tool) = control.0 else {
            continue;
        };
        let wanted = states.get(tool) != PadState::Enabled;
        if wanted && !dim {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !wanted && dim {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
}

/// A button pressed is a press for the model to gate.
pub fn reports(mut reports: MessageReader<UiReport>, mut presses: MessageWriter<ToolPressed>) {
    for report in reports.read() {
        if let (UiId::Tool(tool), Payload::Activate) = (report.id, &report.payload) {
            presses.write(ToolPressed(tool));
        }
    }
}
