//! Objects standing in cells (M8.5 step 2): what the save remembers of each placement, the state
//! it is in now, and whether its footprint blocks a step. Placements and kinds come from data
//! (`omnis_data::placement`); a map whose objects were never touched saves nothing new.

mod act;
mod outcome;

pub(crate) use act::act;

use crate::world::{MapState, World};
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{Facing, ItemId, Level, MapId};
use omnis_data::footprint::Footprint;
use omnis_data::placement::{ResolvedPlacement, in_state};
use omnis_data::{Data, School};
use serde::{Deserialize, Serialize};

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(value: &bool) -> bool {
    !*value
}

/// What the save remembers of one placement; every field left out holds its default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectState {
    /// The state it is in, when changed from where it started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Taken away for good.
    #[serde(default, skip_serializing_if = "is_false")]
    pub removed: bool,
    /// Interactions marked `once` that have succeeded, by index in the kind.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub used: BTreeSet<u8>,
    /// A hidden object the party has found.
    #[serde(default, skip_serializing_if = "is_false")]
    pub noticed: bool,
    /// A container's items, once changed from what it started with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<(ItemId, u16)>>,
}

/// Why an object does not allow what was asked of it now. Checks are not refusals: a failed
/// roll spends the time and says the kind's failure text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    /// The kind offers no such interaction in the state the object is in.
    NotOffered,
    /// It succeeds once, and it has.
    Spent,
    /// The party carries none of an item it needs, or none to spare when one is used up.
    Lacks {
        /// The item's id.
        item: String,
    },
    /// A world flag is below what it needs.
    Flag {
        /// The flag's id.
        flag: String,
    },
    /// Nobody who can act knows a spell of the school.
    School {
        /// The school.
        school: School,
    },
    /// The kind's rules slot answers no.
    Rule {
        /// The slot's name.
        rule: String,
    },
    /// Nobody in the party can act.
    NoneCanAct,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::NotOffered => f.write_str("the object does not allow that now"),
            Refusal::Spent => f.write_str("that has been done already"),
            Refusal::Lacks { item } => write!(f, "that needs {item}"),
            Refusal::Flag { flag } => write!(f, "that needs {flag} first"),
            Refusal::School { school } => {
                write!(f, "that needs someone who knows {school:?} magic")
            }
            Refusal::Rule { rule } => write!(f, "{rule} does not allow that"),
            Refusal::NoneCanAct => f.write_str("nobody in the party can act"),
        }
    }
}

/// The saved state of a placement, if the save holds any.
#[must_use]
pub fn saved<'w>(world: &'w World, map: MapId, id: &str) -> Option<&'w ObjectState> {
    world.maps.get(&map).and_then(|s| s.objects.get(id))
}

/// The state a placement is in now, read from its map's saved state.
#[must_use]
pub fn state_in<'a>(state: Option<&'a MapState>, p: &'a ResolvedPlacement) -> &'a str {
    state
        .and_then(|s| s.objects.get(&p.id))
        .and_then(|s| s.state.as_deref())
        .unwrap_or(&p.state)
}

/// Whether a placement is in sight of the party: not hidden, or found.
#[must_use]
pub fn found_in(state: Option<&MapState>, p: &ResolvedPlacement) -> bool {
    p.hidden.is_none()
        || state
            .and_then(|s| s.objects.get(&p.id))
            .is_some_and(|s| s.noticed)
}

/// Change what the save remembers of a placement in the party's map; an entry back at its
/// defaults is dropped, so a map whose objects are as they started saves nothing.
pub(crate) fn edit(world: &mut World, map: MapId, id: &str, change: impl FnOnce(&mut ObjectState)) {
    let objects = &mut world.map_state(map).objects;
    let entry = objects.entry(id.into()).or_default();
    change(entry);
    if *entry == ObjectState::default() {
        objects.remove(id);
    }
}

/// Whether a placement has been taken away, read from its map's saved state.
#[must_use]
pub fn removed_in(state: Option<&MapState>, p: &ResolvedPlacement) -> bool {
    state
        .and_then(|s| s.objects.get(&p.id))
        .is_some_and(|s| s.removed)
}

/// The state a placement is in now: the saved one, else the one it started in.
#[must_use]
pub fn state_of<'a>(world: &'a World, map: MapId, p: &'a ResolvedPlacement) -> &'a str {
    saved(world, map, &p.id)
        .and_then(|s| s.state.as_deref())
        .unwrap_or(&p.state)
}

/// Whether a placement has been taken away.
#[must_use]
pub fn removed(world: &World, map: MapId, p: &ResolvedPlacement) -> bool {
    saved(world, map, &p.id).is_some_and(|s| s.removed)
}

/// Whether an object's footprint, in its state now, stands in the path of a step from `(x, y)`
/// toward `heading` on `level`: the path runs centre to centre, half a cell wide, and the
/// objects of both cells count.
#[must_use]
pub fn blocks_step(
    world: &World,
    data: &Data,
    map_id: MapId,
    level: Level,
    (x, y): (u16, u16),
    heading: Facing,
) -> bool {
    let Some(map) = data.maps.get(&map_id) else {
        return false;
    };
    if map.objects.is_empty() {
        return false;
    }
    let path = Footprint::path(x, y, heading);
    let next = match heading {
        Facing::North => y.checked_sub(1).map(|y| (x, y)),
        Facing::South => y.checked_add(1).map(|y| (x, y)),
        Facing::East => x.checked_add(1).map(|x| (x, y)),
        Facing::West => x.checked_sub(1).map(|x| (x, y)),
    };
    let cells = core::iter::once((x, y)).chain(next);
    cells
        .flat_map(|(cx, cy)| map.objects_at(level, cx, cy))
        .any(|(_, p)| blocks(world, data, map_id, p, &path))
}

fn blocks(
    world: &World,
    data: &Data,
    map_id: MapId,
    p: &ResolvedPlacement,
    path: &Footprint,
) -> bool {
    let Some(kind) = data.objects.get(&p.kind) else {
        return false;
    };
    if removed(world, map_id, p) {
        return false;
    }
    let (look, moves, _) = in_state(kind, state_of(world, map_id, p));
    moves && p.footprint(look).overlaps(path)
}
