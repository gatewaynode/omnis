//! A save's pack migrations (M8.5 7h6): for each pack it was made with, the files of the loaded
//! pack leading on from that version (`omnis_data::migration::chain`). Their name steps are
//! composed and read by the remap as it finds each saved name again; their place steps act on
//! the remapped world, in order. The chain notes which steps found something of the save:
//! only those make a load ask first (owner, 2026-10-19); the rest are recorded as applied.

use crate::encounter::EncounterState;
use crate::world::{Mode, World, door_key};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use omnis_core::{Facing, Level, MapId};
use omnis_data::migration::{self, Kind, PackMigration, Step, Target};
use omnis_data::{Data, PackFingerprint};
use serde::{Deserialize, Serialize};

/// A migration file applied to a world: its pack and its file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Migrated {
    /// The pack's id.
    pub pack: String,
    /// The file, under the pack's `migrations/`.
    pub file: String,
}

/// A migration offered to a save: whose, which file, the version it leads to, and its steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offered {
    /// The pack's id.
    pub pack: String,
    /// The file.
    pub file: String,
    /// The version it leads to.
    pub to: String,
    /// Its steps.
    pub steps: Vec<Step>,
}

/// The migrations a save made with `saved` is offered by the loaded packs.
pub(crate) struct Chain<'a> {
    files: Vec<(&'a str, &'a PackMigration)>,
    names: BTreeMap<(Kind, String), Target>,
    /// Whether a step found something of the save.
    used: RefCell<bool>,
}

impl<'a> Chain<'a> {
    /// The chain for a world made with the packs `saved`, on the loaded packs `data`.
    pub(crate) fn of(saved: &[PackFingerprint], data: &'a Data) -> Chain<'a> {
        let mut files = Vec::new();
        let mut names = BTreeMap::new();
        for pack in saved {
            let Some((id, _)) = data.migrations.get_key_value(&pack.id) else {
                continue;
            };
            let chain = migration::chain(data, &pack.id, &pack.version);
            names.extend(migration::names(&chain));
            files.extend(chain.into_iter().map(|m| (id.as_str(), m)));
        }
        Chain {
            files,
            names,
            used: RefCell::new(false),
        }
    }

    /// No migration leads on from the save's packs.
    pub(crate) fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Whether a step found something of the save.
    pub(crate) fn used(&self) -> bool {
        *self.used.borrow()
    }

    fn use_it(&self) {
        *self.used.borrow_mut() = true;
    }

    /// Where the chain leads the saved name `name`, if it names it: a step found it.
    pub(crate) fn name(&self, kind: Kind, name: &str) -> Option<&Target> {
        let target = self.lead(kind, name)?;
        self.use_it();
        Some(target)
    }

    /// Where the chain leads `name`, for what is told, not held (the log): no step is found.
    pub(crate) fn lead(&self, kind: Kind, name: &str) -> Option<&Target> {
        self.names.get(&(kind, String::from(name)))
    }

    /// The files, as offered.
    pub(crate) fn offered(&self) -> Vec<Offered> {
        self.files
            .iter()
            .map(|(pack, m)| Offered {
                pack: (*pack).into(),
                file: m.file.clone(),
                to: m.to.clone(),
                steps: m.steps.clone(),
            })
            .collect()
    }

    /// The files, as recorded once applied.
    pub(crate) fn migrated(&self) -> Vec<Migrated> {
        self.files
            .iter()
            .map(|(pack, m)| Migrated {
                pack: (*pack).into(),
                file: m.file.clone(),
            })
            .collect()
    }

    /// The place steps, in order, on the remapped world: a cell moved, an encounter or an
    /// object renamed. A map the packs do not have was refused at pack load.
    pub(crate) fn places(&self, world: &mut World, data: &Data) {
        for step in self.files.iter().flat_map(|(_, m)| &m.steps) {
            let found = match step {
                Step::MovePlace { map, from, to } => data
                    .registry
                    .maps
                    .get(map)
                    .is_some_and(|map| move_place(world, map, spot(*from), spot(*to))),
                Step::RenameEncounter { map, from, to } => {
                    data.registry.maps.get(map).is_some_and(|map| {
                        let mut found = rename_local(world, map, from, to);
                        if let Some(state) = world.maps.get_mut(&map)
                            && state.cleared.remove(from)
                        {
                            state.cleared.insert(to.clone());
                            found = true;
                        }
                        found | rename_fight(world, map, from, to)
                    })
                }
                Step::RenameObject { map, from, to } => data
                    .registry
                    .maps
                    .get(map)
                    .is_some_and(|map| rename_local(world, map, from, to)),
                Step::Rename { .. } | Step::Replace { .. } | Step::Remove { .. } => false,
            };
            if found {
                self.use_it();
            }
        }
    }
}

fn spot((level, x, y): migration::Spot) -> (Level, u16, u16) {
    (Level(level), x, y)
}

/// Move what the world holds of a cell: the party on it, a fight's retreat to it, the pile on
/// it (joined to one already there), its open doors and what the automap knows of it.
fn move_place(
    world: &mut World,
    map: MapId,
    from: (Level, u16, u16),
    to: (Level, u16, u16),
) -> bool {
    let mut found = false;
    let at = |p: &omnis_core::Position| p.map == map && (p.level, p.x, p.y) == from;
    if at(&world.position) {
        (world.position.level, world.position.x, world.position.y) = to;
        found = true;
    }
    if let Some(e) = encounter_mut(&mut world.mode)
        && at(&e.retreat)
    {
        (e.retreat.level, e.retreat.x, e.retreat.y) = to;
        found = true;
    }
    if let Some(state) = world.maps.get_mut(&map) {
        if let Some(pile) = state.ground.remove(&from) {
            match state.ground.get_mut(&to) {
                Some(there) => there.items.extend(pile.items),
                None => {
                    state.ground.insert(to, pile);
                }
            }
            found = true;
        }
        for facing in [Facing::North, Facing::East, Facing::South, Facing::West] {
            let (fx, fy, ff) = door_key(from.1, from.2, facing);
            let (tx, ty, tf) = door_key(to.1, to.2, facing);
            let moved = if from.0.is_ground() {
                state.open_doors.remove(&(fx, fy, ff))
            } else {
                state.level_doors.remove(&(from.0, fx, fy, ff))
            };
            if moved {
                if to.0.is_ground() {
                    state.open_doors.insert((tx, ty, tf));
                } else {
                    state.level_doors.insert((to.0, tx, ty, tf));
                }
                found = true;
            }
        }
    }
    let automap = &mut world.automap;
    let known = if from.0.is_ground() {
        automap.maps.get_mut(&map)
    } else {
        automap.levels.get_mut(&(map, from.0))
    }
    .and_then(|tiles| tiles.remove(&(from.1, from.2)));
    if let Some(known) = known {
        let tiles = if to.0.is_ground() {
            automap.maps.entry(map).or_default()
        } else {
            automap.levels.entry((map, to.0)).or_default()
        };
        tiles.insert((to.1, to.2), known);
        found = true;
    }
    found
}

/// Rename an id of a map's placements in what the world holds of it: the object's state and
/// what the party has noticed, been told and learned of it.
fn rename_local(world: &mut World, map: MapId, from: &str, to: &str) -> bool {
    let mut found = false;
    if let Some(state) = world.maps.get_mut(&map)
        && let Some(object) = state.objects.remove(from)
    {
        state.objects.insert(to.into(), object);
        found = true;
    }
    let known = &mut world.known;
    for set in [&mut known.noticed, &mut known.told, &mut known.lore] {
        if set.remove(&(map, String::from(from))) {
            set.insert((map, to.into()));
            found = true;
        }
    }
    found
}

/// A fight or an encounter met at the renamed encounter on `map`.
fn rename_fight(world: &mut World, map: MapId, from: &str, to: &str) -> bool {
    let here = world.position.map == map;
    match encounter_mut(&mut world.mode) {
        Some(e) if here => match &mut e.source {
            crate::EncounterSource::Fixed(id) if id == from => {
                *id = to.into();
                true
            }
            _ => false,
        },
        _ => false,
    }
}

fn encounter_mut(mode: &mut Mode) -> Option<&mut EncounterState> {
    match mode {
        Mode::Encounter(e) => Some(e),
        Mode::Combat(c) => Some(&mut c.encounter),
        Mode::Explore | Mode::Town(_) => None,
    }
}
