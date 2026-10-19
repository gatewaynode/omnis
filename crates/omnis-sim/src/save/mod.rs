//! The world cut into the parts the save model keeps apart (PRD D28, M8.5 7b): what the packs and
//! the seed name (the manifest), what belongs to no region (the world base), what each touched
//! region holds (its base), and the party at this moment (the checkpoint). `split` and `join` are
//! pure and inverse: `join(split(w))` is `w` but for the log and the journal, which a save does
//! not keep in these parts. Registry numbers are kept until saves name things by id (7h).

mod character;
mod check;
mod disk;
mod fold;
mod join;
mod names;
mod places;
pub(crate) mod remap;
mod report;
pub(crate) mod rows;
mod split;
mod tidy;

pub use character::{BaseNames, CharacterSave, is_world_id, world_id};
pub use check::{
    Examined, PackChange, PackStamp, SaveCheck, Verdict, examine, examine_running, restore,
};
pub use disk::{FileOut, PackFit, Restored, WORLDS, archive_path, load_save, save_files};
pub(crate) use fold::fold_from;
pub use fold::{Archived, FOLD_INTERACTIONS, FOLD_RULE, Segment, due, fold, fold_at};
pub use join::{JoinError, join};
pub use names::{NameTable, Signature};
pub use places::{PlaceFix, PlaceWarning, Spot};
pub use remap::{Missing, NameKind};
pub use report::{RegionSize, SplitReport, split_report};
pub(crate) use rows::{row_or_id, rows_or_ids};
pub use split::split;
pub(crate) use split::{Owner, owner_of, stream_owner};
pub use tidy::{tidy_list, unnamed};

use crate::bus::Bus;
use crate::objects::Knowledge;
use crate::party::Party;
use crate::time::PartyTime;
use crate::world::{Automap, MapState, Mode, Settings};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use omnis_core::{Clock, Contact, FlagId, HolderId, MapId, Pcg32, Position, RegionId, StreamName};
use omnis_data::PackFingerprint;
use serde::{Deserialize, Serialize};

/// What a world is made from: its seed and its packs, and what its files' numbers stand for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The world seed every stream derives from (A14).
    pub seed: u64,
    /// The packs, in load order.
    pub packs: Vec<PackFingerprint>,
    /// The string ids behind the numbers (M8.5 7h2); left out before, so an older world's
    /// folder keeps its name.
    #[serde(default, skip_serializing_if = "NameTable::is_empty")]
    pub names: NameTable,
}

impl Manifest {
    /// The manifest of `world` on the loaded packs.
    #[must_use]
    pub fn of(world: &crate::world::World, data: &omnis_data::Data) -> Manifest {
        Manifest {
            seed: world.seed,
            packs: world.packs.clone(),
            names: NameTable::of(data),
        }
    }
}

/// Holders' time and draws: clocks, the contacts kept on the holder's side, and the streams.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holdings {
    /// Clocks by holder.
    pub clocks: BTreeMap<HolderId, Clock>,
    /// Contacts keyed (self, other), as the world keeps them.
    pub contacts: BTreeMap<(HolderId, HolderId), Contact>,
    /// Random streams by name.
    pub streams: BTreeMap<StreamName, Pcg32>,
}

impl Holdings {
    /// Whether nothing is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clocks.is_empty() && self.contacts.is_empty() && self.streams.is_empty()
    }
}

/// One region as play has left it: its maps' states, its clock, its contacts and its streams.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionBase {
    /// The state of each of its maps play has touched, empty ones included.
    pub maps: BTreeMap<MapId, MapState>,
    /// The region's clock, contacts and streams.
    pub holdings: Holdings,
}

impl RegionBase {
    /// Whether play has left nothing here: the region is as its packs wrote it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.maps.is_empty() && self.holdings.is_empty()
    }
}

/// What belongs to no region and to no party: the world's flags (every flag no pack declares
/// the party's), holders that are neither (actors, characters apart, projects), and the state of
/// maps no loaded pack has.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldBase {
    /// The world's flags.
    pub flags: BTreeMap<FlagId, i64>,
    /// Other holders' clocks, contacts and streams.
    pub holdings: Holdings,
    /// Map states whose map no loaded pack has.
    pub strays: BTreeMap<MapId, MapState>,
}

impl WorldBase {
    /// Whether the world is as its packs wrote it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty() && self.holdings.is_empty() && self.strays.is_empty()
    }
}

/// The party at one moment: everything it carries from place to place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// The members and their stores.
    pub party: Party,
    /// The party's shared time and the date it believes.
    pub party_time: PartyTime,
    /// The flags a pack declares the party's: a quest taken, an oath, a debt (M8.5 7c).
    pub flags: BTreeMap<FlagId, i64>,
    /// The party's clock, its contacts, and the streams no region or other holder names.
    pub holdings: Holdings,
    /// The signal bus's subscriptions.
    pub bus: Bus,
    /// Where the party is.
    pub position: Position,
    /// What the party knows of the maps.
    pub automap: Automap,
    /// The hidden things it has found and the lore it has been told.
    #[serde(default, skip_serializing_if = "Knowledge::is_empty")]
    pub known: Knowledge,
    /// What the party is doing.
    pub mode: Mode,
    /// The difficulty options.
    pub settings: Settings,
    /// Commands applied so far.
    pub turn: u64,
}

/// What the world holds at a base: the world's part and each region's. The journal's changes
/// lie over them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bases {
    /// What belongs to no region.
    pub world: WorldBase,
    /// Each region play has touched.
    pub regions: BTreeMap<RegionId, RegionBase>,
}

impl From<Split> for Bases {
    fn from(split: Split) -> Bases {
        Bases {
            world: split.world,
            regions: split.regions,
        }
    }
}

/// A world in its parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Split {
    /// Seed and packs.
    pub manifest: Manifest,
    /// What belongs to no region.
    pub world: WorldBase,
    /// Each region play has touched; a region with nothing changed has none.
    pub regions: BTreeMap<RegionId, RegionBase>,
    /// The party.
    pub checkpoint: Checkpoint,
}
