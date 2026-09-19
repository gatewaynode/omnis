//! What the simulation says happened (ARCHITECTURE.md §4.2). Events carry keys, ids, numbers,
//! and roll traces, never text: a client renders them and a test asserts on them.

use crate::dev::DevCommand;
use crate::encounter::EncounterSource;
use alloc::vec::Vec;
use omnis_core::{
    CharacterId, ConditionId, Facing, HolderId, MapId, MonsterId, Position, RollTrace,
};
use omnis_data::{DamageType, Disposition};
use omnis_rules::{DamageAdjust, DeathSaveResult, Roll};
use serde::{Deserialize, Serialize};

/// Why a step did not happen. Not an error and not a rejection: the turn was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BlockReason {
    /// A wall on the edge.
    Wall,
    /// A closed door on the edge.
    ClosedDoor,
    /// The target terrain cannot be entered.
    Impassable,
    /// The target is off the map or the map is unknown.
    MapEdge,
}

/// A message for the player, named so packs can localize it under `sim:message:<name>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MessageKey {
    /// Interact found nothing.
    NothingHere,
}

impl MessageKey {
    /// The text key clients look up.
    #[must_use]
    pub const fn text_key(self) -> &'static str {
        match self {
            MessageKey::NothingHere => "sim:message:nothing_here",
        }
    }
}

/// A tile the party perceived this turn, in viewport coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeenTile {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Tiles ahead of the party.
    pub depth: u8,
    /// Tiles to the right of the facing line; negative is left.
    pub offset: i8,
}

/// Someone in a fight: a party member by identity (marching order can change mid-fight), a
/// whole stack (it rolls initiative once), or one individual in a stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ActorRef {
    /// A party member.
    Member(CharacterId),
    /// A stack, by its index in the encounter.
    Stack(u8),
    /// One individual: the stack and its index among the living at that moment.
    Monster {
        /// The stack.
        stack: u8,
        /// The individual.
        index: u8,
    },
}

/// Who lost their first round to surprise.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Surprise {
    /// Nobody.
    #[default]
    None,
    /// The party did not notice the monsters.
    Party,
    /// The monsters did not notice the party (reserved; M4 never produces it).
    Monsters,
}

/// How a fight ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CombatOutcome {
    /// Every stack is dead.
    Victory,
    /// The party got away.
    Fled,
    /// Every member is down.
    Defeat,
}

/// Which check an encounter or a fight asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CheckKind {
    /// The monsters' Stealth against the party's passive Perception at the trigger.
    Stealth,
    /// The party's Stealth against the monsters' passive Perception.
    Hide,
    /// The party's Dexterity against the run difficulty, before a fight.
    Run,
    /// The same, from inside a fight.
    Flee,
}

/// What happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// The party moved, possibly to another map through a portal.
    Moved {
        /// Where it was.
        from: Position,
        /// Where it is.
        to: Position,
    },
    /// A step did not happen.
    Blocked {
        /// Why.
        reason: BlockReason,
    },
    /// A holder's clock advanced.
    TimeAdvanced {
        /// Whose clock.
        holder: HolderId,
        /// By how much.
        minutes: u32,
        /// Whether a day boundary was crossed.
        day_rolled: bool,
    },
    /// What the party perceives after the command.
    Visible {
        /// Every visible tile, nearest first.
        tiles: Vec<SeenTile>,
    },
    /// A door changed state.
    Door {
        /// The map.
        map: MapId,
        /// The tile the party stands on.
        x: u16,
        /// The tile the party stands on.
        y: u16,
        /// The edge the door is on.
        facing: Facing,
        /// Its new state.
        open: bool,
    },
    /// A message for the player.
    Message {
        /// Which message.
        key: MessageKey,
    },
    /// The party's members or their order changed.
    PartyChanged,
    /// A step onto a tile of a map with a random table rolled for an encounter.
    EncounterCheck {
        /// The d100.
        roll: RollTrace,
        /// The map's chance.
        chance: u8,
        /// Whether one starts.
        fired: bool,
    },
    /// Monsters stand before the party.
    EncounterStarted {
        /// Where they came from.
        source: EncounterSource,
        /// Monster and count per stack.
        stacks: Vec<(MonsterId, u8)>,
        /// How they feel about the party.
        disposition: Disposition,
        /// The count dice of a random encounter, one per stack.
        counts: Vec<RollTrace>,
        /// The monsters' Stealth roll, none when they are friendly.
        stealth: Option<Roll>,
        /// The party's best passive Perception.
        perception: i64,
        /// Whether the party noticed them; if not, the fight starts with the party surprised.
        noticed: bool,
    },
    /// A check against a difficulty.
    Check {
        /// Who rolled.
        actor: ActorRef,
        /// Which check.
        kind: CheckKind,
        /// The roll, none when the outcome needed no die.
        roll: Option<Roll>,
        /// What it had to reach.
        dc: i64,
        /// Whether it did.
        success: bool,
    },
    /// The monsters took the party's gold and left.
    Bribed {
        /// Gold paid.
        cost: u32,
    },
    /// The fight is on.
    CombatStarted {
        /// Who lost their first round.
        surprised: Surprise,
    },
    /// The order of the fight, highest first.
    Initiative {
        /// Each actor and its total.
        order: Vec<(ActorRef, i64)>,
        /// The dice, in the order they were rolled: members in marching order, then stacks.
        rolls: Vec<RollTrace>,
    },
    /// A round began.
    RoundStarted {
        /// Its number, from one.
        round: u32,
    },
    /// An actor's turn began; for a member, the simulation now waits for a command.
    Turn {
        /// Whose.
        actor: ActorRef,
    },
    /// A stack could do nothing from where it stands.
    Waited {
        /// Which.
        actor: ActorRef,
    },
    /// A member dodges until the round ends.
    Dodging {
        /// Who.
        actor: ActorRef,
    },
    /// Two members swapped marching-order slots.
    Exchanged {
        /// The acting member's slot.
        a: u8,
        /// The other slot.
        b: u8,
    },
    /// An attack roll against an armor class.
    AttackResolved {
        /// Who attacked.
        attacker: ActorRef,
        /// Whom.
        target: ActorRef,
        /// The d20 and its parts.
        roll: Roll,
        /// The armor class.
        ac: i64,
        /// Whether it hit.
        hit: bool,
        /// Whether it was a critical hit.
        crit: bool,
    },
    /// Damage dealt.
    Damage {
        /// Who took it.
        target: ActorRef,
        /// The damage type.
        kind: DamageType,
        /// The dice.
        rolls: Vec<RollTrace>,
        /// Before defenses.
        raw: i64,
        /// After defenses.
        amount: i64,
        /// Which defense applied.
        adjust: DamageAdjust,
    },
    /// Hit points regained, by a potion or a spell.
    Healed {
        /// Who.
        target: CharacterId,
        /// The dice.
        rolls: Vec<RollTrace>,
        /// Points regained before the cap.
        amount: i64,
        /// Hit points after.
        hp: i32,
    },
    /// A member fell to zero hit points.
    Down {
        /// Who.
        target: CharacterId,
    },
    /// Damage to a member already at zero: a failed death save, two for a critical hit.
    Wounded {
        /// Who.
        member: CharacterId,
        /// Failures after it.
        failures: u8,
    },
    /// A death saving throw at the end of a round.
    DeathSave {
        /// Who.
        member: CharacterId,
        /// The d20.
        roll: RollTrace,
        /// What it led to.
        result: DeathSaveResult,
        /// Successes after it.
        successes: u8,
        /// Failures after it.
        failures: u8,
    },
    /// A condition came or went.
    Condition {
        /// Who.
        target: ActorRef,
        /// Which.
        condition: ConditionId,
        /// Applied or removed.
        applied: bool,
    },
    /// A combatant died.
    Death {
        /// Who.
        target: ActorRef,
        /// The gold a monster dropped.
        gold: Option<RollTrace>,
    },
    /// The fight is over.
    CombatEnded {
        /// How.
        outcome: CombatOutcome,
        /// Experience each surviving member gained.
        xp: u32,
        /// Gold the party gained.
        gold: u32,
        /// Members removed by permadeath.
        fallen: Vec<CharacterId>,
    },
    /// A debugging edit was applied; what it caused follows.
    Dev {
        /// The edit.
        command: DevCommand,
    },
}
