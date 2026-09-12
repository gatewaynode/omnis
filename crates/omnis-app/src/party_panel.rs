//! `PartyPanelPlugin`: the six party rows in the sidebar column under the minimap, as UI
//! text positioned over the canvas rectangle `layout::SIDEBAR_PARTY` and sized to the
//! integer scale, so it tracks the window like the pixels do.

use crate::layout::{SIDEBAR_PARTY, canvas_rect_to_window, window_scale};
use crate::sim::{PackData, SimSet, SimWorld};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The panel root.
#[derive(Component)]
struct PartyRoot;

/// One member's row.
#[derive(Component)]
struct PartyRow(usize);

/// Canvas pixels of text per row, before scaling.
const ROW_FONT: f32 = 7.0;

/// The party panel plugin.
pub struct PartyPanelPlugin;

impl Plugin for PartyPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, refresh.in_set(SimSet::Publish));
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            PartyRoot,
        ))
        .with_children(|parent| {
            for i in 0..6 {
                parent.spawn((
                    Text::new(""),
                    TextFont::from_font_size(ROW_FONT),
                    TextColor(Color::WHITE),
                    PartyRow(i),
                ));
            }
        });
}

/// One member's two lines: a front-row marker and name, then hit and spell points.
#[must_use]
pub fn row_text(name: &str, front: bool, hp: i32, hp_max: i32, spell_points: u32) -> String {
    let marker = if front { '*' } else { ' ' };
    format!("{marker}{name}\n {hp}/{hp_max} sp {spell_points}")
}

fn refresh(
    window: Query<&Window, With<PrimaryWindow>>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut root: Query<&mut Node, With<PartyRoot>>,
    mut rows: Query<(&mut Text, &mut TextFont, &PartyRow)>,
) {
    let scale = window.single().map_or(1.0, |window| {
        let (w, h) = (window.width(), window.height());
        let (left, top, width, height) = canvas_rect_to_window(SIDEBAR_PARTY, w, h);
        for mut node in &mut root {
            node.left = Val::Px(left);
            node.top = Val::Px(top);
            node.width = Val::Px(width);
            node.height = Val::Px(height);
        }
        window_scale(w, h)
    });
    let front_row = data
        .as_ref()
        .map_or(3, |d| omnis_sim::party::front_row(&d.0));
    for (mut text, mut font, PartyRow(i)) in &mut rows {
        let wanted = world
            .as_ref()
            .and_then(|w| w.0.party.members.get(*i))
            .map(|m| row_text(&m.name, *i < front_row, m.hp, m.hp_max, m.spell_points))
            .unwrap_or_default();
        if text.0 != wanted {
            text.0 = wanted;
        }
        let size = ROW_FONT * scale;
        if font.font_size != bevy::text::FontSize::Px(size) {
            font.font_size = bevy::text::FontSize::Px(size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_show_the_row_marker_and_the_points() {
        assert_eq!(row_text("Brenna", true, 12, 12, 0), "*Brenna\n 12/12 sp 0");
        assert_eq!(row_text("Ilvara", false, 7, 7, 4), " Ilvara\n 7/7 sp 4");
    }
}
