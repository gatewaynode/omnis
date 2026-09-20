//! `MenusPlugin`: the title, new game, character creation, and pause screens, driven by the
//! state machines in `menu.rs` and painted by `screens.rs` through `ui.rs`. Keys arrive as
//! logical `KeyboardInput` so names and seeds can be typed; clicks on the composed frame's
//! widgets arrive as `UiClick`s and become the same keys.

use crate::AppConfig;
use crate::combat_menu::{CombatMenu, DefeatMenu, EncounterMenu};
use crate::cursor::UiSet;
use crate::debug_menu::DebugMenu;
use crate::inventory_menu::InventoryMenu;
use crate::menu::{
    Catalog, CreationAction, CreationForm, MenuKey, NewGameAction, NewGameForm, Pause, PauseAction,
    Title, TitleAction, debug_available,
};
use crate::screen::{self, Target};
use crate::sheet_menu::SheetMenu;
use crate::sim::{
    AppState, CommandRefused, MenuState, Notice, PackData, PlayState, PlayerCommand, ShellCommand,
    SimEvent, SimWorld, StartIn, WorldReplaced, load,
};
use crate::spell_menu::{CastIntent, CastMenu, cast_rows};
use crate::ui::UiClick;
use crate::widget::Hit;
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use omnis_sim::{Command, Event, PartyCommand, World};

/// Which skin party creation wears (the Feathers experiment, PRD D26). Both skins edit the
/// same `CreationForm`. An app with the Feathers plugin wears the panel; the canvas screen is
/// what every other build shows (release, the `MinimalPlugins` tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreationSkin {
    /// The Feathers panel is showing; the canvas paints only the backdrop and takes no keys.
    pub feathers: bool,
}

/// What the Feathers panel asks of the creation flow: the same actions as the canvas keys.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct CreationAsk(pub CreationAction);

/// Every screen's state, kept so a screen reopens where it was.
#[derive(Resource, Default, Debug)]
pub struct Screens {
    /// Party creation's skin.
    pub skin: CreationSkin,
    /// The title.
    pub title: Title,
    /// The new game form.
    pub new_game: NewGameForm,
    /// The creation form.
    pub creation: CreationForm,
    /// What creation may choose from.
    pub catalog: Catalog,
    /// The pause overlay.
    pub pause: Pause,
    /// The choice before a fight.
    pub encounter: EncounterMenu,
    /// The fight.
    pub combat: CombatMenu,
    /// The modal after a wipe.
    pub defeat: DefeatMenu,
    /// The debug menu.
    pub debug: DebugMenu,
    /// The cast menu while exploring.
    pub cast: CastMenu,
    /// The character sheet.
    pub sheet: SheetMenu,
    /// The inventory overlay.
    pub inventory: InventoryMenu,
}

/// Which screen is up, if any.
#[derive(SystemParam)]
pub struct Where<'w> {
    app: Res<'w, State<AppState>>,
    menu: Option<Res<'w, State<MenuState>>>,
    play: Option<Res<'w, State<PlayState>>>,
}

/// The active screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Active {
    /// The title.
    Title,
    /// The new game form.
    NewGame,
    /// Character creation.
    CreateParty,
    /// The pause overlay.
    Paused,
    /// The choice before a fight.
    Encounter,
    /// The fight.
    Combat,
    /// The modal after a wipe.
    Defeat,
    /// The debug menu.
    Debug,
    /// The cast menu while exploring.
    Cast,
    /// The character sheet.
    Sheet,
    /// The inventory overlay.
    Inventory,
    /// No screen: booting or exploring.
    None,
}

impl Where<'_> {
    /// The active screen.
    #[must_use]
    pub fn screen(&self) -> Active {
        match (
            self.app.get(),
            self.menu.as_deref().map(State::get),
            self.play.as_deref().map(State::get),
        ) {
            (AppState::MainMenu, Some(MenuState::Title), _) => Active::Title,
            (AppState::MainMenu, Some(MenuState::NewGame), _) => Active::NewGame,
            (AppState::Playing, _, Some(PlayState::CreateParty)) => Active::CreateParty,
            (AppState::Playing, _, Some(PlayState::Paused)) => Active::Paused,
            (AppState::Playing, _, Some(PlayState::Encounter)) => Active::Encounter,
            (AppState::Playing, _, Some(PlayState::Combat)) => Active::Combat,
            (AppState::Playing, _, Some(PlayState::Defeat)) => Active::Defeat,
            (AppState::Playing, _, Some(PlayState::Debug)) => Active::Debug,
            (AppState::Playing, _, Some(PlayState::Cast)) => Active::Cast,
            (AppState::Playing, _, Some(PlayState::Sheet)) => Active::Sheet,
            (AppState::Playing, _, Some(PlayState::Inventory)) => Active::Inventory,
            _ => Active::None,
        }
    }

    /// Whether a game is running.
    #[must_use]
    pub fn playing(&self) -> bool {
        *self.app.get() == AppState::Playing
    }

    /// Whether the party is walking the map.
    #[must_use]
    pub fn exploring(&self) -> bool {
        *self.app.get() == AppState::Playing
            && self.play.as_deref().map(State::get) == Some(&PlayState::Explore)
    }
}

/// The menus plugin.
pub struct MenusPlugin;

impl Plugin for MenusPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<KeyboardInput>()
            .add_message::<UiClick>()
            .add_message::<CreationAsk>()
            .init_resource::<Screens>()
            .add_systems(OnEnter(PlayState::CreateParty), open_creation)
            .add_systems(Update, (menu_keys, creation_asks).in_set(UiSet::Dispatch))
            .add_systems(Update, refresh.in_set(UiSet::Model));
    }
}

fn open_creation(data: Res<PackData>, mut screens: ResMut<Screens>) {
    screens.catalog = Catalog::from_data(&data.0);
    screens.creation = CreationForm::new(&screens.catalog);
}

/// A logical key press as the menus see it.
pub(crate) fn menu_key(input: &KeyboardInput) -> Option<MenuKey> {
    if input.state != ButtonState::Pressed {
        return None;
    }
    Some(match &input.logical_key {
        Key::ArrowUp => MenuKey::Up,
        Key::ArrowDown => MenuKey::Down,
        Key::ArrowLeft => MenuKey::Left,
        Key::ArrowRight => MenuKey::Right,
        Key::Enter => MenuKey::Enter,
        Key::Escape => MenuKey::Escape,
        Key::Backspace => MenuKey::Backspace,
        Key::Space => MenuKey::Char(' '),
        Key::Tab => MenuKey::Char('\t'),
        Key::Character(text) => {
            let c = text.chars().next()?;
            if c.is_control() {
                return None;
            }
            MenuKey::Char(c)
        }
        _ => return None,
    })
}

/// The keys a click on the active screen stands for.
fn click_keys(screens: &mut Screens, active: Active, hit: Hit) -> Vec<MenuKey> {
    let target = match active {
        Active::Title => Target::Title(&mut screens.title),
        Active::NewGame => Target::NewGame(&mut screens.new_game),
        Active::CreateParty => Target::Creation(&mut screens.creation),
        Active::Paused => Target::Pause(&mut screens.pause),
        Active::Cast => Target::Cast(&mut screens.cast),
        // The combat, debug, sheet and inventory plugins handle their screens' clicks.
        Active::Encounter
        | Active::Combat
        | Active::Defeat
        | Active::Debug
        | Active::Sheet
        | Active::Inventory
        | Active::None => {
            return Vec::new();
        }
    };
    screen::click(target, hit)
}

/// State transitions the menus request.
#[derive(SystemParam)]
struct Next<'w> {
    app: ResMut<'w, NextState<AppState>>,
    menu: ResMut<'w, NextState<MenuState>>,
    play: ResMut<'w, NextState<PlayState>>,
}

/// What a menu action may touch.
struct Actions<'a, 'c, 'cs, 'n, 'p, 's, 'e, 'r> {
    commands: &'a mut Commands<'c, 'cs>,
    next: &'a mut Next<'n>,
    data: Option<&'a PackData>,
    config: &'a AppConfig,
    player: &'a mut MessageWriter<'p, PlayerCommand>,
    shell: &'a mut MessageWriter<'s, ShellCommand>,
    exit: &'a mut MessageWriter<'e, AppExit>,
    replaced: &'a mut MessageWriter<'r, WorldReplaced>,
    notice: &'a mut Notice,
    /// The play state the current world's mode calls for: where Resume goes.
    resume: PlayState,
    /// Whether the debug menu can open for the current world.
    debug: bool,
}

impl Actions<'_, '_, '_, '_, '_, '_, '_, '_> {
    fn start_game(&mut self, world: World, start: PlayState) {
        self.commands.insert_resource(SimWorld(world));
        self.commands.insert_resource(StartIn(start));
        self.next.app.set(AppState::Playing);
        // Presentation draws the new world before its first step.
        self.replaced.write(WorldReplaced);
    }

    fn leave_game(&mut self) {
        leave_game(self.commands, self.next);
    }
}

fn leave_game(commands: &mut Commands, next: &mut Next) {
    commands.remove_resource::<SimWorld>();
    next.app.set(AppState::MainMenu);
}

#[allow(clippy::too_many_arguments)]
fn menu_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<UiClick>,
    mut commands: Commands,
    at: Where,
    mut next: Next,
    mut screens: ResMut<Screens>,
    config: Res<AppConfig>,
    data: Option<Res<PackData>>,
    world: Option<Res<SimWorld>>,
    mut player: MessageWriter<PlayerCommand>,
    mut shell: MessageWriter<ShellCommand>,
    mut exit: MessageWriter<AppExit>,
    mut replaced: MessageWriter<WorldReplaced>,
    mut notice: ResMut<Notice>,
    // The UI plugin's selection; absent in an app without it (the menus alone are testable).
    selected: Option<Res<crate::ui::Selected>>,
) {
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    let resume = world
        .as_ref()
        .map_or(PlayState::Explore, |w| PlayState::for_mode(&w.0.mode));
    let debug = world
        .as_ref()
        .is_some_and(|w| debug_available(w.0.settings));
    let active = at.screen();
    let mut pressed: Vec<MenuKey> = keys.read().filter_map(menu_key).collect();
    for UiClick(hit) in clicks.read() {
        pressed.extend(click_keys(&mut screens, active, *hit));
    }
    if pressed.is_empty() {
        // Nothing to do, and no mutable borrow of the notice: taking `&mut` on it marks it
        // changed, which the message line reads as a new notice every frame.
        return;
    }
    let mut act = Actions {
        commands: &mut commands,
        next: &mut next,
        data: data.as_deref(),
        config: &config,
        player: &mut player,
        shell: &mut shell,
        exit: &mut exit,
        replaced: &mut replaced,
        notice: &mut notice,
        resume,
        debug,
    };
    for key in pressed {
        let Screens {
            title,
            new_game,
            creation,
            catalog,
            pause,
            cast,
            skin,
            ..
        } = &mut *screens;
        match active {
            Active::Title => {
                if let Some(action) = title.key(key) {
                    title_action(action, &mut act);
                }
            }
            Active::NewGame => {
                if let Some(action) = new_game.key(key) {
                    new_game_action(action, new_game, &mut act);
                }
            }
            // In the Feathers skin the panel's widgets own the keyboard.
            Active::CreateParty if skin.feathers => {}
            Active::CreateParty => {
                if let Some(action) = creation.key(key, catalog, members) {
                    creation_action(action, act.player, act.commands, act.next);
                }
            }
            Active::Paused => {
                if let Some(action) = pause.key(key) {
                    pause_action(action, &mut act);
                }
            }
            Active::Cast => cast_key(
                key,
                cast,
                &mut act,
                world.as_deref(),
                selected.as_ref().and_then(|s| s.0),
            ),
            // The combat, debug, sheet and inventory plugins handle their screens' keys.
            Active::Encounter
            | Active::Combat
            | Active::Defeat
            | Active::Debug
            | Active::Sheet
            | Active::Inventory
            | Active::None => {}
        }
    }
}

fn title_action(action: TitleAction, act: &mut Actions<'_, '_, '_, '_, '_, '_, '_, '_>) {
    match action {
        TitleAction::NewGame => act.next.menu.set(MenuState::NewGame),
        TitleAction::Load => {
            let Some(data) = act.data else { return };
            match load(&data.0, &act.config.save_path, false) {
                Ok(world) => {
                    let start = PlayState::for_mode(&world.mode);
                    act.start_game(world, start);
                }
                Err(e) => act.notice.0 = format!("Load failed: {e}"),
            }
        }
        TitleAction::Quit => {
            act.exit.write(AppExit::Success);
        }
    }
}

fn new_game_action(
    action: NewGameAction,
    form: &NewGameForm,
    act: &mut Actions<'_, '_, '_, '_, '_, '_, '_, '_>,
) {
    match action {
        NewGameAction::Start => {
            let Some(data) = act.data else { return };
            match World::new(
                &data.0,
                form.seed(crate::entropy_seed()),
                crate::sim::game_settings(form.settings),
            ) {
                Ok(world) => {
                    info!("new game, seed {:#x}, {:?}", world.seed, world.settings);
                    act.start_game(world, PlayState::CreateParty);
                }
                Err(e) => act.notice.0 = format!("{e}"),
            }
        }
        NewGameAction::Back => act.next.menu.set(MenuState::Title),
    }
}

fn creation_action(
    action: CreationAction,
    player: &mut MessageWriter<PlayerCommand>,
    commands: &mut Commands,
    next: &mut Next,
) {
    match action {
        CreationAction::Add(draft) => {
            player.write(PlayerCommand(Command::Party(PartyCommand::Create(draft))));
        }
        CreationAction::Begin => next.play.set(PlayState::Explore),
        CreationAction::Back => leave_game(commands, next),
    }
}

/// What the Feathers panel asked for, done exactly as the canvas screen's keys do it.
fn creation_asks(
    mut asks: MessageReader<CreationAsk>,
    at: Where,
    mut player: MessageWriter<PlayerCommand>,
    mut commands: Commands,
    mut next: Next,
) {
    for CreationAsk(action) in asks.read() {
        if at.screen() == Active::CreateParty {
            creation_action(action.clone(), &mut player, &mut commands, &mut next);
        }
    }
}

/// A key on the cast menu: a cast for the simulation, or back to exploring.
fn cast_key(
    key: MenuKey,
    menu: &mut CastMenu,
    act: &mut Actions<'_, '_, '_, '_, '_, '_, '_, '_>,
    world: Option<&SimWorld>,
    selected: Option<usize>,
) {
    let Some((world, data)) = world.zip(act.data) else {
        return;
    };
    let rows = cast_rows(&world.0, &data.0);
    menu.sync(&rows);
    match menu.key(key, &rows, selected) {
        Some(CastIntent::Command(command)) => {
            act.player.write(PlayerCommand(command));
            act.next.play.set(PlayState::Explore);
        }
        Some(CastIntent::Close) => act.next.play.set(PlayState::Explore),
        None => {}
    }
}

/// A pause item. Save and Load stay on the overlay so their notice shows on the band; Resume
/// after a load goes where the loaded world's mode says, since `resume` is read at key time.
fn pause_action(action: PauseAction, act: &mut Actions<'_, '_, '_, '_, '_, '_, '_, '_>) {
    match action {
        PauseAction::Resume => act.next.play.set(act.resume),
        PauseAction::Save => {
            act.shell.write(ShellCommand::Save);
        }
        PauseAction::Load => {
            act.shell.write(ShellCommand::Load);
        }
        PauseAction::Sheet => act.next.play.set(PlayState::Sheet),
        PauseAction::Debug if act.debug => act.next.play.set(PlayState::Debug),
        PauseAction::Debug => {
            act.notice.0 = if cfg!(feature = "devtools") {
                "Devtools are off in this game".to_owned()
            } else {
                "This build has no debug menu".to_owned()
            };
        }
        PauseAction::QuitToTitle => act.leave_game(),
        PauseAction::Quit => {
            act.exit.write(AppExit::Success);
        }
    }
}

fn refresh(
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    mut screens: ResMut<Screens>,
) {
    if events.read().any(|e| e.0 == Event::PartyChanged) {
        let Screens {
            creation, catalog, ..
        } = &mut *screens;
        creation.next_member(catalog);
    }
    for CommandRefused(rejection) in refused.read() {
        screens.creation.message = rejection.to_string();
    }
}
