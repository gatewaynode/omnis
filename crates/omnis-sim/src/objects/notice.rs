//! Finding hidden objects (M8.5 step 2), three ways. A sharp eye: after each look while
//! exploring, the party's best passive Perception against each hidden object whose cell is in
//! sight within `notice_tiles`. A reveal in force: every hidden object with a magical aura within
//! its range, seen or not. A search: one Investigation roll by the member best at it against
//! each hidden object in the party's cell and the one it faces, which takes `search_minutes`
//! whether or not anything is there. The same roll turns up lore in reach (owner, 2026-10-16):
//! each cell's, then each object's (a placement's own, else its kind's), when it asks no
//! difficulty or the roll meets it, and a `once` lore only the first time.

use super::act::{reach, reachable};
use super::{Refusal, edit, saved};
use crate::apply::advance;
use crate::checks::{self, CheckSpec};
use crate::combat::Roller;
use crate::combat::state::can_fight;
use crate::command::Rejection;
use crate::event::{Event, Notice, SeenTile};
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::placement::ResolvedPlacement;
use omnis_data::{Data, Skill};
use omnis_rules::{RollMode, passive, reveal_range};

/// Cells a sharp eye reaches when the rules do not say.
const NOTICE_TILES: u16 = 2;
/// Minutes a search takes when the rules do not say.
const SEARCH_MINUTES: u32 = 10;

/// What the party finds by looking, after a look while exploring: by a reveal in force or by
/// its best passive Perception.
pub(crate) fn look(world: &mut World, data: &Data, tiles: &[SeenTile], events: &mut Vec<Event>) {
    let pos = world.position;
    let placements: Vec<&ResolvedPlacement> = data
        .maps
        .get(&pos.map)
        .into_iter()
        .flat_map(|m| m.objects.iter())
        .filter(|p| p.level == pos.level && unfound(world, p))
        .collect();
    if placements.is_empty() || !matches!(world.mode, Mode::Explore) {
        return;
    }
    let near = rule(data, "notice_tiles").map_or(NOTICE_TILES, |v| u16::try_from(v).unwrap_or(0));
    let reveal = reveal_range(&world.party.effects).map(u16::from);
    let eye = world
        .party
        .members
        .iter()
        .filter(|m| can_fight(m, data))
        .filter_map(|m| passive(m, data, Skill::Perception).ok())
        .max();
    let found: Vec<(String, Notice)> = placements
        .into_iter()
        .filter_map(|p| {
            let far = distance(world, p);
            let aura = data
                .objects
                .get(&p.kind)
                .is_some_and(|k| k.attributes.aura.is_some());
            let seen = tiles.iter().any(|t| (t.x, t.y) == (p.x, p.y));
            let sharp = eye.is_some_and(|e| p.hidden.is_some_and(|dc| e >= i64::from(dc)));
            if aura && reveal.is_some_and(|r| far <= r) {
                Some((p.id.clone(), Notice::Spell))
            } else if seen && far <= near && sharp {
                Some((p.id.clone(), Notice::Passive))
            } else {
                None
            }
        })
        .collect();
    for (object, how) in found {
        mark(world, object, how, events);
    }
}

/// Search the party's cell and the one it faces.
pub(crate) fn search(
    world: &mut World,
    data: &Data,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let spec = CheckSpec {
        skill: Some(Skill::Investigation),
        ability: Skill::Investigation.ability(),
        mode: RollMode::Normal,
    };
    let member = checks::best(world, data, spec).ok_or(Rejection::VerbRefused {
        reason: Refusal::NoneCanAct,
    })?;
    let mut roller = Roller::take_stream(world, "object");
    let roll =
        checks::roll(world, data, member, spec, &mut roller, events).map_err(Rejection::Rule)?;
    roller.store(world);
    let pos = world.position;
    let mut found: Vec<String> = Vec::new();
    if let Some(map) = data.maps.get(&pos.map) {
        let placements = reach(world, map, pos)
            .into_iter()
            .flat_map(|(x, y)| map.objects_at(pos.level, x, y))
            .map(|(_, p)| p);
        for p in placements {
            let beaten = p.hidden.is_some_and(|dc| roll.total >= i64::from(dc));
            if beaten && unfound(world, p) && !found.contains(&p.id) {
                found.push(p.id.clone());
            }
        }
    }
    let total = roll.total;
    events.push(Event::Searched {
        member: world.party.members[member].id,
        roll,
    });
    for object in found {
        mark(world, object, Notice::Search, events);
    }
    tell(world, data, total, events);
    let minutes =
        rule(data, "search_minutes").map_or(SEARCH_MINUTES, |v| u32::try_from(v).unwrap_or(0));
    advance(world, data, minutes, events);
    Ok(())
}

/// Whose lore a search tells: a cell's by its id, or a placement's.
enum Teller {
    Cell(String),
    Object(String),
}

/// Tell the lore in reach that a search's `total` meets, once lore only once.
fn tell(world: &mut World, data: &Data, total: i64, events: &mut Vec<Event>) {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return;
    };
    let meets = |dc: Option<u8>| dc.is_none_or(|dc| total >= i64::from(dc));
    let state = world.maps.get(&pos.map);
    let cells = reach(world, map, pos)
        .into_iter()
        .flat_map(|(x, y)| map.lore.get(&(pos.level, x, y)).into_iter().flatten())
        .filter(|l| meets(l.dc) && !(l.once && state.is_some_and(|s| s.lore_told.contains(&l.id))))
        .map(|l| (l.text.clone(), l.once, Teller::Cell(l.id.clone())));
    let objects = reachable(world, data).filter_map(|p| {
        let lore = p
            .lore
            .as_ref()
            .or_else(|| data.objects.get(&p.kind)?.lore.as_ref())?;
        let spent = lore.once && saved(world, pos.map, &p.id).is_some_and(|s| s.told);
        (meets(lore.dc) && !spent)
            .then(|| (lore.text.clone(), lore.once, Teller::Object(p.id.clone())))
    });
    let told: Vec<_> = cells.chain(objects).collect();
    for (key, once, teller) in told {
        let object = match teller {
            Teller::Cell(id) => {
                if once {
                    world.map_state(pos.map).lore_told.insert(id);
                }
                None
            }
            Teller::Object(id) => {
                if once {
                    edit(world, pos.map, &id, |s| s.told = true);
                }
                Some(id)
            }
        };
        events.push(Event::Lore { key, object });
    }
}

/// Whether a placement is hidden and not yet found nor taken away.
fn unfound(world: &World, p: &ResolvedPlacement) -> bool {
    p.hidden.is_some()
        && !saved(world, world.position.map, &p.id).is_some_and(|s| s.noticed || s.removed)
}

/// Cells from the party to a placement's own cell, as a king moves.
fn distance(world: &World, p: &ResolvedPlacement) -> u16 {
    let pos = world.position;
    pos.x.abs_diff(p.x).max(pos.y.abs_diff(p.y))
}

fn rule(data: &Data, key: &str) -> Option<i64> {
    data.rules.value(key)
}

fn mark(world: &mut World, object: String, how: Notice, events: &mut Vec<Event>) {
    edit(world, world.position.map, &object, |s| s.noticed = true);
    events.push(Event::Noticed { object, how });
}
