//! The HUD: a status block and the recent event lines (alt-ARCHITECTURE.md §8).

use super::controls::GREEN;
use super::movement::Intent;
use super::session::Session;
use super::{VectorSet, ViewState};
use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use omnis_sim::Mode;

/// Marks the status text.
#[derive(Component)]
pub struct StatusText;

/// The status block, top left; in a fight, its first lines only, at the bottom left under the
/// action column (`arena::Layout::status`).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn)
            .add_systems(Update, status.in_set(VectorSet::Draw));
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        StatusText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(GREEN),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        },
    ));
}

/// Day, hour and minute from elapsed party minutes.
#[must_use]
pub fn clock(elapsed: i64) -> String {
    let day = elapsed.div_euclid(1440) + 1;
    let minute = elapsed.rem_euclid(1440);
    format!("Day {day} {:02}:{:02}", minute / 60, minute % 60)
}

fn mode_name(mode: &Mode) -> &'static str {
    match mode {
        Mode::Explore => "Exploring",
        Mode::Encounter(_) => "Encounter",
        Mode::Combat(_) => "Combat",
    }
}

/// The frame rate and frame time, smoothed over recent frames by Bevy's diagnostics, so the
/// reading is steady enough to report; one frame's rate when the diagnostics are absent.
fn rate(diagnostics: Option<&DiagnosticsStore>, time: &Time) -> String {
    let smoothed = |path| {
        diagnostics
            .and_then(|d| d.get(path))
            .and_then(Diagnostic::smoothed)
    };
    match (
        smoothed(&FrameTimeDiagnosticsPlugin::FPS),
        smoothed(&FrameTimeDiagnosticsPlugin::FRAME_TIME),
    ) {
        (Some(fps), Some(ms)) => format!("{fps:.0} fps  {ms:.1} ms"),
        _ => format!("{:.0} fps", 1.0 / time.delta_secs().max(1e-4)),
    }
}

fn status(
    session: Res<Session>,
    time: Res<Time>,
    diagnostics: Option<Res<DiagnosticsStore>>,
    intent: Res<Intent>,
    window: Query<&Window, With<PrimaryWindow>>,
    view: Option<Res<State<ViewState>>>,
    mut text: Query<(&mut Text, &mut TextFont, &mut Node), With<StatusText>>,
) {
    let Ok((mut text, mut font, mut node)) = text.single_mut() else {
        return;
    };
    let fight = view.is_some_and(|v| *v.get() == ViewState::Fight);
    let (top, bottom) = if fight {
        (Val::Auto, Val::Px(16.0))
    } else {
        (Val::Px(16.0), Val::Auto)
    };
    if node.top != top {
        // Smaller in a fight, to fit under the action column.
        node.top = top;
        node.bottom = bottom;
        font.font_size = FontSize::Px(if fight { 14.0 } else { 18.0 });
    }
    let w = &session.world;
    let map = session
        .data
        .maps
        .get(&w.position.map)
        .map_or("?", |m| m.def.id.as_str());
    let mut rate = rate(diagnostics.as_deref(), &time);
    if let Ok(window) = window.single() {
        // Physical pixels: what the frame rate was measured at.
        rate.push_str(&format!(
            "  {}x{}",
            window.physical_width(),
            window.physical_height()
        ));
    }
    let mut out = format!(
        "{}  {}\ncommands {}  refusals {}  disagreements {}  {rate}",
        mode_name(&w.mode),
        clock(w.party_clock().elapsed),
        session.binder.log.len(),
        session.binder.refusals,
        session.binder.disagreements,
    );
    if fight {
        // The roll log has the fight's lines; the map is out of sight.
        text.0 = out;
        return;
    }
    out = format!(
        "{map}  cell ({}, {}) facing {:?}\n{out}\n",
        w.position.x, w.position.y, w.position.facing
    );
    if intent.looking {
        out.push_str("Mouse look on: click, Space or Esc gives the pointer back\n");
    }
    for line in &session.lines {
        out.push('\n');
        out.push_str(line);
    }
    text.0 = out;
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn the_clock_reads_days_hours_minutes() {
        assert_eq!(clock(0), "Day 1 00:00");
        assert_eq!(clock(1440 + 61), "Day 2 01:01");
    }
}
