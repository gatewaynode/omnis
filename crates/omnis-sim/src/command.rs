//! What a client may ask the simulation to do, and why it may refuse (ARCHITECTURE.md §4.2).
//! Commands carry no client state so a command stream is a replay and, later, a network
//! protocol. What happened is `event::Event`.

use crate::combat::CombatCommand;
use crate::dev::DevCommand;
use crate::encounter::EncounterChoice;
use crate::party::PartyCommand;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{Direction, ItemId, Rotation};
use omnis_rules::{CreationError, RuleError};
use serde::{Deserialize, Serialize};

/// One player action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Move one tile relative to the facing, without turning.
    Step(Direction),
    /// Turn in place.
    Turn(Rotation),
    /// Use whatever is on the facing edge or tile: a door in M1.
    Interact,
    /// Build or reorder the party.
    Party(PartyCommand),
    /// Choose what to do about the monsters ahead.
    Encounter(EncounterChoice),
    /// Act in a fight, on the acting member's turn.
    Combat(CombatCommand),
    /// A debugging edit; accepted only when the world's settings say `devtools`.
    Dev(DevCommand),
}

impl Command {
    /// The script word for this command; see [`parse_script`]. Party commands carry data and
    /// have no script word; they log as `party`. Combat commands log their bare verb.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Command::Step(Direction::Forward) => "forward",
            Command::Step(Direction::Back) => "back",
            Command::Step(Direction::Left) => "left",
            Command::Step(Direction::Right) => "right",
            Command::Turn(Rotation::Left) => "turn-left",
            Command::Turn(Rotation::Right) => "turn-right",
            Command::Turn(Rotation::Around) => "around",
            Command::Interact => "use",
            Command::Party(_) => "party",
            Command::Encounter(EncounterChoice::Attack) => "fight",
            Command::Encounter(EncounterChoice::Bribe) => "bribe",
            Command::Encounter(EncounterChoice::Hide) => "hide",
            Command::Encounter(EncounterChoice::Run) => "run",
            Command::Combat(CombatCommand::Attack { .. }) => "attack",
            Command::Combat(CombatCommand::Dodge) => "dodge",
            Command::Combat(CombatCommand::Exchange { .. }) => "swap",
            Command::Combat(CombatCommand::Run) => "flee",
            Command::Dev(_) => "dev",
        }
    }

    /// The command for a script word, if it is one.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Command> {
        Some(match word {
            "forward" => Command::Step(Direction::Forward),
            "back" => Command::Step(Direction::Back),
            "left" => Command::Step(Direction::Left),
            "right" => Command::Step(Direction::Right),
            "turn-left" => Command::Turn(Rotation::Left),
            "turn-right" => Command::Turn(Rotation::Right),
            "around" => Command::Turn(Rotation::Around),
            "use" => Command::Interact,
            "fight" => Command::Encounter(EncounterChoice::Attack),
            "bribe" => Command::Encounter(EncounterChoice::Bribe),
            "hide" => Command::Encounter(EncounterChoice::Hide),
            "run" => Command::Encounter(EncounterChoice::Run),
            "attack" => Command::Combat(CombatCommand::Attack { stack: 0 }),
            "dodge" => Command::Combat(CombatCommand::Dodge),
            "flee" => Command::Combat(CombatCommand::Run),
            _ => {
                if let Some(n) = word.strip_prefix("attack-") {
                    return n
                        .parse()
                        .ok()
                        .map(|stack| Command::Combat(CombatCommand::Attack { stack }));
                }
                if let Some(n) = word.strip_prefix("swap-") {
                    return n
                        .parse()
                        .ok()
                        .map(|with| Command::Combat(CombatCommand::Exchange { with }));
                }
                return None;
            }
        })
    }
}

/// A word in a script that is not a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    /// One-based line of the word.
    pub line: usize,
    /// The word.
    pub word: String,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: unknown command '{}'", self.line, self.word)
    }
}

/// Parse a command script: words `forward`, `back`, `left`, `right` (sidesteps),
/// `turn-left`, `turn-right`, `around`, `use`, before a fight `fight`, `bribe`, `hide`, `run`,
/// and in one `attack` (the first stack), `attack-N`, `dodge`, `swap-N`, `flee`, separated by
/// whitespace or commas; `#` starts a comment that runs to the end of the line.
pub fn parse_script(text: &str) -> Result<Vec<Command>, ScriptError> {
    let mut commands = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = line.split('#').next().unwrap_or("");
        for word in code.split(|c: char| c.is_whitespace() || c == ',') {
            if word.is_empty() {
                continue;
            }
            match Command::from_word(word) {
                Some(command) => commands.push(command),
                None => {
                    return Err(ScriptError {
                        line: index + 1,
                        word: word.to_string(),
                    });
                }
            }
        }
    }
    Ok(commands)
}

/// A command the rules refuse. Not an error: the world is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rejection {
    /// The command does not apply in the current mode.
    WrongMode,
    /// Every party slot is taken.
    PartyFull,
    /// The draft does not make a character.
    Character(CreationError),
    /// The order is not a permutation of the current members.
    BadOrder,
    /// The fight is not waiting on a member, or not on a living one.
    NotYourTurn,
    /// No stack has that index.
    NoSuchStack {
        /// The index asked for.
        stack: u8,
    },
    /// Nobody in that stack still stands.
    StackDead {
        /// The index asked for.
        stack: u8,
    },
    /// A front-row member without a ranged weapon cannot reach a stack behind the front.
    OutOfReach {
        /// The index asked for.
        stack: u8,
    },
    /// A back-row member needs a ranged weapon to attack at all.
    NeedsRangedWeapon,
    /// No member has that slot.
    NoSuchMember {
        /// The slot asked for.
        index: u8,
    },
    /// A member cannot exchange with themselves.
    SameMember,
    /// The party cannot pay.
    CannotAfford {
        /// The price.
        cost: u32,
        /// The purse.
        gold: u32,
    },
    /// The stores or the kit hold fewer of an item than needed.
    NotEnough {
        /// The item.
        item: ItemId,
        /// How many there are.
        have: u16,
    },
    /// A `Dev` command in a world whose settings do not allow them.
    DevOnly,
    /// No loaded pack defines that id.
    UnknownId {
        /// The id asked for.
        id: String,
    },
    /// A count of zero, a score outside `1..=30`, or an index past the end.
    OutOfRange,
    /// The tile is not on the map.
    OffMap {
        /// Column.
        x: u16,
        /// Row.
        y: u16,
    },
    /// A rule formula failed while resolving: bad pack data, reported rather than a panic.
    Rule(RuleError),
}

impl core::fmt::Display for Rejection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Rejection::WrongMode => f.write_str("command does not apply in the current mode"),
            Rejection::PartyFull => f.write_str("the party is full"),
            Rejection::Character(e) => write!(f, "{e}"),
            Rejection::BadOrder => f.write_str("order must list every member once"),
            Rejection::NotYourTurn => f.write_str("it is not a member's turn"),
            Rejection::NoSuchStack { stack } => write!(f, "there is no stack {stack}"),
            Rejection::StackDead { stack } => write!(f, "stack {stack} is dead"),
            Rejection::OutOfReach { stack } => {
                write!(
                    f,
                    "stack {stack} is behind the front; a ranged weapon reaches it"
                )
            }
            Rejection::NeedsRangedWeapon => {
                f.write_str("a back-row member needs a ranged weapon to attack")
            }
            Rejection::NoSuchMember { index } => write!(f, "there is no member in slot {index}"),
            Rejection::SameMember => f.write_str("a member cannot exchange with themselves"),
            Rejection::CannotAfford { cost, gold } => {
                write!(f, "that costs {cost} gold; the party has {gold}")
            }
            Rejection::NotEnough { item, have } => {
                write!(f, "not enough of item {item}; there are {have}")
            }
            Rejection::DevOnly => f.write_str("dev commands need a devtools world"),
            Rejection::UnknownId { id } => write!(f, "no loaded pack defines '{id}'"),
            Rejection::OutOfRange => f.write_str("a count, score, or index is out of range"),
            Rejection::OffMap { x, y } => write!(f, "({x}, {y}) is not on the map"),
            Rejection::Rule(e) => write!(f, "rule error: {e}"),
        }
    }
}
