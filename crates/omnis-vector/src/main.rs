//! The `omnis` binary (package `omnis-vector`):
//! `omnis [--pack DIR]... [--seed N] [--save FILE] [--settings FILE] [--screen title|new-game|create-party|pause|actions|verbs|inventory]`
//! `[--windowed] [--no-vsync] [--frame-stats] [--monitor N] [--log PATH] [--screenshot PATH [--walk N] [--size WxH] [--timing]]`,
//! `[--editor [MAP-ID]]` (start in the editor, M5.4),
//! and with feature `devtools` `[--dev-socket IP:PORT | --no-dev-socket]`. It starts at the
//! title, or in a game with `--seed`, a capture of the game view, or a screen shown over a game.

use bevy::app::ScheduleRunnerPlugin;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{ExitCondition, MonitorSelection, PresentMode, WindowMode, WindowResolution};
use bevy::winit::WinitPlugin;
use omnis_sim::Settings;
use omnis_vector::party;
use omnis_vector::shell::capture::{Capture, CapturePlugin, CaptureSize};
use omnis_vector::shell::session::{Packs, Session, StartScreen, parse};
use omnis_vector::shell::{
    ShellPlugin,
    cinema::CinemaPlugin,
    combat::{CombatPlugin, CombatViewPlugin},
    controls::ControlsPlugin,
    editor::{Editing, EditorPlugin},
    fonts::FontsPlugin,
    hud::HudPlugin,
    minimap::MinimapPlugin,
    movement::MovementPlugin,
    preview::PreviewPlugin,
    render::RenderPlugin,
    screen::ScreenPlugin,
    settings::{Current, SettingsPlugin},
};
use omnis_vector::sim::game_settings;
use std::time::Duration;

fn main() -> AppExit {
    let config = match parse(std::env::args().skip(1)) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("omnis: {e}");
            return AppExit::error();
        }
    };
    let packs = match Packs::load(&config) {
        Ok(packs) => packs,
        Err(e) => {
            eprintln!("omnis: {e}");
            return AppExit::error();
        }
    };
    let session = if config.starts_in_game() {
        // Party creation starts with no one; every other start takes the fixed party.
        let drafts = if config.screen == Some(StartScreen::CreateParty) {
            Vec::new()
        } else {
            party::fixed()
        };
        let settings = game_settings(Settings::default());
        match Session::with_party(&packs, config.seed, settings, &drafts) {
            Ok(session) => Some(session),
            Err(e) => {
                eprintln!("omnis: {e}");
                return AppExit::error();
            }
        }
    } else {
        None
    };
    let mut app = App::new();
    if let Some(path) = config.screenshot.clone() {
        // Offscreen: no window, a fixed-rate loop, the camera rendering into an image.
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .disable::<WinitPlugin>(),
        )
        // Timed, the loop runs as fast as the frames are made.
        .add_plugins(ScheduleRunnerPlugin::run_loop(if config.timing {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(1.0 / 60.0)
        }))
        .insert_resource(CaptureSize(config.size.0, config.size.1));
        let capture = Capture::new(path, config.walk);
        app.insert_resource(if config.timing {
            capture.timed()
        } else {
            capture
        });
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Omnis".into(),
                mode: if config.windowed {
                    WindowMode::Windowed
                } else {
                    WindowMode::BorderlessFullscreen(
                        config
                            .monitor
                            .map_or(MonitorSelection::Current, MonitorSelection::Index),
                    )
                },
                resolution: WindowResolution::new(1600, 900),
                present_mode: if config.vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                ..default()
            }),
            ..default()
        }));
    }
    // The dev socket serves a window; the offscreen capture runs alone.
    #[cfg(feature = "devtools")]
    if let (Some(addr), None) = (config.dev_socket.clone(), &config.screenshot) {
        app.add_plugins(omnis_vector::shell::socket::DevSocketPlugin { addr, ..default() });
    }
    if let Some(session) = session {
        app.insert_resource(session);
    }
    if let Some(map) = &config.editor {
        match open_editor(&packs, map.as_deref()) {
            Ok(editing) => {
                app.insert_resource(editing);
            }
            Err(e) => {
                eprintln!("omnis: {e}");
                return AppExit::error();
            }
        }
    }
    let (prefs, note) = omnis_vector::settings::read(std::path::Path::new("."), &config.settings);
    if let Some(note) = note {
        eprintln!("omnis: {note}; the defaults are used");
    }
    app.insert_resource(Current(prefs));
    if let Some(screen) = config.screen {
        app.insert_resource(screen);
    }
    if config.frame_stats {
        app.add_plugins(LogDiagnosticsPlugin::default());
    }
    app.insert_resource(packs).add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        ShellPlugin,
        MovementPlugin,
        RenderPlugin,
        HudPlugin,
        ControlsPlugin,
        ScreenPlugin,
        SettingsPlugin,
        FontsPlugin,
        MinimapPlugin,
        CinemaPlugin,
        CombatPlugin,
        CombatViewPlugin,
        CapturePlugin,
    ));
    // The editor and its preview (M5.4, M5.7); a tuple of plugins holds fifteen at most.
    app.add_plugins((EditorPlugin, PreviewPlugin));
    app.run()
}

/// The editor over the loaded packs, with `map` open when one is named.
fn open_editor(packs: &Packs, map: Option<&str>) -> Result<Editing, String> {
    let mut editor = omnis_vector::editor::Editor::new(packs.config.packs.clone())
        .map_err(|e| format!("the editor cannot open the pack: {e}"))?;
    if let Some(id) = map {
        editor
            .open(id, &packs.data)
            .map_err(|e| format!("the editor cannot open {id}: {e}"))?;
    }
    Ok(Editing(editor))
}
