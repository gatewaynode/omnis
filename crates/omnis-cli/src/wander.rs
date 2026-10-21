//! A seeded party that wanders the loaded maps for a number of commands, recorded as a replay
//! (M8.5 7a): the load to measure the journal against (PRD D28). It plays through the public
//! API only, the way a player would, with the dev commands a tester would use to keep going.

use omnis_core::{Direction, Facing, Pcg32, Rotation, StreamName};
use omnis_data::ron_io;
use omnis_data::{Alignment, Data, Skill};
use omnis_sim::omnis_rules::Draft;
use omnis_sim::{
    ActorRef, CombatCommand, Command, DevCommand, EncounterChoice, Event, Mode, PartyCommand,
    Replay, RestCommand, ServiceCommand, Settings, World, apply, combat_view, party_view,
};
use std::collections::BTreeSet;

/// Commands between two teleports to a random cell, so every map is walked.
pub const TELEPORT_EVERY: usize = 1000;
/// Refusals in a row before the wanderer gives up as stuck.
const STUCK: usize = 200;
/// Commands between two tries at a long rest while someone is hurt.
const REST_EVERY: usize = 50;

/// A wander and what it cost.
#[derive(Debug, Clone)]
pub struct Wander {
    /// The commands, recorded with the fingerprint they end on.
    pub replay: Replay,
    /// Bytes of RON the events took, without the per-step `Visible` and encounter checks.
    pub event_bytes: usize,
    /// The party's age at the end, in minutes.
    pub minutes: i64,
    /// Maps the party stood on.
    pub maps: usize,
    /// Fights started.
    pub fights: usize,
    /// Fallen members raised by a dev command.
    pub revived: usize,
}

/// Four drafts, marching order fighter, cleric, wizard, rogue, on the base pack.
fn drafts() -> Vec<Draft> {
    let draft = |name: &str, race: &str, class: &str, scores, skills: &[Skill]| Draft {
        name: name.to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::LawfulGood,
        scores,
        skills: skills.to_vec(),
    };
    vec![
        draft(
            "Brenna",
            "human",
            "fighter",
            [15, 14, 13, 12, 10, 8],
            &[Skill::Athletics, Skill::Perception],
        ),
        draft(
            "Durin",
            "dwarf",
            "cleric",
            [10, 8, 14, 10, 15, 8],
            &[Skill::Medicine, Skill::History],
        ),
        draft(
            "Ilvara",
            "elf",
            "wizard",
            [8, 14, 13, 15, 12, 10],
            &[Skill::Arcana, Skill::History],
        ),
        draft(
            "Pip",
            "halfling",
            "rogue",
            [8, 15, 12, 10, 13, 14],
            &[
                Skill::Stealth,
                Skill::Acrobatics,
                Skill::Deception,
                Skill::Perception,
            ],
        ),
    ]
}

/// The walk under way.
struct Walk<'a> {
    data: &'a Data,
    world: World,
    rng: Pcg32,
    commands: Vec<Command>,
    event_bytes: usize,
    maps: BTreeSet<omnis_core::MapId>,
    fights: usize,
    revived: usize,
}

impl Walk<'_> {
    /// Apply `command`; whether it was accepted. Accepted commands are recorded.
    fn try_apply(&mut self, command: Command) -> Result<bool, String> {
        let Ok(events) = apply(&mut self.world, self.data, command.clone()) else {
            return Ok(false);
        };
        let kept: Vec<&Event> = events
            .iter()
            .filter(|e| !matches!(e, Event::Visible { .. } | Event::EncounterCheck { .. }))
            .collect();
        self.event_bytes += ron_io::to_string(&kept).map_err(|e| e.to_string())?.len();
        self.fights += events
            .iter()
            .filter(|e| matches!(e, Event::CombatStarted { .. }))
            .count();
        self.maps.insert(self.world.position.map);
        self.commands.push(command);
        Ok(true)
    }

    /// The first of `choices` the world accepts; whether one was.
    fn first(&mut self, choices: Vec<Command>) -> Result<bool, String> {
        for command in choices {
            if self.try_apply(command)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// One move in whatever mode the world is in.
    fn turn(&mut self) -> Result<bool, String> {
        let choices = match &self.world.mode {
            Mode::Town(_) => vec![
                Command::Service(ServiceCommand::Leave),
                Command::Step(Direction::Back),
            ],
            Mode::Encounter(_) => vec![
                Command::Encounter(EncounterChoice::Attack),
                Command::Encounter(EncounterChoice::Run),
            ],
            Mode::Combat(_) => self.fight(),
            Mode::Explore => return self.explore(),
        };
        self.first(choices)
    }

    /// A member's turn: hit the first stack in reach, else end the turn, dodge, or run.
    fn fight(&self) -> Vec<Command> {
        let mut choices = Vec::new();
        if let Some(view) = combat_view(&self.world, self.data)
            && matches!(view.current, Some(ActorRef::Member(_)))
            && let Some(stack) = view.stacks.iter().find(|s| s.alive && s.reachable)
        {
            choices.push(Command::Combat(CombatCommand::Attack {
                stack: stack.stack,
            }));
        }
        choices.extend([
            Command::Combat(CombatCommand::EndTurn),
            Command::Combat(CombatCommand::Dodge),
            Command::Combat(CombatCommand::Run),
        ]);
        choices
    }

    /// Walking: revive a fallen member, rest the hurt now and then, teleport now and then, else a
    /// seeded step, turn, use or search; a refusal turns right.
    fn explore(&mut self) -> Result<bool, String> {
        let party = party_view(&self.world, self.data);
        if let Some(fallen) = party.members.iter().find(|m| m.hp <= 0) {
            let revived = self.try_apply(Command::Dev(DevCommand::SetHp {
                member: fallen.member,
                hp: fallen.hp_max,
            }))?;
            self.revived += usize::from(revived);
            return Ok(revived);
        }
        let n = self.commands.len();
        let hurt = party.members.iter().any(|m| m.hp * 2 < m.hp_max);
        if hurt
            && n.is_multiple_of(REST_EVERY)
            && self.try_apply(Command::Rest(RestCommand::Long))?
        {
            return Ok(true);
        }
        if n > 0 && n.is_multiple_of(TELEPORT_EVERY) && self.teleport()? {
            return Ok(true);
        }
        let command = match self.rng.below(20) {
            0..=13 => Command::Step(Direction::Forward),
            14 => Command::Turn(Rotation::Left),
            15 => Command::Turn(Rotation::Right),
            16 => Command::Interact,
            17 => Command::Search,
            18 => Command::Door,
            _ => Command::Step(Direction::Left),
        };
        if self.try_apply(command)? {
            return Ok(true);
        }
        self.try_apply(Command::Turn(Rotation::Right))
    }

    /// A seeded cell of a seeded map, tried until one takes the party.
    fn teleport(&mut self) -> Result<bool, String> {
        let names: Vec<String> = self.data.registry.maps.names().map(String::from).collect();
        for _ in 0..STUCK {
            let at = self.rng.below(u32::try_from(names.len()).unwrap_or(1)) as usize;
            let Some(id) = self.data.registry.maps.get(&names[at]) else {
                continue;
            };
            let def = &self.data.maps[&id].def;
            let command = Command::Dev(DevCommand::Teleport {
                map: names[at].clone(),
                level: omnis_core::Level::GROUND,
                x: self.rng.below(u32::from(def.width)) as u16,
                y: self.rng.below(u32::from(def.height)) as u16,
                facing: Facing::ALL[self.rng.below(4) as usize],
            });
            if self.try_apply(command)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Wander `commands` commands from a new game on `seed`, the party of four created first (those
/// commands count).
pub fn wander(data: &Data, seed: u64, commands: usize) -> Result<Wander, String> {
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    let world = World::new(data, seed, settings).map_err(|e| e.to_string())?;
    let mut walk = Walk {
        data,
        world,
        rng: Pcg32::for_stream(seed, &StreamName::new("wander")),
        commands: Vec::new(),
        event_bytes: 0,
        maps: BTreeSet::new(),
        fights: 0,
        revived: 0,
    };
    for draft in drafts() {
        if !walk.try_apply(Command::Party(PartyCommand::Create(draft)))? {
            return Err("a draft was refused".into());
        }
    }
    let mut refused = 0;
    while walk.commands.len() < commands {
        if walk.turn()? {
            refused = 0;
        } else {
            refused += 1;
            if refused >= STUCK {
                return Err(format!(
                    "stuck after {} commands in {:?}",
                    walk.commands.len(),
                    walk.world.mode.kind()
                ));
            }
        }
    }
    let fingerprint = walk.world.fingerprint().map_err(|e| e.to_string())?;
    Ok(Wander {
        minutes: walk.world.party_clock().elapsed,
        maps: walk.maps.len(),
        fights: walk.fights,
        revived: walk.revived,
        event_bytes: walk.event_bytes,
        replay: Replay {
            seed,
            packs: data.fingerprints.clone(),
            settings,
            commands: walk.commands,
            fingerprint,
        },
    })
}
