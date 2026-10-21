//! Fixed encounters by id (M8.5 7h2). Before schema 9 a save named a map's fixed encounter by
//! its row in the map file: in the cleared set of a map's state, in an encounter or fight's
//! source, and in the journal's `Cleared` change. Such a row reads as a marker, `#` and the row,
//! which no id can be (`[a-z0-9_]`); a load turns each marker into the id of that row through
//! the loaded maps, and refuses a row a loaded map does not have. A map no loaded pack has keeps
//! its markers: nothing reads its state. The party's log is told, not played: a row its map no
//! longer has keeps its marker there, and never blocks the load (owner, 2026-10-18).

use super::Split;
use crate::encounter::EncounterSource;
use crate::event::Event;
use crate::journal::{Change, Journal};
use crate::log::Logged;
use crate::world::{LoadError, MapState, Mode, World};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use omnis_core::MapId;
use omnis_data::Data;
use serde::{Deserialize, Deserializer};

/// What a save holds where an encounter is named: its id, or its row before schema 9.
#[derive(Deserialize)]
#[serde(untagged)]
enum Written {
    Row(u16),
    Id(String),
}

impl From<Written> for String {
    fn from(written: Written) -> String {
        match written {
            Written::Row(row) => format!("#{row}"),
            Written::Id(id) => id,
        }
    }
}

/// An encounter's id as written, a row read as its marker.
pub(crate) fn row_or_id<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Written::deserialize(d).map(String::from)
}

/// A set of encounter ids as written, rows read as their markers.
pub(crate) fn rows_or_ids<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeSet<String>, D::Error> {
    let written = alloc::vec::Vec::<Written>::deserialize(d)?;
    Ok(written.into_iter().map(String::from).collect())
}

/// The id a marker stands for on `map`; an id is left as it is.
fn resolve(data: &Data, map: MapId, key: &mut String) -> Result<(), LoadError> {
    let Some(row) = key.strip_prefix('#') else {
        return Ok(());
    };
    let Some(def) = data.maps.get(&map) else {
        return Ok(());
    };
    let id = row
        .parse::<usize>()
        .ok()
        .and_then(|row| def.encounters.get(row))
        .map(|e| e.id.clone())
        .ok_or_else(|| {
            LoadError::BadSave(format!(
                "map '{}' has no fixed encounter in row {row}",
                data.registry.maps.name(map).unwrap_or("?")
            ))
        })?;
    *key = id;
    Ok(())
}

/// Every map state's cleared rows, as ids.
fn maps(data: &Data, maps: &mut BTreeMap<MapId, MapState>) -> Result<(), LoadError> {
    for (map, state) in maps.iter_mut() {
        let mut cleared = BTreeSet::new();
        for mut key in core::mem::take(&mut state.cleared) {
            resolve(data, *map, &mut key)?;
            cleared.insert(key);
        }
        state.cleared = cleared;
    }
    Ok(())
}

/// The source of an encounter or fight on the party's map, as an id.
fn mode(data: &Data, map: MapId, mode: &mut Mode) -> Result<(), LoadError> {
    let source = match mode {
        Mode::Encounter(state) => &mut state.source,
        Mode::Combat(state) => &mut state.encounter.source,
        Mode::Explore | Mode::Town(_) => return Ok(()),
    };
    match source {
        EncounterSource::Fixed(key) => resolve(data, map, key),
        EncounterSource::Random | EncounterSource::Ambush => Ok(()),
    }
}

/// A whole world's rows, as ids.
pub(crate) fn world(data: &Data, world: &mut World) -> Result<(), LoadError> {
    maps(data, &mut world.maps)?;
    mode(data, world.position.map, &mut world.mode)
}

/// The encounters the party's log names, as ids where its maps still have the rows.
pub(crate) fn log(data: &Data, log: &mut [Logged]) {
    for line in log {
        if let Event::EncounterStarted {
            source: EncounterSource::Fixed(key),
            ..
        } = &mut line.event
        {
            // A row the map no longer has keeps its marker.
            let _ = resolve(data, line.map, key);
        }
    }
}

/// A character save's rows, as ids: its bases, its party's encounter and its journal's changes.
pub(crate) fn parts(
    data: &Data,
    parts: &mut Split,
    journal: &mut Journal,
) -> Result<(), LoadError> {
    maps(data, &mut parts.world.strays)?;
    for region in parts.regions.values_mut() {
        maps(data, &mut region.maps)?;
    }
    let checkpoint = &mut parts.checkpoint;
    mode(data, checkpoint.position.map, &mut checkpoint.mode)?;
    for entry in journal.entries_mut() {
        for change in &mut entry.changes {
            if let Change::Cleared { map, id, .. } = change {
                resolve(data, *map, id)?;
            }
        }
    }
    Ok(())
}
