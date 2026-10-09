//! What a client may ask the simulation to do, and why it may refuse (ARCHITECTURE.md §4.2).
//! Commands carry no client state so a command stream is a replay and, later, a network
//! protocol. What happened is `event::Event`.

use crate::combat::{CombatCommand, Target};
use crate::dev::DevCommand;
use crate::encounter::EncounterChoice;
use crate::items::ItemCommand;
use crate::party::PartyCommand;
use crate::rest::RestCommand;
use crate::service::ServiceCommand;
use alloc::string::String;
use core::fmt;
use omnis_core::{CharacterId, Coins, Direction, ItemId, Rotation};
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
        /// The caster.
        caster: CharacterId,
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
    /// The script word for this command; see [`crate::word::parse_script`]. Party commands carry data and
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
        /// The member asked for.
        member: CharacterId,
    },
    /// A member cannot exchange with themselves.
    SameMember,
    /// A list names the same member twice (a short rest's hit dice).
    MemberTwice {
        /// The member named twice.
        member: CharacterId,
    },
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
        /// The member asked for.
        member: CharacterId,
    },
    /// The member is at zero hit points and cannot act.
    MemberDown {
        /// The member asked for.
        member: CharacterId,
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
        /// The member asked for.
        member: CharacterId,
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
        /// The member asked for.
        member: CharacterId,
    },
    /// Only the dead are raised.
    NotDead {
        /// The member asked for.
        member: CharacterId,
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
        /// The member asked for.
        member: CharacterId,
        /// Hit dice the member has left.
        left: u8,
    },
    /// The member's experience has not reached their next level.
    NotReady {
        /// The member asked for.
        member: CharacterId,
        /// Experience held.
        xp: u32,
        /// Experience the next level needs.
        needed: u32,
    },
    /// The member is at the highest level.
    MaxLevel {
        /// The member asked for.
        member: CharacterId,
    },
    /// The member has no spell picks left to choose.
    NoPicks {
        /// The member asked for.
        member: CharacterId,
    },
    /// No such row on the list the command names (the class's spells, the service's stock).
    NoSuchSpell {
        /// The row asked for.
        row: u8,
    },
    /// The spell is not on the member's class list.
    NotOnList {
        /// The member asked for.
        member: CharacterId,
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
        /// The member asked for.
        member: CharacterId,
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
            Rejection::NoSuchMember {
                member: CharacterId(member),
            } => write!(f, "there is no member {member}"),
            Rejection::MemberTwice {
                member: CharacterId(member),
            } => write!(f, "member {member} is named twice"),
            Rejection::SameMember => f.write_str("a member cannot exchange with themselves"),
            Rejection::CannotAfford { cost, gold } => write!(
                f,
                "that costs {}; the party has {}",
                Coins::of(*cost),
                Coins::of(*gold)
            ),
            Rejection::MemberDead {
                member: CharacterId(member),
            } => write!(f, "the member {member} is dead"),
            Rejection::MemberDown {
                member: CharacterId(member),
            } => write!(f, "the member {member} is down"),
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
            Rejection::TargetDead {
                member: CharacterId(member),
            } => write!(f, "the member {member} is dead"),
            Rejection::ZeroCount => f.write_str("a count of zero moves nothing"),
            _ => return None,
        })
    }

    /// The wording of the service refusals; `None` for the rest.
    fn fmt_town(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Rejection::NotOffered => f.write_str("this service does not offer that"),
            Rejection::NothingToTreat {
                member: CharacterId(member),
            } => {
                write!(f, "the member {member} needs no treatment")
            }
            Rejection::NotDead {
                member: CharacterId(member),
            } => write!(f, "the member {member} is not dead"),
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
            Rejection::NoHitDice {
                member: CharacterId(member),
                left,
            } => {
                write!(f, "the member {member} has {left} hit dice left")
            }
            _ => return self.fmt_level(f),
        })
    }

    /// The wording of the trainer's and the spell sellers' refusals; `None` for the rest.
    fn fmt_level(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        Some(match self {
            Rejection::NotReady {
                member: CharacterId(member),
                xp,
                needed,
            } => write!(
                f,
                "the member {member} has {xp} experience; the next level needs {needed}"
            ),
            Rejection::MaxLevel {
                member: CharacterId(member),
            } => {
                write!(f, "the member {member} is at the highest level")
            }
            Rejection::NoPicks {
                member: CharacterId(member),
            } => {
                write!(f, "the member {member} has no spell picks left")
            }
            Rejection::NoSuchSpell { row } => write!(f, "there is no spell in row {row}"),
            Rejection::NotOnList {
                member: CharacterId(member),
            } => {
                write!(f, "that spell is not on the list of the member {member}")
            }
            Rejection::CantripNotLearned => f.write_str("cantrips come with the class"),
            Rejection::SpellTooHigh { level, max } => write!(
                f,
                "that is a level {level} spell; the member may learn up to level {max}"
            ),
            Rejection::AlreadyKnown {
                member: CharacterId(member),
            } => {
                write!(f, "the member {member} already knows that spell")
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
