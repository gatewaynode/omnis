//! The editor's keys, while no screen stands over the editor and no field has the keyboard
//! (`editor::route` asks each frame): the arrows and WASD move the cursor, Enter paints at it
//! (or takes a pending wall run; Backspace drops the run's last corner), the digits choose
//! palette entries, V, B, M, L, F, P, N and X the tools (`Tool::key`), `-` and `=` step the
//! height, `[` and `]` zoom, C centres, Q and E turn the facing, and with Cmd or Ctrl, Z
//! undoes, Shift+Z and Y redo, S saves. Esc is the screens' (`Nav::escape` opens the menu).

use super::editor::{Editing, act};
use super::session::Packs;
use crate::editor::EditAct;
use crate::editor::brush::{Target, Tool};
use bevy::prelude::*;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;

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

/// The frame's keys applied to the editor.
pub(super) fn keys(
    keys: &ButtonInput<KeyCode>,
    editing: &mut Editing,
    packs: &mut Packs,
    data: &Data,
) {
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
            act(editing, packs, a, 0);
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
            if editing.run.is_some() {
                editing.take_run(data);
            } else {
                let cell = editing.cursor;
                editing.paint(data, Target { cell, edge: None }, false);
                editing.end_stroke();
            }
        }
        if open && keys.just_pressed(KeyCode::Backspace) && editing.run.is_some() {
            editing.drop_corner();
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
        .or_else(|| digit.map(|i| (EditAct::Brush(u8::try_from(i).unwrap_or(0)), 0)))
        .or_else(|| {
            // The tools' keys (V, B, M, L, F, P, N, X: `Tool::key`), shortcuts for the
            // grid's icons.
            [
                (KeyCode::KeyV, Tool::Select),
                (KeyCode::KeyB, Tool::Brush),
                (KeyCode::KeyM, Tool::Move),
                (KeyCode::KeyL, Tool::Lasso),
                (KeyCode::KeyF, Tool::Fill),
                (KeyCode::KeyP, Tool::Corners),
                (KeyCode::KeyN, Tool::Freehand),
                (KeyCode::KeyX, Tool::Eraser),
            ]
            .into_iter()
            .find(|(key, _)| keys.just_pressed(*key))
            .map(|(_, tool)| (EditAct::Tool(tool), 0))
        });
        if let Some((a, step)) = stepped
            && open
        {
            editing.apply(data, a, step);
        }
    }
}
