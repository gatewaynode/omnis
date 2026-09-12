//! `DevPlugin` (feature `devtools`): scripted commands and a screenshot from the command line,
//! so a build can be checked without a person at the keyboard. The dev socket and MCP tools
//! of M2 grow from here.

use crate::sim::{PlayState, PlayerCommand, ShellCommand, SimSet};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use omnis_sim::Command;
use std::path::PathBuf;

/// What to do unattended.
#[derive(Resource, Debug, Clone, Default)]
pub struct DevScript {
    /// Commands to issue one per frame once playing.
    pub commands: Vec<ScriptStep>,
    /// Save a screenshot of the window here, then exit.
    pub screenshot: Option<PathBuf>,
    /// Frames to wait after the last command before the screenshot.
    pub settle_frames: u32,
    /// Capture the 320×180 canvas instead of the window.
    pub canvas: bool,
}

/// One scripted action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptStep {
    /// A simulation command.
    Play(Command),
    /// A shell action.
    Shell(ShellCommand),
}

/// Parse a comma-separated script: the simulation's command words (`Command::from_word`:
/// `forward`, `back`, `left`, `right`, `turn-left`, `turn-right`, `around`, `use`) plus the
/// shell words `map`, `save`, `load`.
pub fn parse_script(text: &str) -> Result<Vec<ScriptStep>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|word| {
            if let Some(command) = Command::from_word(word) {
                return Ok(ScriptStep::Play(command));
            }
            Ok(match word {
                "map" => ScriptStep::Shell(ShellCommand::ToggleAutomap),
                "save" => ScriptStep::Shell(ShellCommand::Save),
                "load" => ScriptStep::Shell(ShellCommand::Load),
                other => return Err(format!("unknown script step '{other}'")),
            })
        })
        .collect()
}

#[derive(Resource, Default)]
struct Progress {
    next: usize,
    settled: u32,
    shot: bool,
}

/// The dev plugin.
pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DevScript>()
            .init_resource::<Progress>()
            .add_systems(
                Update,
                drive
                    .in_set(SimSet::Collect)
                    .run_if(in_state(PlayState::Explore)),
            );
    }
}

fn drive(
    mut commands: Commands,
    script: Res<DevScript>,
    mut progress: ResMut<Progress>,
    mut play: MessageWriter<PlayerCommand>,
    mut shell: MessageWriter<ShellCommand>,
    mut exit: MessageWriter<AppExit>,
    canvas: Option<Res<crate::pixel::CanvasImage>>,
) {
    if let Some(step) = script.commands.get(progress.next) {
        match step {
            ScriptStep::Play(c) => {
                play.write(PlayerCommand(c.clone()));
            }
            ScriptStep::Shell(s) => {
                shell.write(*s);
            }
        }
        progress.next += 1;
        return;
    }
    let Some(path) = &script.screenshot else {
        return;
    };
    progress.settled += 1;
    if progress.settled == script.settle_frames && !progress.shot {
        progress.shot = true;
        info!("saving screenshot to {}", path.display());
        let target = match (&canvas, script.canvas) {
            (Some(canvas), true) => Screenshot::image(canvas.0.clone()),
            _ => Screenshot::primary_window(),
        };
        commands.spawn(target).observe(save_to_disk(path.clone()));
    }
    if progress.settled > script.settle_frames + 30 {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_core::Rotation;

    #[test]
    fn scripts_parse() {
        let steps = parse_script("forward, turn-left,use,map").unwrap();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[1], ScriptStep::Play(Command::Turn(Rotation::Left)));
        assert_eq!(steps[3], ScriptStep::Shell(ShellCommand::ToggleAutomap));
        assert!(parse_script("fly").is_err());
        assert!(parse_script("").unwrap().is_empty());
    }
}
