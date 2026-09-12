//! Typed identifiers. Data files name things as `pack:type:name` strings; `omnis-data` interns
//! those into these `u32` newtypes at load, so the simulation never compares strings.

use alloc::string::String;
use core::fmt;
use serde::{Deserialize, Serialize};

macro_rules! define_id {
    ($(#[$doc:meta] $name:ident),* $(,)?) => {$(
        #[$doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $name(pub u32);

        impl $name {
            /// The interned index.
            #[must_use]
            pub const fn index(self) -> u32 {
                self.0
            }
        }

        impl From<u32> for $name {
            fn from(index: u32) -> Self {
                $name(index)
            }
        }

        impl From<$name> for u32 {
            fn from(id: $name) -> u32 {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.0)
            }
        }
    )*};
}

define_id! {
    /// A map (one dungeon level, town, or outdoor patch).
    MapId,
    /// A tileset declaring per-depth viewport slots.
    TilesetId,
    /// A region: the temporal holder that owns maps, lairs, and ecosystem state.
    RegionId,
    /// A localized text key; clients render the string from pack `text/`.
    TextKey,
    /// A named integer flag in the world.
    FlagId,
    /// A spell.
    SpellId,
    /// A monster type.
    MonsterId,
    /// An item type.
    ItemId,
    /// A quest.
    QuestId,
    /// A player party (one in v1; each co-op party later).
    PartyId,
    /// A named actor with its own clock (empty in v1 content).
    ActorId,
    /// A character separated from the party who keeps their own clock.
    CharacterId,
    /// A construction or other long-running project with its own clock.
    ProjectId,
    /// An age of the world. v1 content has one era, `present`.
    EraId,
    /// A playable race.
    RaceId,
    /// A character class.
    ClassId,
    /// A character background.
    BackgroundId,
    /// A condition a creature can be under.
    ConditionId,
}

/// Anything that experiences subjective time (ARCHITECTURE.md §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HolderId {
    /// A player party.
    Party(PartyId),
    /// A region.
    Region(RegionId),
    /// A named actor.
    Actor(ActorId),
    /// A separated character.
    Character(CharacterId),
    /// A project.
    Project(ProjectId),
}

impl fmt::Display for HolderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HolderId::Party(id) => write!(f, "party:{}", id.0),
            HolderId::Region(id) => write!(f, "region:{}", id.0),
            HolderId::Actor(id) => write!(f, "actor:{}", id.0),
            HolderId::Character(id) => write!(f, "character:{}", id.0),
            HolderId::Project(id) => write!(f, "project:{}", id.0),
        }
    }
}

/// The canonical name of an RNG stream (ARCHITECTURE.md §11, A14): `party`, `combat`,
/// `time:<a>:<b>`, `eco:<region>`, `story:<region>`, `gen:<x>:<y>:<layer>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StreamName(pub String);

impl StreamName {
    /// A stream name from any string-like value.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The name as bytes, the input to stream derivation.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// The `time:<a>:<b>` stream for a holder pair, with the holders in canonical order so
    /// both sides of a contact derive the same stream.
    #[must_use]
    pub fn time(a: HolderId, b: HolderId) -> Self {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        Self(alloc::format!("time:{lo}:{hi}"))
    }
}

impl fmt::Display for StreamName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for StreamName {
    fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}
