//! `omnis`: the game. `omnis [--pack <dir>]... [--seed <n>] [--save <file>]`.
//! With feature `devtools`: `[--script <steps>] [--screenshot <file>] [--settle <frames>]`.
#![forbid(unsafe_code)]

use bevy::prelude::*;
use bevy::window::WindowResolution;
use omnis_app::{AppConfig, assets, hud, input, pixel, sim, viewport};
use std::path::PathBuf;

#[cfg(feature = "devtools")]
type Script = omnis_app::dev::DevScript;
#[cfg(not(feature = "devtools"))]
type Script = ();

fn parse_args() -> Result<(AppConfig, Script), String> {
    let mut config = AppConfig {
        packs: Vec::new(),
        ..AppConfig::default()
    };
    #[allow(unused_mut, clippy::let_unit_value)]
    let mut script = Script::default();
    let mut seeded = false;
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
        config.seed = entropy_seed();
    }
    Ok((config, script))
}

/// A seed from the clock. The simulation never touches entropy; it only receives the number.
fn entropy_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    omnis_sim::omnis_core::splitmix64(nanos as u64 ^ (nanos >> 64) as u64)
}

fn main() -> AppExit {
    let (config, script) = match parse_args() {
        Ok(c) => c,
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
                    resolution: WindowResolution::new(1280, 720),
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(config)
    .add_plugins((
        sim::SimPlugin,
        input::InputPlugin,
        pixel::PixelPlugin,
        assets::PackAssetPlugin,
        viewport::ViewportPlugin,
        hud::HudPlugin,
    ));
    #[cfg(feature = "devtools")]
    app.insert_resource(script)
        .add_plugins(omnis_app::dev::DevPlugin);
    #[cfg(not(feature = "devtools"))]
    let () = script;
    app.run()
}
