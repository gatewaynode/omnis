//! Save migrations, one function per schema step (ARCHITECTURE.md §6.1). A save from build N
//! loads in build N+1 (PRD §7.7): the old shape is read as written, then converted.

use crate::event::Event;
use crate::party::Party;
use crate::world::{Automap, MapState, Mode, SAVE_SCHEMA, SaveRule, Settings, World};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use omnis_core::{Clock, FlagId, HolderId, MapId, Pcg32, Position, StreamName};
use omnis_data::PackFingerprint;
use serde::Deserialize;

/// Schema 1 settings: the save rule was a single switch.
#[derive(Deserialize)]
struct SettingsV1 {
    save_anywhere: bool,
}

/// Schema 1 world: no party.
#[derive(Deserialize)]
pub(crate) struct WorldV1 {
    seed: u64,
    packs: Vec<PackFingerprint>,
    rngs: BTreeMap<StreamName, Pcg32>,
    clocks: BTreeMap<HolderId, Clock>,
    position: Position,
    maps: BTreeMap<MapId, MapState>,
    automap: Automap,
    mode: Mode,
    flags: BTreeMap<FlagId, i64>,
    settings: SettingsV1,
    turn: u64,
}

/// Schema 1 to 2: an empty party, and the save switch becomes the three-level rule.
pub(crate) fn v1_to_v2(old: WorldV1) -> World {
    World {
        schema: SAVE_SCHEMA,
        seed: old.seed,
        packs: old.packs,
        rngs: old.rngs,
        clocks: old.clocks,
        position: old.position,
        maps: old.maps,
        automap: old.automap,
        mode: old.mode,
        flags: old.flags,
        settings: Settings {
            save_rule: if old.settings.save_anywhere {
                SaveRule::Anywhere
            } else {
                SaveRule::InnOnly
            },
            permadeath: false,
        },
        party: Party::default(),
        turn: old.turn,
        log: Vec::<Event>::new(),
    }
}

/// Schema 2 to 3: the mode may be an encounter or a fight, map states remember cleared
/// encounters, members carry death saves. Every new field has a default, so a schema-2 text
/// reads as a `World` as is; only the number changes.
pub(crate) fn v2_to_v3(mut world: World) -> World {
    world.schema = SAVE_SCHEMA;
    world
}
