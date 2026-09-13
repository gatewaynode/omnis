//! `CombatPlugin`: the encounter, fight, and defeat screens driven by the state machines in
//! `combat_menu.rs`, the play state following the world's mode, and the roll log fed from
//! `combat_text.rs`. Keys and clicks arrive as they do for the menus; the intents become
//! simulation or shell commands. Headless-capable.

use crate::combat_menu::{CombatIntent, DefeatAction, EncounterIntent, fight_view};
use crate::combat_text::batch_lines;
use crate::cursor::UiSet;
use crate::menu::MenuKey;
use crate::menus::{Active, Screens, Where, menu_key};
use crate::screen::{self, Target};
use crate::sim::{
    AppState, PackData, PlayState, PlayerCommand, ShellCommand, SimEvent, SimSet, SimWorld,
    WorldReplaced,
};
use crate::ui::{EventNames, RollLog, Selected, UiClick, message_line};
use crate::widget::Hit;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use omnis_sim::{CombatOutcome, Command, Event};

/// The combat plugin.
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<KeyboardInput>()
            .add_message::<UiClick>()
            .add_systems(Update, combat_keys.in_set(UiSet::Dispatch))
            .add_systems(
                Update,
                follow_mode.in_set(SimSet::Publish).before(UiSet::Model),
            )
            .add_systems(
                Update,
                combat_model.in_set(UiSet::Model).before(message_line),
            );
    }
}

/// The play state follows the world's mode: a wipe opens the defeat modal, which a load
/// leaves through the loaded world's mode.
fn follow_mode(
    mut events: MessageReader<SimEvent>,
    mut replaced: MessageReader<WorldReplaced>,
    state: Option<Res<State<PlayState>>>,
    world: Option<Res<SimWorld>>,
    mut next: ResMut<NextState<PlayState>>,
) {
    let wiped = events.read().any(|e| {
        matches!(
            e.0,
            Event::CombatEnded {
                outcome: CombatOutcome::Defeat,
                ..
            }
        )
    });
    let was_replaced = replaced.read().count() > 0;
    let (Some(state), Some(world)) = (state, world) else {
        return;
    };
    let current = *state.get();
    let wanted = PlayState::for_mode(&world.0.mode);
    let follows = matches!(
        current,
        PlayState::Explore | PlayState::Encounter | PlayState::Combat
    ) && current != wanted;
    let leaves_defeat = current == PlayState::Defeat && was_replaced;
    if wiped {
        next.set(PlayState::Defeat);
    } else if follows || leaves_defeat {
        next.set(wanted);
    }
}

/// Names and the roll log follow the events; the combat menu's target follows the stacks.
fn combat_model(
    mut events: MessageReader<SimEvent>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut names: ResMut<EventNames>,
    mut log: ResMut<RollLog>,
    mut screens: ResMut<Screens>,
) {
    let (Some(world), Some(data)) = (world, data) else {
        return;
    };
    let batch: Vec<Event> = events.read().map(|e| e.0.clone()).collect();
    if batch
        .iter()
        .any(|e| matches!(e, Event::EncounterStarted { .. }))
    {
        log.clear();
    }
    names.0.refresh(&world.0, &data.0);
    for text in batch.iter().filter_map(crate::ui::event_text) {
        log.push(text);
    }
    for line in batch_lines(&batch, &names.0) {
        log.push(line.long);
    }
    if let Some(view) = fight_view(&world.0, &data.0) {
        screens.combat.sync(&view);
    }
}

/// Keys and clicks on the fight screens.
#[allow(clippy::too_many_arguments)]
fn combat_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<UiClick>,
    mut commands: Commands,
    at: Where,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    selected: Res<Selected>,
    mut player: MessageWriter<PlayerCommand>,
    mut shell: MessageWriter<ShellCommand>,
    mut next: ResMut<NextState<AppState>>,
) {
    // Read every frame so a key from before the fight is never replayed into it.
    let mut pressed: Vec<MenuKey> = keys.read().filter_map(menu_key).collect();
    let hits: Vec<Hit> = clicks.read().map(|c| c.0).collect();
    let active = at.screen();
    if !matches!(active, Active::Encounter | Active::Combat | Active::Defeat) {
        return;
    }
    let view = world
        .as_ref()
        .zip(data.as_ref())
        .and_then(|(w, d)| fight_view(&w.0, &d.0));
    for hit in hits {
        let target = match active {
            Active::Encounter => Target::Encounter(&mut screens.encounter),
            Active::Combat => Target::Combat(&mut screens.combat),
            _ => Target::Defeat(&mut screens.defeat),
        };
        pressed.extend(screen::click(target, hit));
    }
    for key in pressed {
        match active {
            Active::Encounter => {
                let Some(view) = &view else { continue };
                match screens.encounter.key(key, view) {
                    Some(EncounterIntent::Choice(choice)) => {
                        player.write(PlayerCommand(Command::Encounter(choice)));
                    }
                    Some(EncounterIntent::Pause) => {
                        shell.write(ShellCommand::Pause);
                    }
                    None => {}
                }
            }
            Active::Combat => {
                let Some(view) = &view else { continue };
                screens.combat.sync(view);
                match screens.combat.key(key, view, selected.0) {
                    Some(CombatIntent::Command(command)) => {
                        player.write(PlayerCommand(Command::Combat(command)));
                    }
                    Some(CombatIntent::Pause) => {
                        shell.write(ShellCommand::Pause);
                    }
                    None => {}
                }
            }
            _ => match screens.defeat.key(key) {
                Some(DefeatAction::Load) => {
                    shell.write(ShellCommand::Load);
                }
                Some(DefeatAction::QuitToTitle) => {
                    commands.remove_resource::<SimWorld>();
                    next.set(AppState::MainMenu);
                }
                None => {}
            },
        }
    }
}
