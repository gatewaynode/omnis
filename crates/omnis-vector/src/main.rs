//! The `omnis-vector` binary:
//! `omnis-vector [--pack DIR]... [--seed N] [--windowed] [--no-vsync] [--monitor N] [--log PATH]`
//! `[--screenshot PATH [--walk N] [--size WxH]]`, and with feature `devtools`
//! `[--dev-socket IP:PORT | --no-dev-socket]`.

use bevy::app::ScheduleRunnerPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::window::{ExitCondition, MonitorSelection, PresentMode, WindowMode, WindowResolution};
use bevy::winit::WinitPlugin;
use omnis_vector::shell::capture::{Capture, CapturePlugin, CaptureSize};
use omnis_vector::shell::session::{Session, parse};
use omnis_vector::shell::{
    ShellPlugin,
    cinema::CinemaPlugin,
    combat::{CombatPlugin, CombatViewPlugin},
    controls::ControlsPlugin,
    hud::HudPlugin,
    minimap::MinimapPlugin,
    movement::MovementPlugin,
    render::RenderPlugin,
    screen::ScreenPlugin,
};
use std::time::Duration;

fn main() -> AppExit {
    let config = match parse(std::env::args().skip(1)) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("omnis-vector: {e}");
            return AppExit::error();
        }
    };
    let session = match Session::start(&config) {
        Ok(session) => session,
        Err(e) => {
            eprintln!("omnis-vector: {e}");
            return AppExit::error();
        }
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
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(CaptureSize(config.size.0, config.size.1))
        .insert_resource(Capture::new(path, config.walk));
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Omnis Vector".into(),
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
    app.insert_resource(session).add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        ShellPlugin,
        MovementPlugin,
        RenderPlugin,
        HudPlugin,
        ControlsPlugin,
        ScreenPlugin,
        MinimapPlugin,
        CinemaPlugin,
        CombatPlugin,
        CombatViewPlugin,
        CapturePlugin,
    ));
    app.run()
}
