//! `omnis`: the game. `omnis [--pack <dir>]... [--seed <n>] [--save <file>] [--autostart]
//! [--window small|medium|large|huge]`; without `--window` it opens fullscreen on the current
//! monitor, the canvas at the largest whole multiple that fits (PRD D20).
//! With feature `devtools`: `[--script <steps>] [--screenshot <file>] [--settle <frames>]
//! [--dev-socket <ip:port>] [--no-dev-socket]`; the dev socket listens on a free loopback port
//! unless disabled, and writes its address to `.omnis/dev.addr`.
#![forbid(unsafe_code)]

use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode, WindowResolution};
use omnis_app::layout::SizeClass;
use omnis_app::{
    AppConfig, assets, combat, cursor, input, inventory, menus, pixel, sheet, sim, ui, viewport,
};
use std::path::PathBuf;

#[cfg(feature = "devtools")]
type Script = omnis_app::dev::DevScript;
#[cfg(not(feature = "devtools"))]
type Script = ();
#[cfg(feature = "devtools")]
type Socket = Option<omnis_app::socket::DevSocketPlugin>;
#[cfg(not(feature = "devtools"))]
type Socket = ();

struct Launch {
    config: AppConfig,
    script: Script,
    socket: Socket,
    window: Option<SizeClass>,
}

#[cfg(feature = "devtools")]
fn default_socket() -> Socket {
    Some(omnis_app::socket::DevSocketPlugin::default())
}
#[cfg(not(feature = "devtools"))]
fn default_socket() -> Socket {}

fn parse_args() -> Result<Launch, String> {
    let mut config = AppConfig {
        packs: Vec::new(),
        ..AppConfig::default()
    };
    #[allow(unused_mut, clippy::let_unit_value)]
    let mut script = Script::default();
    #[allow(unused_mut, clippy::let_unit_value)]
    let mut socket = default_socket();
    let mut seeded = false;
    let mut window = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pack" => config.packs.push(PathBuf::from(
                args.next().ok_or("--pack needs a directory")?,
            )),
            "--seed" => {
                let value = args.next().ok_or("--seed needs a number")?;
                config.seed = value.parse().map_err(|_| format!("bad seed '{value}'"))?;
                seeded = true;
            }
            "--save" => config.save_path = PathBuf::from(args.next().ok_or("--save needs a file")?),
            "--autostart" => config.autostart = true,
            "--window" => {
                let value = args
                    .next()
                    .ok_or("--window needs small|medium|large|huge")?;
                window = Some(SizeClass::parse(&value).ok_or_else(|| {
                    format!("bad window size '{value}': small|medium|large|huge")
                })?);
            }
            #[cfg(feature = "devtools")]
            "--script" => {
                script.commands =
                    omnis_app::dev::parse_script(&args.next().ok_or("--script needs steps")?)?;
            }
            #[cfg(feature = "devtools")]
            "--screenshot" => {
                script.screenshot = Some(PathBuf::from(
                    args.next().ok_or("--screenshot needs a file")?,
                ));
                script.settle_frames = script.settle_frames.max(30);
            }
            #[cfg(feature = "devtools")]
            "--screenshot-canvas" => {
                script.screenshot = Some(PathBuf::from(
                    args.next().ok_or("--screenshot-canvas needs a file")?,
                ));
                script.settle_frames = script.settle_frames.max(30);
                script.canvas = true;
            }
            #[cfg(feature = "devtools")]
            "--dev-socket" => {
                socket = Some(omnis_app::socket::DevSocketPlugin {
                    addr: args.next().ok_or("--dev-socket needs <ip:port>")?,
                    ..Default::default()
                });
            }
            #[cfg(feature = "devtools")]
            "--no-dev-socket" => socket = None,
            #[cfg(feature = "devtools")]
            "--settle" => {
                let value = args.next().ok_or("--settle needs a frame count")?;
                script.settle_frames = value
                    .parse()
                    .map_err(|_| format!("bad frame count '{value}'"))?;
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if config.packs.is_empty() {
        config.packs = AppConfig::default().packs;
    }
    if !seeded {
        config.seed = omnis_app::entropy_seed();
    }
    // A seed, a script, or a capture means an unattended run: skip the menus, and stay in a
    // small window rather than take the screen.
    let unattended = seeded || script_given(&script);
    config.autostart |= unattended;
    if unattended && window.is_none() {
        window = Some(SizeClass::Small);
    }
    Ok(Launch {
        config,
        script,
        socket,
        window,
    })
}

#[cfg(feature = "devtools")]
fn script_given(script: &Script) -> bool {
    !script.commands.is_empty() || script.screenshot.is_some()
}
#[cfg(not(feature = "devtools"))]
fn script_given(_script: &Script) -> bool {
    false
}

fn main() -> AppExit {
    let Launch {
        config,
        script,
        socket,
        window,
    } = match parse_args() {
        Ok(launch) => launch,
        Err(e) => {
            eprintln!("omnis: {e}");
            return AppExit::error();
        }
    };
    let mut app = App::new();
    if let Some(root) = config.packs.last() {
        assets::register_pack_source(&mut app, root);
    }
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Omnis".into(),
                    mode: match window {
                        Some(_) => WindowMode::Windowed,
                        None => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    },
                    resolution: {
                        let (w, h) = window.map_or((1280, 720), SizeClass::physical);
                        WindowResolution::new(w, h)
                    },
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(config)
    .insert_resource(pixel::RequestedWindow(window))
    .add_plugins((
        sim::SimPlugin,
        input::InputPlugin,
        cursor::CursorPlugin,
        pixel::PixelPlugin,
        assets::PackAssetPlugin,
        viewport::ViewportPlugin,
        menus::MenusPlugin,
        combat::CombatPlugin,
        sheet::SheetPlugin,
        inventory::InventoryPlugin,
        ui::UiPlugin,
    ));
    #[cfg(feature = "devtools")]
    {
        app.insert_resource(script)
            .add_plugins((omnis_app::dev::DevPlugin, omnis_app::debug::DebugPlugin));
        if let Some(socket) = socket {
            app.add_plugins(socket);
        }
    }
    #[cfg(not(feature = "devtools"))]
    let ((), ()) = (script, socket);
    app.run()
}
