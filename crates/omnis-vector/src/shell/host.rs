//! The dev socket's host ops on the session (ARCHITECTURE.md §9.1, `docs/api.md` §5): what the
//! engine's `dispatch` leaves to a host, modelled on `omnis_cli::Headless`. No Bevy: the socket
//! (`shell/socket.rs`) hands each op here with the session in hand, except `screenshot` and
//! `screen.text`, which need the renderer and the UI tree.
//!
//! Every other op goes to the engine's `dispatch` unchanged, so the rules and the replies are
//! the engine's. The commands it accepts are then kept in the binder's log, so the session
//! still replays, and the presentation follows them as it follows the keyboard: the HUD lines,
//! the roll log, the reshape flag and the pose.

use super::session::{Packs, Session};
use crate::game_io;
use crate::pose::Pose;
use omnis_sim::ops::rules_set;
use omnis_sim::{Command, DevCommand, Op, OpError, PartyCommand, Reply};
use std::path::Path;
use std::sync::Arc;

/// Answer one op. Relative paths in it (saves) are taken under `root`, the directory the
/// bridge reads them from.
///
/// # Errors
/// The op's error, as the engine or the host rules give it.
pub fn handle(session: &mut Session, root: &Path, op: &Op) -> Result<Reply, OpError> {
    match op {
        Op::SaveWrite { path } => {
            game_io::save(&session.world, &session.data, root, path)?;
            Ok(Reply::Written { path: path.clone() })
        }
        Op::SaveRead { path, force } => {
            let loaded = game_io::load(&session.data, root, path, *force)?;
            let lines = loaded.lines(path);
            session.replace(loaded.world, &format!("the save {path} was loaded"));
            for line in lines {
                session.say(line);
            }
            session.world.status(&session.data).map(Reply::Status)
        }
        Op::SaveCheck { path } => {
            let check = game_io::examine(&session.data, root, path)?.into_report();
            Ok(Reply::Check { check })
        }
        Op::PackReload => {
            let fresh = Packs::load(&session.config).map_err(OpError::failed)?.data;
            let check = session.reload(fresh)?;
            Ok(Reply::Done { check })
        }
        Op::RulesSet { slot, source } => {
            let reply = rules_set(Arc::make_mut(&mut session.data), slot, source)?;
            session.refresh();
            session.unreplayable = Some(format!("the rule {slot} was set"));
            Ok(reply)
        }
        Op::Screenshot { .. } | Op::ScreenText => Err(OpError::failed(
            "the socket answers this with the renderer and the interface in hand",
        )),
        other => {
            let before = session.views.position;
            let reply = session.world.dispatch(&session.data, other)?;
            let accepted = accepted(other, &reply);
            if !accepted.is_empty() {
                session.binder.log.extend(accepted);
                if let Reply::Events { events } | Reply::Script { events, .. } = &reply {
                    session.note(events);
                }
                // The pose takes the party's place and facing, or the binder would turn or walk
                // the party back to where the pose still is.
                if session.views.position != before {
                    session.pose = Pose::at(session.views.position);
                }
            }
            Ok(reply)
        }
    }
}

/// The commands `op` had the world accept, as its reply reports them: none for an op that only
/// reads, the one command of an accepted `sim.command`, `party.create` or `time.reconcile`, and
/// the first `applied` of a script.
fn accepted(op: &Op, reply: &Reply) -> Vec<Command> {
    match (op, reply) {
        (Op::SimCommand { command }, Reply::Events { .. }) => vec![command.clone()],
        (Op::SimScript { commands }, Reply::Script { applied, .. }) => {
            commands.iter().take(*applied).cloned().collect()
        }
        (Op::PartyCreate { character }, Reply::Events { .. }) => {
            vec![Command::Party(PartyCommand::Create(character.clone()))]
        }
        (Op::TimeReconcile { region }, Reply::Events { .. }) => {
            vec![Command::Dev(DevCommand::Reconcile {
                region: region.clone(),
            })]
        }
        _ => Vec::new(),
    }
}
