//! A session at the dungeon's placed group, met by walking into it, and its turns.

use super::app::{met_with, set};
use bevy::prelude::Interaction;
use omnis_sim::{ActorRef, CombatCommand, Command, EncounterChoice, Mode, combat_view};
use omnis_vector::combat_menu::{Act, Step};
use omnis_vector::screen::ControlId;
use omnis_vector::screen::fight::FightAct;
use omnis_vector::shell::screen::{FightPress, Screens};
use omnis_vector::shell::session::Session;

/// The session at the placed group's encounter, seed 1.
pub fn met() -> Session {
    let mut app = met_with(1);
    app.world_mut()
        .remove_resource::<Session>()
        .expect("a session")
}

pub fn fighting(session: &Session) -> bool {
    matches!(session.world.mode, Mode::Combat(_))
}

/// The name of the member whose turn it is.
pub fn acting(session: &Session) -> Option<String> {
    let view = combat_view(&session.world, &session.data)?;
    let Some(ActorRef::Member(id)) = view.current else {
        return None;
    };
    let member = session.world.party.members.iter().find(|m| m.id == id)?;
    Some(member.name.clone())
}

/// Fight, then dodge every turn until `name` acts.
pub fn turn_of(name: &str) -> Session {
    let mut session = met();
    session.order(Command::Encounter(EncounterChoice::Attack));
    for _ in 0..40 {
        assert!(fighting(&session), "the fight lasted until {name} acted");
        if acting(&session).as_deref() == Some(name) {
            return session;
        }
        session.order(Command::Combat(CombatCommand::Dodge));
    }
    panic!("{name} never acted");
}

/// The fight screen's controls in an app: each entry's control and what it does, as the menu
/// lists them at its step now.
pub fn entries(app: &bevy::prelude::App) -> Vec<(ControlId, Act)> {
    let session = app.world().resource::<Session>();
    let menu = app.world().resource::<Screens>().forms.fight.menu;
    menu.entries(&session.world, &session.data)
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let n = u8::try_from(i).expect("a few entries");
            (ControlId::Fight(FightAct::Entry(n)), e.act)
        })
        .collect()
}

/// The menu's step in an app.
pub fn step(app: &bevy::prelude::App) -> Step {
    app.world().resource::<Screens>().forms.fight.menu.step
}

/// What the fight screen's live buttons do, top to bottom.
pub fn enabled(app: &mut bevy::prelude::App) -> Vec<Act> {
    let acts = entries(app);
    let world = app.world_mut();
    let mut query =
        world.query_filtered::<&FightPress, bevy::prelude::With<bevy::prelude::Button>>();
    let live: Vec<ControlId> = query.iter(world).map(|p| p.0).collect();
    acts.into_iter()
        .filter(|(id, _)| live.contains(id))
        .map(|(_, act)| act)
        .collect()
}

/// Press the fight screen's button that does `act`, then let the frame run; the press is let go
/// after, as a click's is.
pub fn press(app: &mut bevy::prelude::App, act: &Act) {
    let id = entries(app)
        .into_iter()
        .find(|(_, a)| a == act)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("an entry {act:?}"));
    set(app, &FightPress(id), Interaction::Pressed);
    app.update();
    let world = app.world_mut();
    let mut buttons = world.query::<(&FightPress, &mut Interaction)>();
    for (p, mut i) in buttons.iter_mut(world) {
        if p.0 == id {
            *i = Interaction::None;
        }
    }
}
