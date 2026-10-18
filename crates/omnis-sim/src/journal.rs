//! The journal (PRD D28, owner 2026-10-17): the interactions since the world's base, each with
//! the changes it made to the world. The party is saved as it stands, so what changes only the
//! party (a step, a turn, a purchase, a rest) leaves no entry; a command that changed what the
//! world holds (a door, an object, a pile, a cleared encounter, a world flag, a region's time)
//! is one. A load applies the changes to the bases in order and runs no rule.
//!
//! The changes are found by comparing what the world holds before and after each command, not
//! told by each place that writes, so no writer can be missed.

use crate::command::Command;
use crate::objects::ObjectState;
use crate::save::{Owner, owner_of, stream_owner};
use crate::world::{MapState, World};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{
    Clock, Contact, Facing, FlagId, HolderId, Level, MapId, Pcg32, Position, StreamName,
};
use omnis_data::placement::Pile;
use omnis_data::{Data, FlagScope};
use serde::{Deserialize, Serialize};

/// One change to what the world holds, as the value after it, so a change applied twice is
/// applied once. `None` is "as the packs wrote it": gone from the state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    /// A door on a map's own level, by its canonical edge (`door_key`).
    Door {
        /// The map.
        map: MapId,
        /// The edge.
        edge: (u16, u16, Facing),
        /// Open now.
        open: bool,
    },
    /// A door on another of the map's levels.
    LevelDoor {
        /// The map.
        map: MapId,
        /// The level and edge.
        edge: (Level, u16, u16, Facing),
        /// Open now.
        open: bool,
    },
    /// A fixed encounter, by its index in the map file.
    Cleared {
        /// The map.
        map: MapId,
        /// The encounter.
        index: u16,
        /// Cleared for good now.
        cleared: bool,
    },
    /// What the world remembers of a placement now.
    Object {
        /// The map.
        map: MapId,
        /// The placement's id.
        id: String,
        /// Its state; `None` as placed.
        state: Option<ObjectState>,
    },
    /// A pile on the ground now.
    Ground {
        /// The map.
        map: MapId,
        /// Its level and cell.
        at: (Level, u16, u16),
        /// The pile; `None` as the map has it.
        pile: Option<Pile>,
    },
    /// A world flag now.
    Flag {
        /// The flag.
        flag: FlagId,
        /// Its value; `None` unset.
        value: Option<i64>,
    },
    /// A clock the party does not own.
    Clock {
        /// Whose.
        holder: HolderId,
        /// The clock; `None` none yet.
        clock: Option<Clock>,
    },
    /// A contact the party does not keep.
    Contact {
        /// Kept by the first, of the second.
        key: (HolderId, HolderId),
        /// The contact; `None` never met.
        contact: Option<Contact>,
    },
    /// An RNG stream the world or a region owns.
    Stream {
        /// The stream's name.
        name: StreamName,
        /// Its state; `None` never drawn.
        rng: Option<Pcg32>,
    },
}

/// One interaction: the command that made changes to the world, where and when, and the
/// changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The party's age in minutes once the command was done.
    pub minute: i64,
    /// Where the party stood once it was done.
    pub at: Position,
    /// The command.
    pub command: Command,
    /// What it changed in the world, in a fixed order.
    pub changes: Vec<Change>,
}

/// The interactions since the world's base, in the order they happened.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Journal(Vec<Entry>);

impl Journal {
    /// Append an interaction.
    pub fn push(&mut self, entry: Entry) {
        self.0.push(entry);
    }

    /// The entries, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.0
    }

    /// How many entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether nothing has changed the world since the base.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<Entry>> for Journal {
    fn from(entries: Vec<Entry>) -> Journal {
        Journal(entries)
    }
}

/// How many interactions the journal holds: what a load applies.
#[must_use]
pub fn journal_len(world: &World) -> usize {
    world.journal.len()
}

/// What the world holds, as opposed to the party: the map states, the world flags, and the
/// clocks, contacts and streams of every holder but the party.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Held {
    maps: BTreeMap<MapId, MapState>,
    flags: BTreeMap<FlagId, i64>,
    clocks: BTreeMap<HolderId, Clock>,
    contacts: BTreeMap<(HolderId, HolderId), Contact>,
    streams: BTreeMap<StreamName, Pcg32>,
}

impl Held {
    /// What `world` holds now.
    pub(crate) fn of(world: &World, data: &Data) -> Held {
        let not_party = |o: Owner| !matches!(o, Owner::Party);
        Held {
            maps: world.maps.clone(),
            flags: world
                .flags
                .iter()
                .filter(|(f, _)| data.flag_scope(**f) == FlagScope::World)
                .map(|(f, v)| (*f, *v))
                .collect(),
            clocks: world
                .clocks
                .iter()
                .filter(|(h, _)| not_party(owner_of(**h)))
                .map(|(h, c)| (*h, *c))
                .collect(),
            contacts: world
                .contacts
                .iter()
                .filter(|(k, _)| not_party(owner_of(k.0)))
                .map(|(k, c)| (*k, *c))
                .collect(),
            streams: world
                .rngs
                .iter()
                .filter(|(n, _)| not_party(stream_owner(n)))
                .map(|(n, r)| (n.clone(), *r))
                .collect(),
        }
    }

    /// The changes that take `self` to `now`.
    pub(crate) fn changes_to(&self, now: &Held) -> Vec<Change> {
        let mut changes = Vec::new();
        let none = MapState::default();
        let maps = self.maps.keys().chain(now.maps.keys());
        let mut seen: Vec<MapId> = maps.copied().collect();
        seen.sort_unstable();
        seen.dedup();
        for map in seen {
            let was = self.maps.get(&map).unwrap_or(&none);
            let is = now.maps.get(&map).unwrap_or(&none);
            if was != is {
                map_changes(map, was, is, &mut changes);
            }
        }
        diff(&self.flags, &now.flags, |flag, value| {
            changes.push(Change::Flag { flag, value });
        });
        diff(&self.clocks, &now.clocks, |holder, clock| {
            changes.push(Change::Clock { holder, clock });
        });
        diff(&self.contacts, &now.contacts, |key, contact| {
            changes.push(Change::Contact { key, contact });
        });
        diff(&self.streams, &now.streams, |name, rng| {
            changes.push(Change::Stream { name, rng });
        });
        changes
    }
}

/// What changed on one map, field by field.
fn map_changes(map: MapId, was: &MapState, is: &MapState, out: &mut Vec<Change>) {
    for edge in was.open_doors.symmetric_difference(&is.open_doors) {
        let open = is.open_doors.contains(edge);
        out.push(Change::Door {
            map,
            edge: *edge,
            open,
        });
    }
    for edge in was.level_doors.symmetric_difference(&is.level_doors) {
        let open = is.level_doors.contains(edge);
        out.push(Change::LevelDoor {
            map,
            edge: *edge,
            open,
        });
    }
    for index in was.cleared.symmetric_difference(&is.cleared) {
        let cleared = is.cleared.contains(index);
        out.push(Change::Cleared {
            map,
            index: *index,
            cleared,
        });
    }
    diff(&was.objects, &is.objects, |id, state| {
        out.push(Change::Object { map, id, state });
    });
    diff(&was.ground, &is.ground, |at, pile| {
        out.push(Change::Ground { map, at, pile });
    });
}

/// Each key whose value differs between `was` and `is`, with its value in `is`.
fn diff<K: Ord + Clone, V: PartialEq + Clone>(
    was: &BTreeMap<K, V>,
    is: &BTreeMap<K, V>,
    mut each: impl FnMut(K, Option<V>),
) {
    for (key, value) in is {
        if was.get(key) != Some(value) {
            each(key.clone(), Some(value.clone()));
        }
    }
    for key in was.keys() {
        if !is.contains_key(key) {
            each(key.clone(), None);
        }
    }
}

/// Put `value` at `key`, or take it away for `None`.
fn put<K: Ord, V>(map: &mut BTreeMap<K, V>, key: K, value: Option<V>) {
    match value {
        Some(value) => {
            map.insert(key, value);
        }
        None => {
            map.remove(&key);
        }
    }
}

/// Put `key` in `set`, or take it out.
fn mark<K: Ord>(set: &mut BTreeSet<K>, key: K, on: bool) {
    if on {
        set.insert(key);
    } else {
        set.remove(&key);
    }
}

/// Write `changes` into `world`, in order, then drop what is back at its defaults. No rule runs:
/// this is how a load lays the journal over the bases.
pub fn apply_changes(world: &mut World, changes: &[Change]) {
    for change in changes.iter().cloned() {
        match change {
            Change::Door { map, edge, open } => {
                mark(&mut world.map_state(map).open_doors, edge, open);
            }
            Change::LevelDoor { map, edge, open } => {
                mark(&mut world.map_state(map).level_doors, edge, open);
            }
            Change::Cleared {
                map,
                index,
                cleared,
            } => mark(&mut world.map_state(map).cleared, index, cleared),
            Change::Object { map, id, state } => put(&mut world.map_state(map).objects, id, state),
            Change::Ground { map, at, pile } => put(&mut world.map_state(map).ground, at, pile),
            Change::Flag { flag, value } => put(&mut world.flags, flag, value),
            Change::Clock { holder, clock } => put(&mut world.clocks, holder, clock),
            Change::Contact { key, contact } => put(&mut world.contacts, key, contact),
            Change::Stream { name, rng } => put(&mut world.rngs, name, rng),
        }
    }
    world.prune();
}
