//! `omnis`: the game. `omnis [--pack <dir>]... [--seed <n>] [--save <file>] [--autostart] [--window <w>x<h>]`.
//! With feature `devtools`: `[--script <steps>] [--screenshot <file>] [--settle <frames>]
//! [--dev-socket <ip:port>] [--no-dev-socket]`; the dev socket listens on a free loopback port
//! unless disabled, and writes its address to `.omnis/dev.addr`.
#![forbid(unsafe_code)]

use bevy::prelude::*;
use bevy::window::WindowResolution;
use omnis_app::{AppConfig, assets, cursor, input, menus, pixel, sim, ui, viewport};
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
    window: (u32, u32),
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
    let mut window = (1280, 720);
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
                let value = args.next().ok_or("--window needs <width>x<height>")?;
                let (w, h) = value
                    .split_once('x')
                    .ok_or_else(|| format!("bad window size '{value}'"))?;
                window = (
                    w.parse().map_err(|_| format!("bad window width '{w}'"))?,
                    h.parse().map_err(|_| format!("bad window height '{h}'"))?,
                );
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
    // A seed, a script, or a capture means an unattended run: skip the menus.
    config.autostart |= seeded || script_given(&script);
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
                    resolution: WindowResolution::new(window.0, window.1),
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(config)
    .add_plugins((
        sim::SimPlugin,
        input::InputPlugin,
        cursor::CursorPlugin,
        pixel::PixelPlugin,
        assets::PackAssetPlugin,
        viewport::ViewportPlugin,
        menus::MenusPlugin,
        ui::UiPlugin,
    ));
    #[cfg(feature = "devtools")]
    {
        app.insert_resource(script)
            .add_plugins(omnis_app::dev::DevPlugin);
        if let Some(socket) = socket {
            app.add_plugins(socket);
        }
    }
    #[cfg(not(feature = "devtools"))]
    let ((), ()) = (script, socket);
    app.run()
}
