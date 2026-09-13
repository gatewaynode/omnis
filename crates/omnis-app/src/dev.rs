//! `DevPlugin` (feature `devtools`): scripted commands and a screenshot from the command line,
//! so a build can be checked without a person at the keyboard. The dev socket and MCP tools
//! of M2 grow from here.

use crate::menu::{Catalog, CreationForm};
use crate::sim::{PackData, PlayState, PlayerCommand, ShellCommand, SimSet, SimWorld};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_rules::Draft;
use omnis_sim::{Command, PartyCommand};
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
    /// Add a stock member to the party, so a scripted run has someone to fight with.
    Party,
}

/// Parse a comma-separated script: the simulation's command words (`Command::from_word`:
/// `forward`, `back`, `left`, `right`, `turn-left`, `turn-right`, `around`, `use`, and the
/// fight words `fight`, `bribe`, `hide`, `run`, `attack`, `attack-N`, `dodge`, `swap-N`,
/// `flee`) plus the shell words `map`, `save`, `load`, and `party` for a stock member.
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
                "party" => ScriptStep::Party,
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
                drive.in_set(SimSet::Collect).run_if(
                    in_state(PlayState::Explore)
                        .or_else(in_state(PlayState::Encounter))
                        .or_else(in_state(PlayState::Combat)),
                ),
            );
    }
}

/// A stock member: a human fighter when the packs have one (else the catalog's first race
/// and class), the first background, the standard array, and the first skills the class
/// allows that the background does not already grant, named `Scout <n>`.
#[must_use]
pub fn recruit(data: &Data, index: usize) -> Draft {
    let catalog = Catalog::from_data(data);
    let mut form = CreationForm::new(&catalog);
    form.name = format!("Scout {}", index + 1);
    form.race = catalog
        .races
        .iter()
        .position(|r| r == "base:race:human")
        .unwrap_or(0);
    form.class = catalog
        .classes
        .iter()
        .position(|c| c == "base:class:fighter")
        .unwrap_or(0);
    form.scores = [15, 14, 13, 12, 10, 8];
    let granted = data
        .registry
        .backgrounds
        .get(&form.draft(&catalog).background)
        .and_then(|id| data.backgrounds.get(&id))
        .map_or_else(Vec::new, |b| b.skills.clone());
    let (choose, skills) = form.skill_list(&catalog);
    form.skills = skills
        .iter()
        .copied()
        .filter(|s| !granted.contains(s))
        .take(usize::from(choose))
        .collect();
    form.draft(&catalog)
}

#[allow(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    script: Res<DevScript>,
    mut progress: ResMut<Progress>,
    data: Res<PackData>,
    world: Res<SimWorld>,
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
            ScriptStep::Party => {
                let draft = recruit(&data.0, world.0.party.members.len());
                play.write(PlayerCommand(Command::Party(PartyCommand::Create(draft))));
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
        let steps = parse_script("forward, turn-left,use,map,party,fight,attack-1").unwrap();
        assert_eq!(steps.len(), 7);
        assert_eq!(steps[1], ScriptStep::Play(Command::Turn(Rotation::Left)));
        assert_eq!(steps[3], ScriptStep::Shell(ShellCommand::ToggleAutomap));
        assert_eq!(steps[4], ScriptStep::Party);
        assert!(matches!(steps[6], ScriptStep::Play(Command::Combat(_))));
        assert!(parse_script("fly").is_err());
        assert!(parse_script("").unwrap().is_empty());
    }

    #[test]
    fn a_recruit_is_a_member_the_rules_accept() {
        use omnis_sim::omnis_data::load_packs;
        use omnis_sim::{Command, Settings, World, apply};
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")])
            .unwrap_or_else(|r| panic!("{r}"));
        let mut world = World::new(&data, 1, Settings::default()).unwrap();
        for i in 0..2 {
            let draft = recruit(&data, world.party.members.len());
            assert_eq!(draft.name, format!("Scout {}", i + 1));
            assert_eq!(draft.class, "base:class:fighter");
            apply(
                &mut world,
                &data,
                Command::Party(PartyCommand::Create(draft)),
            )
            .unwrap_or_else(|r| panic!("{r}"));
        }
        assert_eq!(world.party.members.len(), 2);
    }
}
