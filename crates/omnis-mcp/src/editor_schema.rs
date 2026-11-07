//! Hand-written JSON Schemas for the editor's argument types (M5.3): `Brush`, `Place`, `Edit`,
//! `NewMap` and what they hold. As with `Command`, nothing is generated: each shape here is the
//! serde form of the Rust type in `omnis_data::edit` and the map file's structs, and the tests
//! deserialise an instance of every branch.

use crate::schema::Schema;
use omnis_cli::omnis_sim::omnis_core::{Facing, Level};
use omnis_cli::omnis_sim::omnis_data::edit::{Brush, EdgeKind, Edit, NewMap, Place, Target};
use serde_json::{Value, json};

impl Schema for u16 {
    fn schema() -> Value {
        json!({"type": "integer", "minimum": 0, "maximum": u16::MAX})
    }
}

impl Schema for i16 {
    fn schema() -> Value {
        json!({"type": "integer", "minimum": i16::MIN, "maximum": i16::MAX})
    }
}

impl Schema for char {
    fn schema() -> Value {
        json!({"type": "string", "minLength": 1, "maxLength": 1, "description": "One character"})
    }
}

impl Schema for Facing {
    fn schema() -> Value {
        json!({"type": "string", "enum": ["North", "East", "South", "West"]})
    }
}

impl Schema for EdgeKind {
    fn schema() -> Value {
        json!({"type": "string", "enum": ["Open", "Wall", "Door", "Ramp"]})
    }
}

/// An object with these properties, the required ones named, nothing else allowed.
fn obj(properties: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": properties, "required": required, "additionalProperties": false})
}

/// A variant carrying one object: `{"Name": {...}}`.
fn variant(name: &str, inner: Value) -> Value {
    json!({"type": "object", "properties": {name: inner}, "required": [name], "additionalProperties": false})
}

/// A string id of the form `pack:kind:name`.
fn id() -> Value {
    json!({"type": "string", "description": "pack:kind:name"})
}

fn level() -> Value {
    Level::schema()
}

fn xy() -> (Value, Value) {
    (u16::schema(), u16::schema())
}

/// A `Portal` as the map file writes it (W1 step 2: linked by id, the landing ahead of the
/// partner; unlinked until something links it).
fn portal() -> Value {
    let (x, y) = xy();
    obj(
        json!({"level": level(), "x": x, "y": y, "id": {"type": "string", "description": "The portal's id, unique across every loaded map: 1 to 32 printable ASCII characters"}, "link": {"type": ["string", "null"], "description": "The id of the portal this one leads to; the other side may state the link instead. None: unlinked, a plain tile in play"}, "facing": Facing::schema(), "marker": {"type": ["string", "null"], "description": "An Object surface of the tileset; every portal must be visible"}}),
        &["x", "y", "id", "facing"],
    )
}

/// A `FixedEncounter`.
fn encounter() -> Value {
    let (x, y) = xy();
    let stand = obj(
        json!({"pos": {"type": "array", "items": {"type": "integer", "minimum": -4, "maximum": 4}, "minItems": 2, "maxItems": 2}, "facing": Facing::schema()}),
        &[],
    );
    obj(
        json!({"id": {"type": "string", "pattern": "^[a-z0-9_]{1,32}$"}, "level": level(), "x": x, "y": y,
            "stacks": {"type": "array", "items": {"type": "array", "prefixItems": [id(), {"type": "integer", "minimum": 1, "maximum": 255}], "minItems": 2, "maxItems": 2}, "minItems": 1, "maxItems": 4},
            "disposition": {"type": "string", "enum": ["Hostile", "Wary", "Neutral", "Friendly"]},
            "once": {"type": "boolean"}, "stand": {"anyOf": [stand, {"type": "null"}]}}),
        &["id", "x", "y", "stacks"],
    )
}

/// `RandomEncounters`.
fn random() -> Value {
    let dice = obj(
        json!({"count": u16::schema(), "sides": u16::schema(), "modifier": {"type": "integer"}}),
        &["count", "sides", "modifier"],
    );
    let entry = obj(
        json!({"weight": {"type": "integer", "minimum": 1, "maximum": u16::MAX},
            "stacks": {"type": "array", "items": {"type": "array", "prefixItems": [id(), dice], "minItems": 2, "maxItems": 2}, "minItems": 1, "maxItems": 4},
            "disposition": {"type": "string", "enum": ["Hostile", "Wary", "Neutral", "Friendly"]}}),
        &["weight", "stacks"],
    );
    obj(
        json!({"chance_percent": {"type": "integer", "minimum": 0, "maximum": 100}, "entries": {"type": "array", "items": entry, "minItems": 1}}),
        &["chance_percent", "entries"],
    )
}

/// A `Site`.
fn site() -> Value {
    let (x, y) = xy();
    obj(
        json!({"level": level(), "x": x, "y": y, "service": id()}),
        &["x", "y", "service"],
    )
}

/// A `RestEventDef`.
fn rest_event() -> Value {
    obj(
        json!({"terrain": {"type": "string", "description": "A terrain's name on this map"}, "rest": {"type": "string", "enum": ["Short", "Long", "Any"]}, "chance": {"type": "integer", "minimum": 0, "maximum": 1000, "description": "Per mille"}, "text": id()}),
        &["terrain", "rest", "chance", "text"],
    )
}

fn stacks(max: usize) -> Value {
    json!({"type": "array", "items": {"type": "array", "prefixItems": [id(), {"type": "integer", "minimum": 1, "maximum": u16::MAX}], "minItems": 2, "maxItems": 2}, "maxItems": max})
}

fn pos() -> Value {
    json!({"type": "array", "items": {"type": "integer", "minimum": -4, "maximum": 4}, "minItems": 2, "maxItems": 2, "description": "Eighths east and south of the cell's centre"})
}

fn lore() -> Value {
    obj(
        json!({"text": id(), "dc": {"type": ["integer", "null"], "minimum": 1, "maximum": 30}, "once": {"type": "boolean"}}),
        &["text"],
    )
}

/// An object `Placement`.
fn placement() -> Value {
    let (x, y) = xy();
    obj(
        json!({"id": {"type": "string", "pattern": "^[a-z0-9_]{1,32}$"}, "kind": id(), "level": level(), "x": x, "y": y, "pos": pos(), "facing": Facing::schema(),
            "state": {"type": ["string", "null"]}, "hidden": {"type": ["integer", "null"], "minimum": 1, "maximum": 30}, "name": {"type": ["string", "null"]},
            "contents": stacks(64), "lore": {"anyOf": [lore(), {"type": "null"}]}}),
        &["id", "kind", "x", "y"],
    )
}

/// A `GroundDef`.
fn ground() -> Value {
    let (x, y) = xy();
    obj(
        json!({"level": level(), "x": x, "y": y, "pos": pos(), "items": stacks(32)}),
        &["x", "y", "items"],
    )
}

/// A `CellLore`.
fn cell_lore() -> Value {
    let (x, y) = xy();
    obj(
        json!({"id": {"type": "string", "pattern": "^[a-z0-9_]{1,32}$"}, "level": level(), "x": x, "y": y, "text": id(), "dc": {"type": ["integer", "null"], "minimum": 1, "maximum": 30}, "once": {"type": "boolean"}}),
        &["id", "x", "y", "text"],
    )
}

/// A `Terrain`.
fn terrain() -> Value {
    let colour = json!({"type": "array", "items": {"type": "integer", "minimum": 0, "maximum": 255}, "minItems": 3, "maxItems": 3});
    obj(
        json!({"glyph": char::schema(), "name": {"type": "string"}, "floor": {"type": "string"}, "ceiling": {"type": ["string", "null"]}, "block": {"type": ["string", "null"]},
            "passable": {"type": "boolean"}, "opaque": {"type": "boolean"}, "color": colour, "visibility_depth": {"type": "integer", "minimum": 1, "maximum": 32},
            "step_minutes": {"type": "integer", "minimum": 0}, "climb": u16::schema(), "join": {"type": ["string", "null"], "enum": ["Up", "Down", "Ladder", "Hole", null]}}),
        &[
            "glyph",
            "name",
            "floor",
            "passable",
            "visibility_depth",
            "step_minutes",
        ],
    )
}

/// `WallSurfaces`.
fn wall() -> Value {
    obj(
        json!({"front": {"type": "string"}, "left": {"type": "string"}, "right": {"type": "string"}}),
        &["front", "left", "right"],
    )
}

impl Schema for Target {
    fn schema() -> Value {
        let (x, y) = xy();
        let at = obj(json!({"level": level(), "x": x, "y": y}), &["x", "y"]);
        let by_id = obj(json!({"id": {"type": "string"}}), &["id"]);
        json!({"description": "What names one placement to remove.", "oneOf": [
            variant("Portal", at.clone()), variant("Encounter", by_id.clone()), variant("Site", at.clone()),
            variant("RestEvent", obj(json!({"terrain": {"type": "string"}, "rest": {"type": "string", "enum": ["Short", "Long", "Any"]}}), &["terrain", "rest"])),
            variant("Object", by_id.clone()), variant("Ground", at), variant("Lore", by_id),
        ]})
    }
}

impl Schema for Brush {
    fn schema() -> Value {
        json!({"description": "What a paint puts on a tile or an edge.", "oneOf": [
            variant("Tile", obj(json!({"glyph": char::schema(), "rise": {"anyOf": [Facing::schema(), {"type": "null"}], "description": "Required when the terrain is a stair up or down"}}), &["glyph"])),
            variant("Edge", obj(json!({"side": Facing::schema(), "kind": EdgeKind::schema()}), &["side", "kind"])),
            variant("Height", obj(json!({"eighths": i16::schema()}), &["eighths"])),
            {"type": "string", "enum": ["Clear"], "description": "No ground"},
        ]})
    }
}

impl Schema for Place {
    fn schema() -> Value {
        let (x, y) = xy();
        json!({"description": "A placement put on the map (replacing one with the same key) or removed.", "oneOf": [
            variant("Start", obj(json!({"level": level(), "x": x, "y": y, "facing": Facing::schema()}), &["x", "y", "facing"])),
            variant("Portal", portal()), variant("Encounter", encounter()),
            variant("Random", json!({"anyOf": [random(), {"type": "null"}]})),
            variant("Site", site()), variant("RestEvent", rest_event()), variant("Object", placement()),
            variant("Ground", ground()), variant("Lore", cell_lore()), variant("Remove", Target::schema()),
        ]})
    }
}

impl Schema for Edit {
    fn schema() -> Value {
        let (x, y) = xy();
        let fields = obj(
            json!({"name": {"type": ["string", "null"]}, "kind": {"type": ["string", "null"], "enum": ["Outdoor", "Town", "Dungeon", "Special", null]}, "tileset": {"type": ["string", "null"]},
                "wall": {"anyOf": [wall(), {"type": "null"}]}, "door": {"type": ["string", "null"]}, "door_open": {"anyOf": [{"type": ["string", "null"]}], "description": "null clears it"}}),
            &[],
        );
        json!({"description": "A change to the map's structure.", "oneOf": [
            variant("Resize", obj(json!({"width": u16::schema(), "height": u16::schema()}), &["width", "height"])),
            variant("Map", fields),
            variant("LevelAdd", obj(json!({"level": {"type": "integer", "minimum": -128, "maximum": 127}, "name": id(), "base": i16::schema()}), &["level", "name"])),
            variant("LevelSet", obj(json!({"level": {"type": "integer", "minimum": -128, "maximum": 127}, "name": {"type": ["string", "null"]}, "base": {"type": ["integer", "null"]}}), &["level"])),
            variant("LevelRemove", obj(json!({"level": {"type": "integer", "minimum": -128, "maximum": 127}}), &["level"])),
            variant("TerrainAdd", terrain()),
            variant("TerrainSet", obj(json!({"glyph": char::schema(), "terrain": terrain()}), &["glyph", "terrain"])),
            variant("TerrainRemove", obj(json!({"glyph": char::schema()}), &["glyph"])),
            variant("Rise", obj(json!({"level": level(), "x": x, "y": y, "rise": Facing::schema()}), &["x", "y", "rise"])),
        ]})
    }
}

impl Schema for NewMap {
    fn schema() -> Value {
        obj(
            json!({"id": id(), "name": id(), "kind": {"type": "string", "enum": ["Outdoor", "Town", "Dungeon", "Special"]}, "tileset": id(),
                "width": {"type": "integer", "minimum": 1, "maximum": 256}, "height": {"type": "integer", "minimum": 1, "maximum": 256},
                "wall": wall(), "door": {"type": "string"}, "door_open": {"type": ["string", "null"]}, "terrain": terrain()}),
            &[
                "id", "name", "kind", "tileset", "width", "height", "wall", "door", "terrain",
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One instance of every branch reads back as the Rust type, so the schema's shapes are the
    /// protocol's.
    #[test]
    fn every_branch_is_an_instance_of_the_type() {
        let brushes = [
            json!({"Tile": {"glyph": "r", "rise": "North"}}),
            json!({"Edge": {"side": "East", "kind": "Door"}}),
            json!({"Height": {"eighths": -8}}),
            json!("Clear"),
        ];
        for b in &brushes {
            serde_json::from_value::<Brush>(b.clone()).unwrap_or_else(|e| panic!("{b}: {e}"));
        }
        assert_eq!(
            Brush::schema()["oneOf"].as_array().unwrap().len(),
            brushes.len()
        );
        let places = [
            json!({"Start": {"x": 1, "y": 2, "facing": "South"}}),
            json!({"Portal": {"x": 11, "y": 2, "id": "town-gate", "link": "meadow-gate", "facing": "West", "marker": "signpost"}}),
            json!({"Encounter": {"id": "rats", "x": 3, "y": 3, "stacks": [["test:monster:rat", 3]], "disposition": "Wary", "once": true, "stand": {"pos": [0, -3], "facing": "South"}}}),
            json!({"Random": {"chance_percent": 3, "entries": [{"weight": 2, "stacks": [["test:monster:goblin", {"count": 1, "sides": 3, "modifier": 0}]]}]}}),
            json!({"Site": {"x": 1, "y": 1, "service": "base:service:inn"}}),
            json!({"RestEvent": {"terrain": "street", "rest": "Any", "chance": 0, "text": "test:text:rest.town.street"}}),
            json!({"Object": {"id": "chest.1", "kind": "objects:object:chest", "x": 4, "y": 2, "pos": [0, -3], "facing": "South", "contents": [["base:item:mace", 1]]}}),
            json!({"Ground": {"x": 4, "y": 2, "items": [["base:item:mace", 1]]}}),
            json!({"Lore": {"id": "scratch", "x": 4, "y": 2, "text": "test:text:lore.scratch", "dc": 10}}),
            json!({"Remove": {"Site": {"x": 1, "y": 1}}}),
        ];
        for p in &places {
            serde_json::from_value::<Place>(p.clone()).unwrap_or_else(|e| panic!("{p}: {e}"));
        }
        // A start names its level when it is not the ground (W1 step 3).
        serde_json::from_value::<Place>(
            json!({"Start": {"level": 1, "x": 4, "y": 2, "facing": "North"}}),
        )
        .expect("a start with its level");
        assert_eq!(
            Place::schema()["oneOf"].as_array().unwrap().len(),
            places.len()
        );
        let targets = [
            json!({"Portal": {"x": 1, "y": 1}}),
            json!({"Encounter": {"id": "rats"}}),
            json!({"Site": {"level": 1, "x": 1, "y": 1}}),
            json!({"RestEvent": {"terrain": "street", "rest": "Short"}}),
            json!({"Object": {"id": "chest.1"}}),
            json!({"Ground": {"x": 1, "y": 1}}),
            json!({"Lore": {"id": "scratch"}}),
        ];
        for t in &targets {
            serde_json::from_value::<Target>(t.clone()).unwrap_or_else(|e| panic!("{t}: {e}"));
        }
        assert_eq!(
            Target::schema()["oneOf"].as_array().unwrap().len(),
            targets.len()
        );
        let terrain = json!({"glyph": "~", "name": "pond", "floor": "water", "passable": false, "visibility_depth": 12, "step_minutes": 1});
        let edits = [
            json!({"Resize": {"width": 8, "height": 6}}),
            json!({"Map": {"name": "test:text:map.town.name", "door_open": null}}),
            json!({"LevelAdd": {"level": 1, "name": "test:text:map.town.upper", "base": 8}}),
            json!({"LevelSet": {"level": 1, "base": 16}}),
            json!({"LevelRemove": {"level": 1}}),
            json!({"TerrainAdd": terrain}),
            json!({"TerrainSet": {"glyph": "r", "terrain": terrain}}),
            json!({"TerrainRemove": {"glyph": "~"}}),
            json!({"Rise": {"x": 2, "y": 0, "rise": "West"}}),
        ];
        for e in &edits {
            serde_json::from_value::<Edit>(e.clone()).unwrap_or_else(|e2| panic!("{e}: {e2}"));
        }
        assert_eq!(
            Edit::schema()["oneOf"].as_array().unwrap().len(),
            edits.len()
        );
        let new = json!({"id": "test:map:hamlet", "name": "test:text:map.town.name", "kind": "Town", "tileset": "test:tileset:outdoor", "width": 4, "height": 3, "wall": {"front": "hedge", "left": "hedge.left", "right": "hedge.right"}, "door": "gate", "terrain": terrain});
        serde_json::from_value::<NewMap>(new.clone()).unwrap();
        for field in NewMap::schema()["required"].as_array().unwrap() {
            assert!(new.get(field.as_str().unwrap()).is_some(), "{field}");
        }
    }
}
