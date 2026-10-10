//! A session at the dungeon's placed group, met by walking into it, and its turns.

use super::app::met_with;
use omnis_sim::{ActorRef, CombatCommand, Command, EncounterChoice, Mode, combat_view};
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
