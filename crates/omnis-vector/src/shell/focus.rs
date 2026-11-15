//! Who holds the keyboard among a screen's text fields (the owner's report, 2026-11-15: "I
//! could only write in one direction field, the cursor disappeared if I tried to choose any
//! other field, I couldn't tab or arrow through the form fields"). Since Bevy 0.20 a click
//! gives the keyboard only to a node with a `TabIndex` and takes it from everything else, so
//! every field carries one ([`field`]); a click on a field's box is a click on its field; Tab
//! and Shift+Tab step through the fields of the screen on top, or of the editor's columns, and
//! Up and Down do the same from a field of one line.

use bevy::input_focus::tab_navigation::{NavAction, TabGroup, TabIndex, TabNavigation};
use bevy::input_focus::{AcquireFocus, FocusCause, InputFocus, InputFocusSystems};
use bevy::prelude::*;
use bevy::text::EditableText;

/// What makes a text field one a click or Tab can give the keyboard to.
#[must_use]
pub const fn field() -> TabIndex {
    TabIndex(0)
}

/// The fields under a screen's root: Tab stays among them while the screen stands.
#[must_use]
pub fn screen() -> TabGroup {
    TabGroup::modal()
}

/// The fields of one of the editor's columns: Tab goes on from one column to the next.
#[must_use]
pub fn column(order: i32) -> TabGroup {
    TabGroup::new(order)
}

/// The box's click and the keys that step from field to field. Bevy's own Tab handler is left
/// out: with no field holding the keyboard it would give it to the editor's form under a
/// screen standing over it.
pub fn plugin(app: &mut App) {
    app.add_observer(boxed)
        .add_systems(PreUpdate, steps.after(InputFocusSystems::Dispatch));
}

/// A click on the box around a field (its padding, the space beside a short text) gives the
/// keyboard to the field, where Bevy would take it from every field.
fn boxed(
    mut asked: On<AcquireFocus>,
    boxes: Query<&Children>,
    fields: Query<(), (With<EditableText>, With<TabIndex>)>,
    mut focus: ResMut<InputFocus>,
) {
    let Ok(children) = boxes.get(asked.focused_entity) else {
        return;
    };
    let mut inside = children.iter().filter(|c| fields.contains(*c));
    // Only the box of one field: a panel that holds several is not a field's box.
    if let (Some(only), None) = (inside.next(), inside.next()) {
        asked.propagate(false);
        if focus.get() != Some(only) {
            focus.set(only, FocusCause::Pressed);
        }
    }
}

/// Tab and Shift+Tab go from the field that holds the keyboard to the next and the one before,
/// among the fields of its screen or of the editor's columns; Up and Down do the same from a
/// field of one line (a field of several keeps the arrows for its own lines). With no field
/// holding the keyboard the keys are someone else's.
fn steps(
    keys: Res<ButtonInput<KeyCode>>,
    fields: Query<&EditableText, With<TabIndex>>,
    nav: TabNavigation,
    focus: Option<ResMut<InputFocus>>,
) {
    let Some(mut focus) = focus else {
        return;
    };
    let Some(field) = focus.get().and_then(|e| fields.get(e).ok()) else {
        return;
    };
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let one_line = !field.allow_newlines;
    let action = if keys.just_pressed(KeyCode::Tab) {
        if shift {
            NavAction::Previous
        } else {
            NavAction::Next
        }
    } else if one_line && keys.just_pressed(KeyCode::ArrowDown) {
        NavAction::Next
    } else if one_line && keys.just_pressed(KeyCode::ArrowUp) {
        NavAction::Previous
    } else {
        return;
    };
    if let Ok(next) = nav.navigate(&focus, action) {
        focus.set(next, FocusCause::Navigated);
    }
}
