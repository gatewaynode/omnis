//! What a client may ask the simulation to do, and why it may refuse (ARCHITECTURE.md §4.2).
//! Commands carry no client state so a command stream is a replay and, later, a network
//! protocol. What happened is `event::Event`.

use crate::combat::{CombatCommand, FeatureChoice, Pay, Target};
use crate::dev::DevCommand;
use crate::encounter::EncounterChoice;
use crate::items::ItemCommand;
use crate::party::PartyCommand;
use crate::rest::RestCommand;
use crate::service::ServiceCommand;
use crate::tactics::TacticsCommand;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{Coins, Direction, ItemId, Rotation};
use omnis_data::EquipSlot;
use omnis_rules::{CreationError, RuleError, TacticsFault};
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
    /// Buy, rest, heal, bank or leave inside a service (M7).
    Service(ServiceCommand),
    /// Rest outside a service: an hour spending hit dice, or the night (M7 step 5).
    Rest(RestCommand),
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
            Command::Combat(CombatCommand::Feature { .. }) => "feature",
            Command::Combat(CombatCommand::EndTurn) => "end",
            Command::Cast { .. } => "cast",
            Command::Item(_) => "item",
            Command::Service(ServiceCommand::Leave) => "leave",
            Command::Service(ServiceCommand::Room) => "room",
            Command::Service(ServiceCommand::Rumor) => "rumor",
            Command::Service(ServiceCommand::BuyFood { .. }) => "food",
            Command::Service(ServiceCommand::Heal { .. }) => "heal",
            Command::Service(ServiceCommand::Cure { .. }) => "cure",
            Command::Service(ServiceCommand::Raise { .. }) => "raise",
            Command::Service(ServiceCommand::Buy { .. }) => "buy",
            Command::Service(ServiceCommand::Sell { .. }) => "sell",
            Command::Service(ServiceCommand::Deposit { .. }) => "deposit",
            Command::Service(ServiceCommand::Withdraw { .. }) => "withdraw",
            Command::Service(ServiceCommand::Train { .. }) => "train",
            Command::Service(ServiceCommand::Choose { .. }) => "choose",
            Command::Service(ServiceCommand::Learn { .. }) => "learn",
            Command::Rest(RestCommand::Long) => "rest",
            Command::Rest(RestCommand::Short { .. }) => "short-rest",
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
            "leave" => Command::Service(ServiceCommand::Leave),
            "room" => Command::Service(ServiceCommand::Room),
            "rumor" => Command::Service(ServiceCommand::Rumor),
            "rest" => Command::Rest(RestCommand::Long),
            "short-rest" => Command::Rest(RestCommand::Short { dice: Vec::new() }),
            "end" => Command::Combat(CombatCommand::EndTurn),
            _ => {
                if let Some(rest) = word.strip_prefix("react-") {
                    return parse_react(rest).map(Command::Party);
                }
                if let Some(rest) = word.strip_prefix("feature-") {
                    return parse_feature(rest).map(Command::Combat);
                }
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
                return parse_town(word);
            }
        })
    }
}

/// `N-M` casts spell `N` at stack `M`; `N-mM` at member `M`; a trailing `-bonus` pays with
/// the bonus action.
fn parse_cast(rest: &str) -> Option<CombatCommand> {
    let (rest, pay) = match rest.strip_suffix("-bonus") {
        Some(rest) => (rest, Pay::BonusAction),
        None => (rest, Pay::Action),
    };
    let (spell, target) = rest.split_once('-')?;
    let spell = spell.parse().ok()?;
    let target = match target.strip_prefix('m') {
        Some(member) => Target::Member(member.parse().ok()?),
        None => Target::Stack(target.parse().ok()?),
    };
    Some(CombatCommand::Cast { spell, target, pay })
}

/// `M-on` or `M-off` switches member `M`'s reactions.
fn parse_react(rest: &str) -> Option<PartyCommand> {
    let (member, on) = match rest.split_once('-')? {
        (member, "on") => (member, true),
        (member, "off") => (member, false),
        _ => return None,
    };
    Some(PartyCommand::Tactics(TacticsCommand::SetReactions {
        member: member.parse().ok()?,
        on,
    }))
}

/// `F` uses feature row `F`; `F-W` exchanges with slot `W` (Cunning Action); `F-hide` hides.
fn parse_feature(rest: &str) -> Option<CombatCommand> {
    let (feature, choice) = match rest.split_once('-') {
        Some((feature, "hide")) => (feature, FeatureChoice::Hide),
        Some((feature, with)) => (
            feature,
            FeatureChoice::Exchange {
                with: with.parse().ok()?,
            },
        ),
        None => (rest, FeatureChoice::None),
    };
    Some(CombatCommand::Feature {
        feature: feature.parse().ok()?,
        choice,
    })
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

/// Town and rest words that carry numbers: `food-N`, `heal-M`, `cure-M`, `raise-M`, `buy-R`
/// and `buy-R-N` (row `R` of the stock), `sell-R` and `sell-R-N` (row `R` of the stores),
/// `deposit-N` and `withdraw-N` (copper), `train-M`, `choose-M-R` (row `R` of the member's
/// class list), `learn-M-R` (row `R` of the service's spells), `short-rest-A-B-…` (hit dice per
/// member in order).
fn parse_town(word: &str) -> Option<Command> {
    if let Some(dice) = word.strip_prefix("short-rest-") {
        let dice = dice
            .split('-')
            .map(|d| d.parse().ok())
            .collect::<Option<Vec<u8>>>()?;
        return Some(Command::Rest(RestCommand::Short { dice }));
    }
    let (verb, args) = word.split_once('-')?;
    let service = match verb {
        "food" => ServiceCommand::BuyFood {
            count: args.parse().ok()?,
        },
        "heal" => ServiceCommand::Heal {
            member: args.parse().ok()?,
        },
        "cure" => ServiceCommand::Cure {
            member: args.parse().ok()?,
        },
        "raise" => ServiceCommand::Raise {
            member: args.parse().ok()?,
        },
        "buy" | "sell" => {
            let (item, count) = match args.split_once('-') {
                Some((item, count)) => (item.parse().ok()?, count.parse().ok()?),
                None => (args.parse().ok()?, 1),
            };
            if verb == "buy" {
                ServiceCommand::Buy { item, count }
            } else {
                ServiceCommand::Sell { item, count }
            }
        }
        "deposit" => ServiceCommand::Deposit {
            amount: args.parse().ok()?,
        },
        "withdraw" => ServiceCommand::Withdraw {
            amount: args.parse().ok()?,
        },
        "train" => ServiceCommand::Train {
            member: args.parse().ok()?,
        },
        "choose" | "learn" => {
            let (member, spell) = args.split_once('-')?;
            let (member, spell) = (member.parse().ok()?, spell.parse().ok()?);
            if verb == "choose" {
                ServiceCommand::Choose { member, spell }
            } else {
                ServiceCommand::Learn { member, spell }
            }
        }
        _ => return None,
    };
    Some(Command::Service(service))
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

impl core::error::Error for ScriptError {}

/// Parse a command script: words `forward`, `back`, `left`, `right` (sidesteps),
/// `turn-left`, `turn-right`, `around`, `use`, before a fight `fight`, `bribe`, `hide`, `run`,
/// and in one `attack` (the first stack), `attack-N`, `cast-N-M` (spell `N` at stack `M`),
/// `cast-N-mM` (at member `M`), either with `-bonus` to pay with the bonus action,
/// `use-item-N` (item `N` of the acting member's kit, on themselves), `use-item-N-mM` (on
/// member `M`), `dodge`, `swap-N`, `flee`, `feature-F` (feature row `F`), `feature-F-W`
/// (Cunning Action's exchange with slot `W`), `feature-F-hide`, `end` (ends the turn), at any
/// time `react-M-on` and `react-M-off` (member `M`'s reactions switch), inside a service
/// `leave`, `room`, `rumor` and the words with numbers of `parse_town` (`buy-0`, `heal-1`, …),
/// outside one `rest`, `short-rest` and `short-rest-A-B-…`, separated by whitespace or commas;
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
        /// The price in copper.
        cost: u32,
        /// The purse in copper.
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
    /// This service does not do that, or does not stock that row.
    NotOffered,
    /// The member has nothing a temple could treat: full hit points, or no condition to cure.
    NothingToTreat {
        /// The slot asked for.
        index: u8,
    },
    /// Only the dead are raised.
    NotDead {
        /// The slot asked for.
        index: u8,
    },
    /// The bank holds less than the withdrawal.
    BankShort {
        /// The withdrawal in copper.
        amount: u32,
        /// The balance in copper.
        bank: u32,
    },
    /// The last long rest ended too recently for another (SRD: one in 24 hours).
    RestTooSoon {
        /// Party-clock minutes until one is allowed.
        minutes: u32,
    },
    /// The stores hold less food than a long rest eats.
    NoFood {
        /// Food the rest eats.
        need: u32,
        /// Food in the stores.
        have: u32,
    },
    /// A member has fewer hit dice left than asked to spend.
    NoHitDice {
        /// The slot asked for.
        index: u8,
        /// Hit dice the member has left.
        left: u8,
    },
    /// The member's experience has not reached their next level.
    NotReady {
        /// The slot asked for.
        index: u8,
        /// Experience held.
        xp: u32,
        /// Experience the next level needs.
        needed: u32,
    },
    /// The member is at the highest level.
    MaxLevel {
        /// The slot asked for.
        index: u8,
    },
    /// The member has no spell picks left to choose.
    NoPicks {
        /// The slot asked for.
        index: u8,
    },
    /// No such row on the list the command names (the class's spells, the service's stock).
    NoSuchSpell {
        /// The row asked for.
        row: u8,
    },
    /// The spell is not on the member's class list.
    NotOnList {
        /// The slot asked for.
        index: u8,
    },
    /// Cantrips come with the class; none is picked or bought.
    CantripNotLearned,
    /// The spell is above the highest level the member may learn.
    SpellTooHigh {
        /// The spell's level.
        level: u8,
        /// The highest the member may learn.
        max: u8,
    },
    /// The spell is already on the member's list.
    AlreadyKnown {
        /// The slot asked for.
        index: u8,
    },
    /// The turn's action is spent (ARCHITECTURE.md §4.7).
    NoActionLeft,
    /// The turn's bonus action is spent.
    NoBonusActionLeft,
    /// That costs a reaction: only a declared reaction pays for it.
    ReactionOnly,
    /// The spell cannot be paid with the bonus action (D24).
    NotABonusAction {
        /// The known-spell row.
        spell: u8,
    },
    /// The spell takes the bonus action only once readied, and readying is not built (D24).
    NeedsPreparation {
        /// The known-spell row.
        spell: u8,
    },
    /// The member has no feature with effect at that row.
    NoSuchFeature {
        /// The row asked for.
        feature: u8,
    },
    /// The feature's uses are spent until a rest.
    NoUsesLeft {
        /// The row asked for.
        feature: u8,
    },
    /// The feature does not do what was asked (a choice it has not, or none where it needs one).
    WrongChoice {
        /// The row asked for.
        feature: u8,
    },
    /// The tactics' shape is refused (a name, a tree, a cap).
    Tactics(TacticsFault),
    /// The member has no such reaction, or it cannot answer that trigger.
    CannotReact,
    /// The default runbook has no entry there.
    NoSuchEntry {
        /// The entry asked for.
        at: u8,
    },
    /// A rule formula failed while resolving: bad pack data, reported rather than a panic.
    Rule(RuleError),
}

impl core::fmt::Display for Rejection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_play(f)
            .or_else(|| self.fmt_items(f))
            .or_else(|| self.fmt_town(f))
            .unwrap_or_else(|| self.fmt_magic(f))
    }
}

impl core::error::Error for Rejection {}

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
            Rejection::CannotAfford { cost, gold } => write!(
                f,
                "that costs {}; the party has {}",
                Coins::of(*cost),
                Coins::of(*gold)
            ),
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

    /// The wording of the service refusals; `None` for the rest.
    fn fmt_town(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Rejection::NotOffered => f.write_str("this service does not offer that"),
            Rejection::NothingToTreat { index } => {
                write!(f, "the member in slot {index} needs no treatment")
            }
            Rejection::NotDead { index } => write!(f, "the member in slot {index} is not dead"),
            Rejection::BankShort { amount, bank } => write!(
                f,
                "that withdraws {}; the bank holds {}",
                Coins::of(*amount),
                Coins::of(*bank)
            ),
            Rejection::RestTooSoon { minutes } => {
                write!(f, "the party rested too recently; {minutes} minutes to go")
            }
            Rejection::NoFood { need, have } => {
                write!(f, "the rest eats {need} food; the stores hold {have}")
            }
            Rejection::NoHitDice { index, left } => {
                write!(f, "the member in slot {index} has {left} hit dice left")
            }
            _ => return self.fmt_level(f),
        })
    }

    /// The wording of the trainer's and the spell sellers' refusals; `None` for the rest.
    fn fmt_level(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Rejection::NotReady { index, xp, needed } => write!(
                f,
                "the member in slot {index} has {xp} experience; the next level needs {needed}"
            ),
            Rejection::MaxLevel { index } => {
                write!(f, "the member in slot {index} is at the highest level")
            }
            Rejection::NoPicks { index } => {
                write!(f, "the member in slot {index} has no spell picks left")
            }
            Rejection::NoSuchSpell { row } => write!(f, "there is no spell in row {row}"),
            Rejection::NotOnList { index } => {
                write!(
                    f,
                    "that spell is not on the list of the member in slot {index}"
                )
            }
            Rejection::CantripNotLearned => f.write_str("cantrips come with the class"),
            Rejection::SpellTooHigh { level, max } => write!(
                f,
                "that is a level {level} spell; the member may learn up to level {max}"
            ),
            Rejection::AlreadyKnown { index } => {
                write!(f, "the member in slot {index} already knows that spell")
            }
            _ => return self.fmt_turn(f),
        })
    }

    /// The wording of the turn budget's and the class features' refusals; `None` for the rest.
    fn fmt_turn(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Rejection::NoActionLeft => f.write_str("the turn's action is spent"),
            Rejection::NoBonusActionLeft => f.write_str("the turn's bonus action is spent"),
            Rejection::ReactionOnly => f.write_str("that is a reaction; declare it in tactics"),
            Rejection::NotABonusAction { spell } => {
                write!(f, "spell {spell} cannot be cast with the bonus action")
            }
            Rejection::NeedsPreparation { spell } => {
                write!(f, "spell {spell} takes the bonus action only once readied")
            }
            Rejection::NoSuchFeature { feature } => {
                write!(f, "there is no feature in row {feature}")
            }
            Rejection::NoUsesLeft { feature } => {
                write!(f, "feature {feature} has no uses left until a rest")
            }
            Rejection::WrongChoice { feature } => {
                write!(f, "feature {feature} does not do that")
            }
            Rejection::Tactics(fault) => write!(f, "tactics refused: {fault:?}"),
            Rejection::CannotReact => {
                f.write_str("the member has no such reaction, or it cannot answer that")
            }
            Rejection::NoSuchEntry { at } => write!(f, "the runbook has no entry {at}"),
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
