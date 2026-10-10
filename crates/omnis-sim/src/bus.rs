//! The signal bus's vocabulary (ARCHITECTURE.md §4.8, A16): what a signal can be about, who
//! listens, and what happened. The mechanism (saved subscriptions in call order, synchronous
//! delivery, the depth and budget limits) is the `omnis-bus` crate; a host here is the
//! simulation at a moment (time's `Sim`, combat's `Fight`), matching each subscriber and signal
//! to a handler.

use omnis_core::RegionId;
use serde::{Deserialize, Serialize};

pub use omnis_bus::{Host, MAX_DEPTH, MAX_SIGNALS, drain};

/// The subscriptions, by topic, in call order. Saved with the world.
pub type Bus = omnis_bus::Bus<Topic, Subscriber>;

/// What a signal is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Topic {
    /// A region: the party entered it.
    Region(RegionId),
    /// The fight under way.
    Battle,
}

/// Who listens. Each is a handler the simulation matches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Subscriber {
    /// Subjective time: reconcile the region with the party (§4.4).
    Reconcile,
    /// Declared reactions answer a combat trigger (§4.7).
    Reactions,
}

/// What happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// The party entered `region`, from `from` (none at the start of a game).
    Entered {
        /// The region entered.
        region: RegionId,
        /// The region left.
        from: Option<RegionId>,
    },
    /// A moment in the fight that declared reactions may answer.
    Battle(Cue),
}

/// A moment in the fight (ARCHITECTURE.md §4.7), by the members' marching-order slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// A monster's attack on `subject` is about to be judged.
    Attack {
        /// The member attacked.
        subject: usize,
    },
    /// A monster's Magic Missile is cast at `subject`.
    Missile {
        /// The member targeted.
        subject: usize,
    },
    /// A monster cast a spell.
    EnemyCast,
    /// `subject` took damage, and fell when `dying`.
    Wound {
        /// The member wounded.
        subject: usize,
        /// Whether they fell.
        dying: bool,
    },
    /// The member `caster` cast a spell.
    Cast {
        /// The member who cast.
        caster: usize,
    },
}

impl omnis_bus::Signal for Signal {
    type Topic = Topic;

    fn topic(&self) -> Topic {
        match self {
            Signal::Entered { region, .. } => Topic::Region(*region),
            Signal::Battle(_) => Topic::Battle,
        }
    }
}
