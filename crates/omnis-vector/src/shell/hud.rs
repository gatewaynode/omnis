//! The HUD: a status block and, under it, the recent event lines (presentation-ARCHITECTURE.md
//! §10.1). The log is its own column (owner, 2026-10-17): no wider than the gap left of the
//! outcome notice, so a long line wraps rather than run under it, in the interface's dim role,
//! each older line fainter, so the notice stands out.

use super::controls::GREEN;
use super::movement::Intent;
use super::notice::NOTICE_WIDTH;
use super::screen::{Screens, colour};
use super::session::Session;
use super::{VectorSet, ViewState};
use crate::screen::ScreenId;
use crate::style::Role;
use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use omnis_sim::ModeKind;
use omnis_sim::api::Here;
use omnis_sim::omnis_data::Data;

/// Marks the status text.
#[derive(Component)]
pub struct StatusText;

/// Marks the HUD's node: the status text and the log under it.
#[derive(Component)]
pub struct Hud;

/// Marks the log's column; a click on it opens the full log.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogColumn;

/// One line of the log, by its place from the top.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLine(pub usize);

/// Lines the log shows, as many as the session keeps.
pub const LOG_LINES: usize = 8;

/// The log column's widest, in percent of the window's width: up to the widest notice's left
/// edge, less the HUD's 16 px margin (2 vw from 800 px wide up).
#[must_use]
pub fn log_width() -> f32 {
    (100.0 - NOTICE_WIDTH) / 2.0 - 2.0
}

/// How opaque a log line `age` lines older than the newest is: whole, then a tenth less each
/// line, so the oldest of [`LOG_LINES`] is at 0.3.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn log_alpha(age: usize) -> f32 {
    1.0 - 0.1 * age as f32
}

/// The status block, top left; in a fight, its first lines only, at the bottom left under the
/// action column (`arena::Layout::status`).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn).add_systems(
            Update,
            (arrange, status, log, open_log).in_set(VectorSet::Draw),
        );
    }
}

fn spawn(mut commands: Commands) {
    let font = || TextFont {
        font_size: FontSize::Px(18.0),
        ..default()
    };
    commands
        .spawn((
            Hud,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|hud| {
            hud.spawn((StatusText, Text::new(""), font(), TextColor(GREEN)));
            hud.spawn((
                LogColumn,
                Node {
                    flex_direction: FlexDirection::Column,
                    max_width: Val::Vw(log_width()),
                    margin: UiRect::top(Val::Px(18.0)),
                    ..default()
                },
                // A click on it opens the full log.
                Interaction::default(),
            ))
            .with_children(|column| {
                for at in 0..LOG_LINES {
                    column.spawn((
                        LogLine(at),
                        Text::new(""),
                        font(),
                        TextColor(Color::NONE),
                        Visibility::Hidden,
                    ));
                }
            });
        });
}

/// Where the party is, for the status block: the map, its level when the map has several
/// (M8.5 step 1b), the cell and the facing.
#[must_use]
pub fn place(data: &Data, here: &Here) -> String {
    let p = &here.position;
    let level = data
        .registry
        .maps
        .get(&p.map)
        .and_then(|id| data.maps.get(&id))
        .and_then(|map| crate::labels::level(data, map, p.level))
        .map_or_else(String::new, |l| format!("{l}  "));
    format!(
        "{}  {level}cell ({}, {}) facing {:?}",
        p.map, p.x, p.y, p.facing
    )
}

fn mode_name(mode: ModeKind) -> &'static str {
    match mode {
        ModeKind::Explore => "Exploring",
        ModeKind::Town => "In town",
        ModeKind::Encounter => "Encounter",
        ModeKind::Combat => "Combat",
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
    mut text: Query<&mut Text, With<StatusText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let fight = view.is_some_and(|v| *v.get() == ViewState::Fight);
    let here = &session.views.here;
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
        mode_name(here.mode),
        crate::labels::clock(here.age),
        session.binder.log.len(),
        session.binder.refusals,
        session.binder.disagreements,
    );
    if fight {
        // The map is out of sight.
        text.0 = out;
        return;
    }
    out = format!("{}\n{out}", place(&session.data, here));
    if intent.looking {
        out.push_str("\nMouse look on: click, Space or Esc gives the pointer back");
    }
    text.0 = out;
}

/// The HUD at the top left while exploring; in a fight at the bottom left, its status smaller, to
/// fit under the action column.
fn arrange(
    view: Option<Res<State<ViewState>>>,
    mut hud: Query<&mut Node, With<Hud>>,
    mut font: Query<&mut TextFont, With<StatusText>>,
) {
    let (Ok(mut node), Ok(mut font)) = (hud.single_mut(), font.single_mut()) else {
        return;
    };
    let fight = view.is_some_and(|v| *v.get() == ViewState::Fight);
    let (top, bottom) = if fight {
        (Val::Auto, Val::Px(16.0))
    } else {
        (Val::Px(16.0), Val::Auto)
    };
    if node.top != top {
        node.top = top;
        node.bottom = bottom;
        font.font_size = FontSize::Px(if fight { 14.0 } else { 18.0 });
    }
}

/// The running log's lines: the session's newest, oldest first, each older one fainter; none in
/// a fight, whose roll log has its lines.
fn log(
    session: Res<Session>,
    screens: Res<Screens>,
    view: Option<Res<State<ViewState>>>,
    mut log: Query<(&LogLine, &mut Text, &mut TextColor, &mut Visibility)>,
) {
    let fight = view.is_some_and(|v| *v.get() == ViewState::Fight);
    // The full log shows them all; the running log does not stand beside it.
    let full = screens.nav.top(Some(&session.views)) == Some(ScreenId::Log);
    let lines: &[String] = if fight || full { &[] } else { &session.lines };
    let first = lines.len().saturating_sub(LOG_LINES);
    let lines = &lines[first..];
    for (at, mut words, mut tint, mut shown) in &mut log {
        let want = match lines.get(at.0) {
            Some(line) => {
                let age = lines.len() - 1 - at.0;
                if words.0 != *line {
                    words.0.clone_from(line);
                }
                tint.0 = colour(Role::Dim).with_alpha(log_alpha(age));
                Visibility::Inherited
            }
            None => Visibility::Hidden,
        };
        if *shown != want {
            *shown = want;
        }
    }
}

/// A click on the running log opens the full log, where it is allowed.
fn open_log(
    clicked: Query<&Interaction, (Changed<Interaction>, With<LogColumn>)>,
    session: Option<Res<Session>>,
    mut screens: ResMut<Screens>,
) {
    if let Some(session) = session
        && clicked.iter().any(|i| *i == Interaction::Pressed)
        && screens.nav.allowed(ScreenId::Log, Some(&session.views))
    {
        screens.nav.open(ScreenId::Log, Some(&session.views));
    }
}
