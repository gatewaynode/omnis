//! The question before a step into or out of a town service, as a small `bevy_ui` panel in the
//! middle of the map: the question, Go and Stay. `confirm_panel.rs` is its model; the held step
//! and the Enter and Escape keys are `input.rs`'s. A button's report is a `ConfirmAnswer`, the
//! same as its key.

use crate::confirm_panel::ConfirmId;
use crate::input::{AskFirst, ConfirmAnswer};
use crate::menus::{Active, Where};
use crate::ui_kit::{PanelRoot, UiId, UiReport, UiScreen, button, row};
use crate::ui_model::Payload;
use bevy::feathers::constants::{fonts, size};
use bevy::feathers::controls::ButtonVariant;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeTextColor};
use bevy::feathers::tokens;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use omnis_sim::omnis_core::fnv1a64;

/// The panel: a see-through root over the viewport (`ui_kit::place` sizes it) holding a box
/// in its middle with the question over the two buttons.
fn confirm_panel(question: String) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(12),
                    padding: {UiRect::all(px(16))},
                }
                TabGroup
                ThemeBackgroundColor(tokens::WINDOW_BG)
                Children [
                    (
                        Text({question.clone()})
                        TextFont {
                            font: FontSourceTemplate::Handle(fonts::BOLD),
                            font_size: size::MEDIUM_FONT,
                        }
                        ThemeTextColor(tokens::TEXT_MAIN)
                    ),
                    (
                        row()
                        Children [
                            button(ConfirmId::Go.into(), "Go", ButtonVariant::Primary),
                            button(ConfirmId::Stay.into(), "Stay", ButtonVariant::Normal),
                        ]
                    ),
                ]
            ),
        ]
    }
}

/// Spawn the panel while a step is held, for its question; despawn it once answered.
pub fn reconcile(
    mut commands: Commands,
    at: Where,
    held: Res<AskFirst>,
    roots: Query<(Entity, &PanelRoot)>,
) {
    let wanted = held.0.as_ref().filter(|_| at.screen() == Active::Confirm);
    let shape = wanted.map(|c| fnv1a64(c.question.as_bytes()));
    let mut standing = false;
    for (entity, root) in &roots {
        if root.screen != UiScreen::Confirm {
            continue;
        }
        if Some(root.shape) == shape && !standing {
            standing = true;
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let (Some(confirm), Some(shape), false) = (wanted, shape, standing) {
        commands
            .spawn_scene(confirm_panel(confirm.question.clone()))
            .insert(PanelRoot {
                screen: UiScreen::Confirm,
                shape,
            });
    }
}

/// Go and Stay, pressed.
pub fn reports(mut reports: MessageReader<UiReport>, mut answers: MessageWriter<ConfirmAnswer>) {
    for report in reports.read() {
        if let (UiId::Confirm(id), Payload::Activate) = (report.id, &report.payload) {
            answers.write(ConfirmAnswer(id));
        }
    }
}
