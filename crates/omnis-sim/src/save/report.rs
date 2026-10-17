//! `split_report`: how big each part of a world is, and whether it joins back (M8.5 7b).

use super::{join, split};
use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::ron_io::to_string;
use omnis_data::{Data, DataError};
use serde::{Deserialize, Serialize};

/// One region's base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionSize {
    /// `pack:region:name`.
    pub id: String,
    /// Bytes of its RON.
    pub bytes: usize,
    /// How many maps it holds a state for.
    pub maps: usize,
}

/// The parts of a world by size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitReport {
    /// Bytes of the world base's RON.
    pub world_bytes: usize,
    /// Bytes of the checkpoint's RON.
    pub checkpoint_bytes: usize,
    /// Each region with a base, in registry order.
    pub regions: Vec<RegionSize>,
    /// Whether the parts join back into this world, to the fingerprint.
    pub round_trip: bool,
}

/// Split `world`, size its parts as RON, and join them back to check the round trip.
pub fn split_report(world: &World, data: &Data) -> Result<SplitReport, DataError> {
    let parts = split(world, data);
    let mut regions = Vec::new();
    for (id, base) in &parts.regions {
        regions.push(RegionSize {
            id: data
                .registry
                .regions
                .name(*id)
                .map_or_else(|| alloc::format!("{id:?}"), String::from),
            bytes: to_string(base)?.len(),
            maps: base.maps.len(),
        });
    }
    let world_bytes = to_string(&parts.world)?.len();
    let checkpoint_bytes = to_string(&parts.checkpoint)?.len();
    let mut kept = world.clone();
    kept.log.clear();
    kept.journal = crate::journal::Journal::default();
    let round_trip = match join(parts, data, true) {
        Ok(back) => back == kept && back.fingerprint()? == world.fingerprint()?,
        Err(_) => false,
    };
    Ok(SplitReport {
        world_bytes,
        checkpoint_bytes,
        regions,
        round_trip,
    })
}
