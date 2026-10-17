//! `split`: a world cut into its parts (PRD D28, M8.5 7b).

use super::{Checkpoint, Holdings, Manifest, RegionBase, Split, WorldBase};
use crate::world::World;
use alloc::collections::BTreeMap;
use omnis_core::{HolderId, RegionId, StreamName};
use omnis_data::{Data, FlagScope};

/// Whose a clock, a contact or a stream is.
#[derive(Clone, Copy)]
enum Owner {
    Party,
    Region(RegionId),
    World,
}

/// A holder's owner: a party's is the checkpoint, a region's its base, any other the world.
fn owner_of(holder: HolderId) -> Owner {
    match holder {
        HolderId::Party(_) => Owner::Party,
        HolderId::Region(r) => Owner::Region(r),
        HolderId::Actor(_) | HolderId::Character(_) | HolderId::Project(_) => Owner::World,
    }
}

/// A stream's owner, read from the holders its name carries (`time:party:0:region:3`): any
/// holder that is neither a party nor a region makes it the world's, else the first region
/// named owns it, else the party. A bare name (`combat`) is the party's.
fn stream_owner(name: &StreamName) -> Owner {
    let mut parts = name.0.split(':').skip(1);
    let mut region = None;
    while let (Some(kind), Some(index)) = (parts.next(), parts.next()) {
        match kind {
            "party" => {}
            "region" => match index.parse::<u32>() {
                Ok(n) => {
                    region.get_or_insert(RegionId(n));
                }
                Err(_) => return Owner::World,
            },
            _ => return Owner::World,
        }
    }
    region.map_or(Owner::Party, Owner::Region)
}

/// The parts that receive what is routed.
struct Parts {
    world: WorldBase,
    regions: BTreeMap<RegionId, RegionBase>,
    party: Holdings,
}

impl Parts {
    fn holdings(&mut self, owner: Owner) -> &mut Holdings {
        match owner {
            Owner::Party => &mut self.party,
            Owner::Region(r) => &mut self.regions.entry(r).or_default().holdings,
            Owner::World => &mut self.world.holdings,
        }
    }
}

/// Cut `world` into its manifest, world base, region bases and checkpoint. Each map's state goes
/// to its map's region (an empty one too: it is in the fingerprint), each clock and contact to
/// its holder's part, each stream to the part its name names, each flag to the party or the
/// world as its pack declares (the world's when undeclared). A region play has not touched
/// gets no base. The log and the journal are left out.
#[must_use]
pub fn split(world: &World, data: &Data) -> Split {
    let mut parts = Parts {
        world: WorldBase::default(),
        regions: BTreeMap::new(),
        party: Holdings::default(),
    };
    let mut party_flags = BTreeMap::new();
    for (flag, value) in &world.flags {
        let to = match data.flag_scope(*flag) {
            FlagScope::Party => &mut party_flags,
            FlagScope::World => &mut parts.world.flags,
        };
        to.insert(*flag, *value);
    }
    for (map, state) in &world.maps {
        let target = match data.maps.get(map) {
            Some(loaded) => &mut parts.regions.entry(loaded.region).or_default().maps,
            None => &mut parts.world.strays,
        };
        target.insert(*map, state.clone());
    }
    for (holder, clock) in &world.clocks {
        parts
            .holdings(owner_of(*holder))
            .clocks
            .insert(*holder, *clock);
    }
    for (key, contact) in &world.contacts {
        parts
            .holdings(owner_of(key.0))
            .contacts
            .insert(*key, *contact);
    }
    for (name, stream) in &world.rngs {
        parts
            .holdings(stream_owner(name))
            .streams
            .insert(name.clone(), *stream);
    }
    Split {
        manifest: Manifest {
            seed: world.seed,
            packs: world.packs.clone(),
        },
        world: parts.world,
        regions: parts.regions,
        checkpoint: Checkpoint {
            party: world.party.clone(),
            party_time: world.party_time,
            flags: party_flags,
            holdings: parts.party,
            bus: world.bus.clone(),
            position: world.position,
            automap: world.automap.clone(),
            mode: world.mode.clone(),
            settings: world.settings,
            turn: world.turn,
        },
    }
}
