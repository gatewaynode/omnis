//! `SimPlugin`: owns the `World`, applies commands in one ordered set, publishes events as
//! messages (ARCHITECTURE.md §8.1). Runs headless.

use crate::AppConfig;
use crate::confirm_panel::Ahead;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use omnis_sim::api::{
    CastView, CombatView, Here, ModeKind, PartyView, RestView, ServiceView, cast_view, combat_view,
    flags, here, party_view, rest_view, service_view,
};
use omnis_sim::api::{Direction, site_ahead, step_lands};
use omnis_sim::omnis_data::ron_io::read_text;
use omnis_sim::omnis_data::{Data, load_packs};
use omnis_sim::{Command, Event, Rejection, Settings, World, apply};
use std::path::{Path, PathBuf};

/// Top-level app state (ARCHITECTURE.md §8.1).
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Loading packs.
    #[default]
    Boot,
    /// The title and new game screens; no world exists.
    MainMenu,
    /// A game is running.
    Playing,
}

/// Which menu screen is up.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::MainMenu)]
pub enum MenuState {
    /// New game, load, quit.
    #[default]
    Title,
    /// Seed and difficulty.
    NewGame,
}

/// What the player is doing while playing.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Playing)]
pub enum PlayState {
    /// Building the party before the first step.
    #[default]
    CreateParty,
    /// Walking the map.
    Explore,
    /// Monsters ahead: attack, bribe, hide, or run.
    Encounter,
    /// The fight.
    Combat,
    /// The pause overlay.
    Paused,
    /// Every member is down: load or quit.
    Defeat,
    /// The debug menu over the world; commands still apply (feature `devtools`).
    Debug,
    /// The cast menu while exploring.
    Cast,
    /// The character sheet over the world.
    Sheet,
    /// The inventory overlay over the world; item commands apply from it.
    Inventory,
    /// A step into or out of a town service waits for Go or Stay (`input::AskFirst`).
    Confirm,
    /// Inside a town service: its panel (`feathers_service.rs`).
    Service,
    /// The camp over the map: rests outside a service (`feathers_camp.rs`).
    Camp,
    /// A member's declared reactions, from the sheet (`feathers_tactics.rs`).
    Tactics,
}

/// The settings a new game starts with: the player's choices, and `devtools` when this build
/// carries the dev socket, so the debug menu and `Dev` commands work in a dev build and never
/// in a release.
#[must_use]
pub fn game_settings(base: Settings) -> Settings {
    Settings {
        devtools: cfg!(feature = "devtools"),
        ..base
    }
}

impl PlayState {
    /// The play state the world's mode calls for.
    #[must_use]
    pub const fn for_kind(mode: ModeKind) -> PlayState {
        match mode {
            ModeKind::Explore => PlayState::Explore,
            ModeKind::Town => PlayState::Service,
            ModeKind::Encounter => PlayState::Encounter,
            ModeKind::Combat => PlayState::Combat,
        }
    }
}

/// Whether a game is running and not paused: commands apply in every other play state.
#[must_use]
pub fn unpaused(state: Option<Res<State<PlayState>>>) -> bool {
    state.is_some_and(|s| *s.get() != PlayState::Paused)
}

/// Where a newly started game begins; read once on entering `Playing`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartIn(pub PlayState);

/// The loaded packs.
#[derive(Resource)]
pub struct PackData(pub Data);

/// The game state. Only `SimPlugin` systems mutate it.
#[derive(Resource)]
pub struct SimWorld(pub World);

impl SimWorld {
    /// What a step in `direction` would meet: whether it lands anywhere, and the service it
    /// would go into.
    #[must_use]
    pub fn ahead(&self, data: &Data, direction: Direction) -> Ahead {
        Ahead {
            lands: step_lands(&self.0, data, direction).is_some(),
            site: site_ahead(&self.0, data, direction),
        }
    }
}

/// What the app reads of the world: the engine's views (ARCHITECTURE.md §4.9), refreshed at the
/// end of [`SimSet::Apply`] whenever the world or the packs changed. Presentation reads this,
/// never the world's fields.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Views {
    /// Where the party is and what it is doing.
    pub here: Here,
    /// The party and every member's sheet.
    pub party: PartyView,
    /// The encounter or fight, if one is on.
    pub combat: Option<CombatView>,
    /// The service the party is inside, if any.
    pub service: Option<ServiceView>,
    /// The rests on offer.
    pub rest: RestView,
    /// The spells castable outside a fight.
    pub casts: Vec<CastView>,
    /// Every flag with its value, when the game carries dev tools; empty otherwise.
    pub flags: Vec<(String, i64)>,
}

impl Views {
    /// Every view of `world`, read through the engine's API only.
    #[must_use]
    pub fn of(world: &World, data: &Data) -> Views {
        let here = here(world, data);
        let flags = if here.settings.devtools {
            flags(world, data)
        } else {
            Vec::new()
        };
        Views {
            here,
            party: party_view(world, data),
            combat: combat_view(world, data),
            service: service_view(world, data),
            rest: rest_view(world, data),
            casts: cast_view(world, data),
            flags,
        }
    }
}

/// Keep [`Views`] with the world: read again when the world or the packs changed or arrived,
/// gone when the world is. Every change of the world marks the views changed.
fn refresh_views(
    mut commands: Commands,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    views: Option<ResMut<Views>>,
) {
    let (Some(world), Some(data)) = (world, data) else {
        if views.is_some() {
            commands.remove_resource::<Views>();
        }
        return;
    };
    match views {
        Some(mut views) if world.is_changed() || data.is_changed() => {
            *views = Views::of(&world.0, &data.0);
        }
        Some(_) => {}
        None => commands.insert_resource(Views::of(&world.0, &data.0)),
    }
}

/// End the game: the world and its views go together.
pub fn close_world(commands: &mut Commands) {
    commands.remove_resource::<SimWorld>();
    commands.remove_resource::<Views>();
}

/// A player action for the simulation.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct PlayerCommand(pub Command);

/// Something outside the simulation: saving, loading, overlays, pausing, quitting.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    /// Write the quick save (the pause menu's Save).
    Save,
    /// Read the quick save (the pause menu's Load).
    Load,
    /// Show or hide the automap.
    ToggleAutomap,
    /// Open the pause overlay.
    Pause,
    /// Open the cast menu while exploring.
    Cast,
    /// Open the character sheet.
    Sheet,
    /// Open the inventory overlay while exploring.
    Inventory,
    /// Look through the first sense item a member carries; a notice when there is none.
    Look,
    /// Open the camp while exploring.
    Camp,
    /// Exit the application.
    Quit,
}

/// One simulation event, re-published one to one.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SimEvent(pub Event);

/// A command the rules refused, for the screen that sent it.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct CommandRefused(pub Rejection);

/// The whole world changed (a load); presentation redraws everything.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldReplaced;

/// A line for the player that is not a simulation event: saved, loaded, errors.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct Notice(pub String);

/// The ordered sets of the frame: collect commands, apply them, publish for presentation.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Input and shells turn into messages.
    Collect,
    /// Commands hit the world.
    Apply,
    /// Presentation reads the results.
    Publish,
}

/// The simulation plugin.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<MenuState>()
            .add_sub_state::<PlayState>()
            .add_message::<PlayerCommand>()
            .add_message::<ShellCommand>()
            .add_message::<SimEvent>()
            .add_message::<CommandRefused>()
            .add_message::<WorldReplaced>()
            .init_resource::<Notice>()
            .configure_sets(
                Update,
                (SimSet::Collect, SimSet::Apply, SimSet::Publish).chain(),
            )
            .add_systems(Startup, boot)
            .add_systems(OnEnter(AppState::Playing), start_in)
            .add_systems(
                Update,
                // The pause stops simulation commands; the shell keeps working, since the pause
                // menu's Save and Load are shell commands. The world may leave mid-frame (quit
                // to title): both skip until the state follows.
                (apply_commands.run_if(unpaused), shell)
                    .chain()
                    .in_set(SimSet::Apply)
                    .run_if(resource_exists::<SimWorld>),
            )
            .add_systems(Update, refresh_views.in_set(SimSet::Apply).after(shell));
    }
}

/// Load the packs and start a new game, or report why not and exit.
fn boot(
    mut commands: Commands,
    config: Res<AppConfig>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let roots: Vec<&Path> = config.packs.iter().map(PathBuf::as_path).collect();
    let data = match load_packs(&roots) {
        Ok(data) => data,
        Err(report) => {
            error!("packs refused:\n{report}");
            exit.write(AppExit::error());
            return;
        }
    };
    if !config.autostart {
        info!("{} pack(s) loaded; main menu", data.packs.len());
        commands.insert_resource(PackData(data));
        next.set(AppState::MainMenu);
        return;
    }
    match World::new(&data, config.seed, game_settings(Settings::default())) {
        Ok(world) => {
            info!(
                "new game on {} pack(s), seed {:#x}",
                data.packs.len(),
                config.seed
            );
            commands.insert_resource(SimWorld(world));
            commands.insert_resource(PackData(data));
            commands.insert_resource(StartIn(PlayState::Explore));
            next.set(AppState::Playing);
        }
        Err(e) => {
            error!("{e}");
            exit.write(AppExit::error());
        }
    }
}

/// Enter the play state the starter asked for; the sub-state's default is party creation.
fn start_in(
    mut commands: Commands,
    start: Option<Res<StartIn>>,
    mut next: ResMut<NextState<PlayState>>,
) {
    if let Some(start) = start {
        next.set(start.0);
        commands.remove_resource::<StartIn>();
    }
}

fn apply_commands(
    mut incoming: MessageReader<PlayerCommand>,
    mut world: ResMut<SimWorld>,
    data: Res<PackData>,
    mut events: MessageWriter<SimEvent>,
    mut refused: MessageWriter<CommandRefused>,
) {
    for PlayerCommand(command) in incoming.read() {
        match apply(&mut world.0, &data.0, command.clone()) {
            Ok(produced) => {
                for event in produced {
                    events.write(SimEvent(event));
                }
            }
            Err(rejection) => {
                info!("{command:?} refused: {rejection}");
                refused.write(CommandRefused(rejection));
            }
        }
    }
}

/// What the shell writes: the notice line, the world-replaced flag, exit, and the pause.
#[derive(SystemParam)]
struct ShellOut<'w> {
    notice: ResMut<'w, Notice>,
    player: MessageWriter<'w, PlayerCommand>,
    replaced: MessageWriter<'w, WorldReplaced>,
    exit: MessageWriter<'w, AppExit>,
    next_play: ResMut<'w, NextState<PlayState>>,
}

fn shell(
    mut incoming: MessageReader<ShellCommand>,
    mut world: ResMut<SimWorld>,
    data: Res<PackData>,
    config: Res<AppConfig>,
    mut out: ShellOut,
) {
    for command in incoming.read() {
        match command {
            ShellCommand::Save if !world.0.may_save() => {
                out.notice.0 = "The save rule forbids saving here".to_owned();
            }
            ShellCommand::Save => match save(&world.0, &config.save_path) {
                Ok(()) => out.notice.0 = format!("Saved to {}", config.save_path.display()),
                Err(e) => out.notice.0 = format!("Save failed: {e}"),
            },
            ShellCommand::Load => match load(&data.0, &config.save_path, false) {
                Ok(loaded) => {
                    world.0 = loaded;
                    out.replaced.write(WorldReplaced);
                    out.notice.0 = format!("Loaded {}", config.save_path.display());
                }
                Err(e) => out.notice.0 = format!("Load failed: {e}"),
            },
            ShellCommand::Pause => out.next_play.set(PlayState::Paused),
            ShellCommand::Cast => out.next_play.set(PlayState::Cast),
            ShellCommand::Sheet => out.next_play.set(PlayState::Sheet),
            ShellCommand::Inventory => out.next_play.set(PlayState::Inventory),
            ShellCommand::Camp if here(&world.0, &data.0).mode == ModeKind::Explore => {
                out.next_play.set(PlayState::Camp);
            }
            ShellCommand::Camp => out.notice.0 = "No camp here".to_owned(),
            ShellCommand::Look => match crate::look::look_command(&world.0, &data.0) {
                Some(command) => {
                    out.player.write(PlayerCommand(command));
                }
                None => out.notice.0 = "Nothing to look through".to_owned(),
            },
            ShellCommand::Quit => {
                out.exit.write(AppExit::Success);
            }
            ShellCommand::ToggleAutomap => {}
        }
    }
}

/// Write the world as RON to `path`, creating the directory.
pub fn save(world: &World, path: &Path) -> Result<(), String> {
    write_text(path, &world.to_ron().map_err(|e| e.to_string())?)
}

/// Write `text` to `path`, making its directory.
pub fn write_text(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Read a world from `path`, checked against the loaded packs unless `force`. The file is read
/// with the pack loader's limits (no symlink, at most `MAX_FILE_BYTES`), as the headless host
/// reads it: a save is untrusted input (B4).
pub fn load(data: &Data, path: &Path, force: bool) -> Result<World, String> {
    let text = read_text(path, path).map_err(|e| e.to_string())?;
    omnis_sim::ops::load_text(&text, data, force).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_core::Rotation;

    #[test]
    fn the_views_arrive_with_the_world_follow_it_and_leave_with_it() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")]).unwrap();
        let world = World::new(&data, 7, Settings::default()).unwrap();
        let mut app = App::new();
        app.add_systems(Update, refresh_views);
        app.update();
        assert!(
            app.world().get_resource::<Views>().is_none(),
            "no world, no views"
        );
        app.insert_resource(PackData(data.clone()))
            .insert_resource(SimWorld(world.clone()));
        app.update();
        assert_eq!(*app.world().resource::<Views>(), Views::of(&world, &data));
        let turned = {
            let mut sim = app.world_mut().resource_mut::<SimWorld>();
            apply(&mut sim.0, &data, Command::Turn(Rotation::Right)).unwrap();
            sim.0.clone()
        };
        app.update();
        let views = app.world().resource::<Views>();
        assert_eq!(*views, Views::of(&turned, &data));
        assert_ne!(views.here.position, Views::of(&world, &data).here.position);
        app.world_mut().remove_resource::<SimWorld>();
        app.update();
        assert!(
            app.world().get_resource::<Views>().is_none(),
            "the views leave with the world"
        );
    }

    #[test]
    fn the_play_state_follows_the_mode_and_only_the_pause_stops_commands() {
        assert_eq!(PlayState::for_kind(ModeKind::Explore), PlayState::Explore);
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .add_sub_state::<PlayState>();
        assert!(!app.world_mut().run_system_cached(unpaused).unwrap());
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();
        assert!(app.world_mut().run_system_cached(unpaused).unwrap());
        app.world_mut()
            .resource_mut::<NextState<PlayState>>()
            .set(PlayState::Paused);
        app.update();
        assert!(!app.world_mut().run_system_cached(unpaused).unwrap());
        app.world_mut()
            .resource_mut::<NextState<PlayState>>()
            .set(PlayState::Defeat);
        app.update();
        assert!(app.world_mut().run_system_cached(unpaused).unwrap());
    }
}
