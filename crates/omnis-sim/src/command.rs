//! What a client may ask the simulation to do, and why it may refuse (ARCHITECTURE.md §4.2).
//! Commands carry no client state so a command stream is a replay and, later, a network
//! protocol. What happened is `event::Event`.

use crate::combat::{CombatCommand, Target};
use crate::dev::DevCommand;
use crate::encounter::EncounterChoice;
use crate::items::ItemCommand;
use crate::party::PartyCommand;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{Direction, ItemId, Rotation};
use omnis_data::EquipSlot;
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
    /// Cast outside a fight: healing, a buff, light, or mage hand.
    Cast {
        /// The caster's slot.
        caster: u8,
        /// Index into the caster's known spells.
        spell: u8,
        /// Whom it goes to (a member for healing and buffs; ignored by light and mage hand).
        target: Target,
    },
    /// Wear, hand over, stow, take, or use a carried item outside a fight.
    Item(ItemCommand),
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
            Command::Combat(CombatCommand::Cast { .. }) => "cast",
            Command::Combat(CombatCommand::Use { .. }) => "use-item",
            Command::Combat(CombatCommand::Dodge) => "dodge",
            Command::Combat(CombatCommand::Exchange { .. }) => "swap",
            Command::Combat(CombatCommand::Run) => "flee",
            Command::Cast { .. } => "cast",
            Command::Item(_) => "item",
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
                if let Some(rest) = word.strip_prefix("cast-") {
                    return parse_cast(rest).map(Command::Combat);
                }
                if let Some(rest) = word.strip_prefix("use-item-") {
                    return parse_use(rest).map(Command::Combat);
                }
                return None;
            }
        })
    }
}

/// `N-M` casts spell `N` at stack `M`; `N-mM` at member `M`.
fn parse_cast(rest: &str) -> Option<CombatCommand> {
    let (spell, target) = rest.split_once('-')?;
    let spell = spell.parse().ok()?;
    let target = match target.strip_prefix('m') {
        Some(member) => Target::Member(member.parse().ok()?),
        None => Target::Stack(target.parse().ok()?),
    };
    Some(CombatCommand::Cast { spell, target })
}

/// `N` uses item `N` of the acting member's kit on themselves; `N-mM` on member `M`.
fn parse_use(rest: &str) -> Option<CombatCommand> {
    let (item, target) = match rest.split_once('-') {
        Some((item, target)) => (item, Some(target.strip_prefix('m')?.parse().ok()?)),
        None => (rest, None),
    };
    Some(CombatCommand::Use {
        item: item.parse().ok()?,
        target,
    })
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
/// and in one `attack` (the first stack), `attack-N`, `cast-N-M` (spell `N` at stack `M`),
/// `cast-N-mM` (at member `M`), `use-item-N` (item `N` of the acting member's kit, on
/// themselves), `use-item-N-mM` (on member `M`), `dodge`, `swap-N`, `flee`, separated by
/// whitespace or commas;
/// `#` starts a comment that runs to the end of the line.
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
    /// The caster knows no spell at that index.
    UnknownSpell {
        /// The index asked for.
        spell: u8,
    },
    /// The spell has no effect the simulation can cast here yet.
    NotCastable {
        /// The index asked for.
        spell: u8,
    },
    /// The caster's pool is short.
    NotEnoughPoints {
        /// The cost.
        need: u32,
        /// The pool.
        have: u32,
    },
    /// The spell's components are not in the party's stores, or a spell at the component
    /// threshold lists none.
    MissingComponents {
        /// The index asked for.
        spell: u8,
    },
    /// A stack for a spell that helps members, or a member for one that hurts monsters.
    WrongTarget,
    /// The member is dead; no spell here raises the dead.
    MemberDead {
        /// The slot asked for.
        index: u8,
    },
    /// The member is at zero hit points and cannot act.
    MemberDown {
        /// The slot asked for.
        index: u8,
    },
    /// The stores or the kit hold fewer of an item than needed.
    NotEnough {
        /// The item.
        item: ItemId,
        /// How many there are.
        have: u16,
    },
    /// The kit has no item at that row.
    UnknownItem {
        /// The row asked for.
        item: u8,
    },
    /// The stores have no item at that row.
    NotInStores {
        /// The row asked for.
        item: u8,
    },
    /// The member does not carry the item.
    NotCarried,
    /// The item has no slot: gear, a component, a focus.
    NotEquippable,
    /// A two-handed weapon and a shield cannot both be held.
    HandsFull,
    /// Nothing is in that slot.
    SlotEmpty {
        /// The slot asked for.
        slot: EquipSlot,
    },
    /// The item does nothing when used.
    NotUsable,
    /// The item is not used from a fight.
    NotUsableHere,
    /// The member the item goes to is dead.
    TargetDead {
        /// The slot asked for.
        index: u8,
    },
    /// A count of zero moves nothing.
    ZeroCount,
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
        self.fmt_play(f)
            .or_else(|| self.fmt_items(f))
            .unwrap_or_else(|| self.fmt_magic(f))
    }
}

impl Rejection {
    /// The wording of the mode, party and fight refusals; `None` for the rest.
    fn fmt_play(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
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
            Rejection::MemberDead { index } => write!(f, "the member in slot {index} is dead"),
            Rejection::MemberDown { index } => write!(f, "the member in slot {index} is down"),
            Rejection::Rule(e) => write!(f, "rule error: {e}"),
            _ => return None,
        })
    }

    /// The wording of the item refusals; `None` for the rest.
    fn fmt_items(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Rejection::UnknownItem { item } => write!(f, "the kit has no item at row {item}"),
            Rejection::NotInStores { item } => {
                write!(f, "the stores have no item at row {item}")
            }
            Rejection::NotCarried => f.write_str("the item is not carried"),
            Rejection::NotEquippable => f.write_str("the item cannot be worn or wielded"),
            Rejection::HandsFull => f.write_str("a two-handed weapon leaves no hand for a shield"),
            Rejection::SlotEmpty { slot } => write!(f, "nothing is in the {slot:?} slot"),
            Rejection::NotUsable => f.write_str("the item does nothing when used"),
            Rejection::NotUsableHere => f.write_str("the item is not used from a fight"),
            Rejection::TargetDead { index } => write!(f, "the member in slot {index} is dead"),
            Rejection::ZeroCount => f.write_str("a count of zero moves nothing"),
            _ => return None,
        })
    }

    /// The wording of the casting, component and dev refusals.
    fn fmt_magic(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::UnknownSpell { spell } => write!(f, "no known spell at {spell}"),
            Rejection::NotCastable { spell } => write!(f, "spell {spell} cannot be cast here"),
            Rejection::NotEnoughPoints { need, have } => {
                write!(f, "that needs {need} spell points; the caster has {have}")
            }
            Rejection::MissingComponents { spell } => {
                write!(f, "the stores lack spell {spell}'s components")
            }
            Rejection::WrongTarget => f.write_str("the spell cannot go to that target"),
            Rejection::NotEnough { item, have } => {
                write!(f, "not enough of item {item}; there are {have}")
            }
            Rejection::DevOnly => f.write_str("dev commands need a devtools world"),
            Rejection::UnknownId { id } => write!(f, "no loaded pack defines '{id}'"),
            Rejection::OutOfRange => f.write_str("a count, score, or index is out of range"),
            Rejection::OffMap { x, y } => write!(f, "({x}, {y}) is not on the map"),
            other => write!(f, "{other:?}"),
        }
    }
}
