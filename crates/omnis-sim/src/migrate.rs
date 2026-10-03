//! Save migrations, one function per schema step (ARCHITECTURE.md §6.1). A save from build N
//! loads in build N+1 (PRD §7.7): the old shape is read as written, then converted.

use crate::event::Event;
use crate::party::Party;
use crate::world::{Automap, MapState, Mode, SAVE_SCHEMA, SaveRule, Settings, World};
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use omnis_core::{Clock, FlagId, HolderId, MapId, Pcg32, Position, StreamName, money};
use omnis_data::{Ability, Data, PackFingerprint};
use omnis_rules::tactics::TACTICS_NAME_BYTES;
use omnis_rules::{ActionRef, Criteria, CriteriaSet, Predicate, Trigger, auto_equip, modifier};
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
        schema: 2,
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
            devtools: false,
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
    world.schema = 3;
    world
}

/// Schema 3 to 4: members wear equipment in slots, carry spell effects and reaction
/// preferences, the party carries effects, and the settings say whether dev commands are
/// accepted. Every field has a default; a member whose slots are empty wears what the old
/// everything-counts rule counted, so the numbers do not change on load.
pub(crate) fn v3_to_v4(mut world: World, data: &Data) -> World {
    for member in &mut world.party.members {
        if member.equipped.is_empty() {
            let dex = modifier(member.scores[Ability::Dexterity.index()]);
            member.equipped = auto_equip(data, &member.equipment, dex);
        }
    }
    world.schema = 4;
    world
}

/// Schema 4 to 5: money is copper, not whole gold (owner, 2026-09-27), so the purse and a
/// fight's unpaid loot are multiplied out; members gain spent hit dice and the party a bank and
/// the time of its last long rest, all defaulting to none.
pub(crate) fn v4_to_v5(mut world: World) -> World {
    world.party.gold = money::from_gp(world.party.gold);
    if let Mode::Combat(fight) = &mut world.mode {
        fight.gold = money::from_gp(fight.gold);
    }
    world.schema = 5;
    world
}

/// Schema 5 to 6: the turn budget and declared reactions (M7c). Each auto-cast spell becomes a
/// declared reaction, `Attacked` when it would turn the hit into a miss, in the default
/// runbook, so the save fights as it did; a fight in progress gets the budget and the
/// reactions the turn slots give, as if its turn had just begun.
pub(crate) fn v5_to_v6(mut world: World, data: &Data) -> World {
    for member in &mut world.party.members {
        for spell in core::mem::take(&mut member.legacy_auto_cast) {
            let label = data
                .spells
                .get(&spell)
                .map(|s| data.label("en", &s.name).to_owned())
                .filter(|l| !l.trim().is_empty() && l.len() <= TACTICS_NAME_BYTES)
                .unwrap_or_else(|| "Reaction".to_owned());
            let set = CriteriaSet {
                name: label,
                action: ActionRef::Spell(spell),
                trigger: Trigger::Attacked,
                when: Criteria::Is(Predicate::WouldChangeOutcome),
            };
            let index = u16::try_from(member.tactics.library.len()).unwrap_or(u16::MAX);
            member.tactics.library.push(set);
            let at = usize::from(member.tactics.default_runbook);
            if let Some(book) = member.tactics.runbooks.get_mut(at) {
                book.entries.push((ActionRef::Spell(spell), index));
            }
        }
    }
    crate::combat::begin_after_load(&mut world, data);
    world.schema = SAVE_SCHEMA;
    world
}
