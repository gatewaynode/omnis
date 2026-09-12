//! `HudPlugin`: map name, position and facing, the party clock, the last message. Native
//! resolution `bevy_ui` text on the outer camera (ARCHITECTURE.md §8.2).

use crate::sim::{Notice, PackData, SimEvent, SimSet, SimWorld};
use bevy::prelude::*;
use omnis_sim::{Event, MINUTES_PER_DAY};

/// Which line a text node shows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudLine {
    /// The map's display name.
    Map,
    /// Column, row, facing.
    Position,
    /// Day and time of the party clock.
    Clock,
    /// The last event or notice.
    Message,
    /// Key help.
    Help,
}

/// The HUD plugin.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, refresh.in_set(SimSet::Publish));
    }
}

fn line(kind: HudLine, size: f32) -> impl Bundle {
    (
        Text::new(""),
        TextFont::from_font_size(size),
        TextColor(Color::WHITE),
        kind,
    )
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        children![
            line(HudLine::Map, 22.0),
            line(HudLine::Position, 18.0),
            line(HudLine::Clock, 18.0)
        ],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        children![line(HudLine::Message, 18.0), line(HudLine::Help, 14.0)],
    ));
}

/// The clock as `Day d, hh:mm`.
#[must_use]
pub fn clock_text(elapsed: i64) -> String {
    let per_day = i64::from(MINUTES_PER_DAY);
    let day = elapsed.div_euclid(per_day) + 1;
    let minute = elapsed.rem_euclid(per_day);
    format!("Day {day}, {:02}:{:02}", minute / 60, minute % 60)
}

/// A one-line description of an event, or `None` for events the HUD does not mention.
#[must_use]
pub fn event_text(event: &Event) -> Option<String> {
    Some(match event {
        Event::Blocked { reason } => format!("Blocked: {reason:?}"),
        Event::Door { open: true, .. } => "The door opens.".into(),
        Event::Door { open: false, .. } => "The door closes.".into(),
        Event::Message { key } => key.text_key().to_owned(),
        Event::Moved { from, to } if from.map != to.map => "You pass through.".into(),
        Event::TimeAdvanced {
            day_rolled: true, ..
        } => "A new day.".into(),
        _ => return None,
    })
}

fn refresh(
    mut events: MessageReader<SimEvent>,
    notice: Res<Notice>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut lines: Query<(&mut Text, &HudLine)>,
) {
    let (Some(world), Some(data)) = (world, data) else {
        return;
    };
    let mut message = None;
    for SimEvent(event) in events.read() {
        if let Some(text) = event_text(event) {
            message = Some(text);
        }
    }
    if notice.is_changed() && !notice.0.is_empty() {
        message = Some(notice.0.clone());
    }
    let p = world.0.position;
    for (mut text, kind) in &mut lines {
        match kind {
            HudLine::Map => {
                let name = data
                    .0
                    .maps
                    .get(&p.map)
                    .map_or("?", |m| data.0.text("en", m.name));
                text.0 = name.to_owned();
            }
            HudLine::Position => text.0 = format!("({}, {}) facing {}", p.x, p.y, p.facing),
            HudLine::Clock => text.0 = clock_text(world.0.party_clock().elapsed),
            HudLine::Message => {
                if let Some(m) = &message {
                    text.0 = m.clone();
                }
            }
            HudLine::Help => {
                text.0 =
                    "Arrows/WASD move  QE sidestep  Space use  M map  F5 save  F9 load  Esc quit"
                        .into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_renders_days_and_minutes() {
        assert_eq!(clock_text(0), "Day 1, 00:00");
        assert_eq!(clock_text(61), "Day 1, 01:01");
        assert_eq!(clock_text(1440 * 3 + 725), "Day 4, 12:05");
    }
}
