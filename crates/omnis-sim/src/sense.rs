//! Remote sensing (PRD §7.2, D18): a member looks along the facing line through a sense item
//! and the automap records what the look reached as remotely seen. Rust rolls one skill check
//! per knowledge layer for the party's best eyes; the `sense.dc` slot sets each tile's
//! difficulty from its distance, the visibility depth, and the layer. A tile is reached by a
//! layer when the check's total meets that tile's difficulty; the reach is the farthest such
//! tile, since the difficulty never falls with distance, and never past the layer below.

use crate::checks::{self, CheckSpec};
use crate::combat::Roller;
use crate::combat::state::can_fight;
use crate::event::{Event, LayerCheck, SensedTile};
use crate::visibility;
use crate::world::{Known, World, layer};
use alloc::vec::Vec;
use core::cmp::Reverse;
use omnis_core::ItemId;
use omnis_data::omnis_expr::Value;
use omnis_data::{Data, Geometry, SenseSource, Skill};
use omnis_rules::{RollMode, RuleError, skill_bonus};

/// The knowledge layers a look can carry, in the order they are checked: the `sense.dc`
/// layer number and the automap bit. Objects and creatures join when the automap has them.
const LAYERS: [(u8, u8); 2] = [(1, layer::TERRAIN), (2, layer::STRUCTURE)];

/// The member who looks: the best Perception among those who can act, the first in
/// marching order on ties.
fn best_eyes(world: &World, data: &Data) -> Result<usize, RuleError> {
    world
        .party
        .members
        .iter()
        .enumerate()
        .filter(|(_, m)| can_fight(m, data))
        .min_by_key(|(_, m)| Reverse(skill_bonus(m, data, Skill::Perception).unwrap_or(0)))
        .map(|(i, _)| i)
        .ok_or_else(|| RuleError::new("sense", "no member can look"))
}

/// The difficulty of a tile `distance` tiles out for a layer (slot `sense.dc`).
fn dc(
    data: &Data,
    distance: u8,
    visibility: u8,
    layer: u8,
    roller: &mut Roller,
) -> Result<i64, RuleError> {
    let outcome = data.rules.eval(
        "sense.dc",
        &[
            ("distance", Value::Int(i64::from(distance))),
            ("visibility", Value::Int(i64::from(visibility))),
            ("layer", Value::Int(i64::from(layer))),
        ],
        &mut roller.rng,
        &roller.stream,
    )?;
    outcome
        .value
        .as_int()
        .ok_or_else(|| RuleError::new("sense.dc", "formula must produce an integer"))
}

/// How many of `tiles` a check with `total` reaches for a layer: the count up to the first
/// tile whose difficulty it misses.
fn reach_for_layer(
    data: &Data,
    total: i64,
    tiles: usize,
    visibility: u8,
    layer: u8,
    roller: &mut Roller,
) -> Result<u8, RuleError> {
    let mut reach = 0u8;
    for distance in 1..=u8::try_from(tiles).unwrap_or(u8::MAX) {
        if total < dc(data, distance, visibility, layer, roller)? {
            break;
        }
        reach = distance;
    }
    Ok(reach)
}

/// Look through `item` from the party's tile: the checks, the recording, and `Sensed`.
pub(crate) fn resolve(
    world: &mut World,
    data: &Data,
    item: ItemId,
    source: &SenseSource,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let pos = world.position;
    let eyes = best_eyes(world, data)?;
    let actor = world.party.members[eyes].id;
    let Some(map) = data.maps.get(&pos.map) else {
        return Err(RuleError::new("sense", "the party's map is not loaded"));
    };
    let Geometry::Ray { range } = source.geometry;
    let ray = visibility::ray(map, world.maps.get(&pos.map), pos, range);
    let visibility = visibility::depth(world, map);
    let mut checks = Vec::new();
    // A layer reaches no farther than the one below it: failing a layer stops the ladder
    // there (PRD §7.2, D18), so a wall's shape is never known where its ground is not.
    let mut floor = u8::try_from(ray.len()).unwrap_or(u8::MAX);
    for (layer, _) in LAYERS.iter().take(usize::from(source.fidelity.rank())) {
        if floor == 0 {
            break;
        }
        let (roll, total) = match source.check {
            Some(skill) => {
                let spec = CheckSpec {
                    skill: Some(skill),
                    ability: skill.ability(),
                    mode: RollMode::Normal,
                };
                let roll = checks::roll(world, data, eyes, spec, roller, events)?;
                let total = roll.total;
                (Some(roll), total)
            }
            None => (None, i64::MAX),
        };
        let reach = reach_for_layer(data, total, ray.len(), visibility, *layer, roller)?.min(floor);
        floor = reach;
        checks.push(LayerCheck {
            layer: *layer,
            roll,
            reach,
        });
    }
    let seen_at = world.party_clock().elapsed;
    let mut tiles = Vec::new();
    for (i, (x, y)) in ray.iter().enumerate() {
        let distance = u8::try_from(i + 1).unwrap_or(u8::MAX);
        let layers = checks
            .iter()
            .zip(LAYERS)
            .filter(|(check, _)| check.reach >= distance)
            .fold(0u8, |bits, (_, (_, bit))| bits | bit);
        let Some(cell) = map.cell(*x, *y).filter(|_| layers != 0) else {
            continue;
        };
        world.automap.record(
            pos.map,
            *x,
            *y,
            Known {
                terrain: cell.terrain,
                walls: cell.walls,
                doors: cell.doors,
                layers: layers | layer::REMOTE,
                seen_at,
            },
        );
        tiles.push(SensedTile {
            x: *x,
            y: *y,
            layers,
        });
    }
    events.push(Event::Sensed {
        actor,
        item,
        checks,
        tiles,
    });
    Ok(())
}
