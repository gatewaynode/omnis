//! `join`: a world put back together from its parts (PRD D28, M8.5 7b).

use super::{Holdings, Split};
use crate::journal::Journal;
use crate::world::{LoadError, SAVE_SCHEMA, World};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use omnis_data::Data;

/// Why the parts do not make a world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinError {
    /// Two parts hold the same thing: what it is.
    Duplicate(String),
    /// The joined world does not fit the loaded packs.
    Load(LoadError),
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoinError::Duplicate(what) => write!(f, "two parts of the world both hold {what}"),
            JoinError::Load(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for JoinError {}

/// Insert every entry of `from` into `into`, refusing a key already there.
fn merge<K: Ord + fmt::Debug, V>(
    into: &mut BTreeMap<K, V>,
    from: BTreeMap<K, V>,
    what: &str,
) -> Result<(), JoinError> {
    for (key, value) in from {
        if into.contains_key(&key) {
            return Err(JoinError::Duplicate(format!("{what} {key:?}")));
        }
        into.insert(key, value);
    }
    Ok(())
}

/// One part's clocks, contacts and streams added to the world's.
fn hold(world: &mut World, holdings: Holdings) -> Result<(), JoinError> {
    merge(&mut world.clocks, holdings.clocks, "the clock of")?;
    merge(&mut world.contacts, holdings.contacts, "the contact")?;
    merge(&mut world.rngs, holdings.streams, "the stream")
}

/// The world `split` cut into these parts, checked as a load checks a save (`force` passes
/// other packs, not the structural checks). Its log and journal start empty.
pub fn join(split: Split, data: &Data, force: bool) -> Result<World, JoinError> {
    let Split {
        manifest,
        world: base,
        regions,
        checkpoint,
    } = split;
    let mut world = World {
        schema: SAVE_SCHEMA,
        seed: manifest.seed,
        packs: manifest.packs,
        rngs: BTreeMap::new(),
        clocks: BTreeMap::new(),
        contacts: BTreeMap::new(),
        party_time: checkpoint.party_time,
        bus: checkpoint.bus,
        position: checkpoint.position,
        maps: BTreeMap::new(),
        automap: checkpoint.automap,
        known: checkpoint.known,
        mode: checkpoint.mode,
        flags: checkpoint.flags,
        settings: checkpoint.settings,
        party: checkpoint.party,
        turn: checkpoint.turn,
        log: Vec::new(),
        journal: Journal::default(),
        origin: None,
        archive: Vec::new(),
    };
    merge(&mut world.flags, base.flags, "the flag")?;
    hold(&mut world, checkpoint.holdings)?;
    hold(&mut world, base.holdings)?;
    merge(&mut world.maps, base.strays, "the state of map")?;
    for (_, region) in regions {
        hold(&mut world, region.holdings)?;
        merge(&mut world.maps, region.maps, "the state of map")?;
    }
    world.checked(data, force).map_err(JoinError::Load)
}
