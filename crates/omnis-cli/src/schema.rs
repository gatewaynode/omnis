//! `omnis-cli schema dump`: the data files by example. There is no reflection over the schema
//! structs, so each section is a minimal instance of the real type, written by the same RON
//! writer the game reads with, so it always parses.

use omnis_data::ron_io::{parse, to_string};
use omnis_data::{
    Background, Class, Condition, DataError, Item, MapDef, Monster, PackFingerprint, PackManifest,
    Race, RulesFile, SCHEMA, Spell, Tileset,
};
use omnis_sim::omnis_core::{Clock, EraId, Facing, MapId, Position};
use omnis_sim::world::SAVE_SCHEMA;
use omnis_sim::{Command, Op, PARTY, Replay, World};
use std::collections::BTreeMap;

const MANIFEST: &str = r#"(
    schema: 1,
    id: "example",
    version: "0.1.0",
    name: "Example pack",
    license: "CC0-1.0",
    attribution: [("Tiles by someone", "CC0-1.0", "assets/LICENSE.txt")],
    depends: [],
    entry: Some("example:map:start"),
)"#;

const TILESET: &str = r#"(
    schema: 1,
    id: "example:tileset:stone",
    detail_depth: 1,
    width: 0,
    viewport: (960, 540),
    surfaces: {
        "floor": (kind: Floor, slots: [(depth: 0, offset: 0, path: "assets/tilesets/stone/floor_d0_o0.png", x: 0, y: 68)]),
        "wall": (kind: WallFront, slots: []),
        "wall.left": (kind: WallLeft, slots: []),
        "wall.right": (kind: WallRight, slots: []),
        "door": (kind: Door, slots: []),
        "door.open": (kind: DoorFrame, slots: []),
        "rock": (kind: Block, slots: []),
    },
)"#;

const MAP: &str = r#"(
    schema: 1,
    id: "example:map:start",
    name: "example:text:map.start.name",
    kind: Dungeon,
    tileset: "example:tileset:stone",
    width: 2,
    height: 1,
    start: (0, 0, East),
    wall: (front: "wall", left: "wall.left", right: "wall.right"),
    door: "door",
    door_open: Some("door.open"),
    terrains: [
        (glyph: '.', name: "floor", floor: "floor", ceiling: None, passable: true, opaque: false, color: (110, 90, 70), visibility_depth: 4, step_minutes: 1),
        (glyph: '#', name: "rock", floor: "floor", block: Some("rock"), passable: false, opaque: true, color: (60, 50, 45), visibility_depth: 4, step_minutes: 1),
    ],
    layout: [
        "+-+-+",
        "|.:#|",
        "+-+-+",
    ],
    portals: [(x: 1, y: 0, to_map: "example:map:start", to_x: 0, to_y: 0, to_facing: East)],
    encounters: [(x: 0, y: 0, stacks: [("example:monster:goblin", 3)], disposition: Wary, once: true)],
    random: Some((chance_percent: 4, entries: [(weight: 1, stacks: [("example:monster:goblin", (count: 1, sides: 4, modifier: 1))], disposition: Hostile)])),
)"#;

const RACE: &str = r#"(
    schema: 1,
    id: "example:race:human",
    name: "example:text:race.human.name",
    ability_bonuses: {Strength: 1, Dexterity: 1, Constitution: 1, Intelligence: 1, Wisdom: 1, Charisma: 1},
    size: Medium,
    speed: 30,
    starting_age: 18,
    features: [(name: "example:text:race.human.languages"), (name: "example:text:race.human.keen", effect: Darkvision(feet: 60))],
)"#;

const CLASS: &str = r#"(
    schema: 1,
    id: "example:class:fighter",
    name: "example:text:class.fighter.name",
    hit_die: 10,
    saving_throws: (Strength, Constitution),
    armor: [Light, Medium, Heavy, Shield],
    weapons: [Simple, Martial],
    weapon_ids: [],
    skills: (choose: 2, from: [Acrobatics, Athletics, Perception]),
    starting_equipment: [("example:item:longsword", 1)],
    casting: Some((ability: Intelligence, half: false, cantrips_at_1: 3, spells_at_1: 6, list: ["example:spell:magic_missile"])),
    features: [(level: 1, name: "example:text:class.fighter.second_wind")],
)"#;

const BACKGROUND: &str = r#"(
    schema: 1,
    id: "example:background:acolyte",
    name: "example:text:background.acolyte.name",
    skills: [Insight, Religion],
    equipment: [("example:item:holy_symbol", 1)],
    gold: 15,
    feature: (name: "example:text:background.acolyte.shelter"),
)"#;

const ITEM: &str = r#"(
    schema: 1,
    id: "example:item:longsword",
    name: "example:text:item.longsword.name",
    kind: Weapon(kind: Martial, damage: (count: 1, sides: 8, modifier: 0), damage_type: Slashing, ranged: false, two_handed: false),
    cost_cp: 1500,
    weight_tenths: 30,
)"#;

const CONDITION: &str = r#"(
    schema: 1,
    id: "example:condition:poisoned",
    name: "example:text:condition.poisoned.name",
    description: "example:text:condition.poisoned.description",
    incapacitated: false,
    attacks_against_advantage: false,
    own_attacks_disadvantage: true,
    auto_fail_str_dex_saves: false,
    melee_hits_crit: false,
    resist_all: false,
)"#;

const SPELL: &str = r#"(
    schema: 1,
    id: "example:spell:magic_missile",
    name: "example:text:spell.magic_missile.name",
    level: 1,
    school: Evocation,
    classes: ["example:class:wizard"],
    concentration: false,
    ritual: false,
    points: None,
    components: [("example:item:gem", 1)],
    description: "example:text:spell.magic_missile.description",
)"#;

const MONSTER: &str = r#"(
    schema: 1,
    id: "example:monster:goblin",
    name: "example:text:monster.goblin.name",
    size: Small,
    ac: 15,
    hit_points: (count: 2, sides: 6, modifier: 0),
    speed: 30,
    abilities: (8, 14, 10, 10, 8, 8),
    challenge: (1, 4),
    xp: 50,
    attacks: [(name: "example:text:monster.goblin.scimitar", to_hit: 4, damage: (count: 1, sides: 6, modifier: 2), damage_type: Slashing, ranged: false)],
    gold: Some((count: 1, sides: 6, modifier: 0)),
    resistances: [],
    immunities: [Poison],
    vulnerabilities: [Bludgeoning],
)"#;

const RULES: &str = r#"(
    schema: 1,
    id: "example:rules:casting",
    slots: {
        "spell_points.pool": (inputs: ["level", "cast_mod", "other_mental_mods", "half_caster"], expr: "max(level, (if half_caster { level / 2 } else { level }) * cast_mod + other_mental_mods)"),
    },
    values: {"component_threshold": 5},
    tables: {"point_cost": [0, 1, 2, 3, 4, 5, 7, 9]},
)"#;

/// Every data file type as a parsed-and-rewritten example, then a save, a replay, and the
/// protocol ops.
pub fn dump() -> Result<String, DataError> {
    let mut out = String::new();
    data_sections(&mut out)?;
    world_sections(&mut out)?;
    Ok(out)
}

fn data_sections(out: &mut String) -> Result<(), DataError> {
    section(
        out,
        &format!("pack.ron (schema {SCHEMA})"),
        &parse::<PackManifest>(MANIFEST)?,
    )?;
    section(
        out,
        &format!("data/tiles/<name>.ron (schema {SCHEMA})"),
        &parse::<Tileset>(TILESET)?,
    )?;
    section(
        out,
        &format!("data/maps/<name>.ron (schema {SCHEMA})"),
        &parse::<MapDef>(MAP)?,
    )?;
    section(out, "data/races/<name>.ron", &parse::<Race>(RACE)?)?;
    section(out, "data/classes/<name>.ron", &parse::<Class>(CLASS)?)?;
    section(
        out,
        "data/backgrounds/<name>.ron",
        &parse::<Background>(BACKGROUND)?,
    )?;
    section(out, "data/items/<name>.ron", &parse::<Item>(ITEM)?)?;
    section(
        out,
        "data/conditions/<name>.ron",
        &parse::<Condition>(CONDITION)?,
    )?;
    section(out, "data/spells/<name>.ron", &parse::<Spell>(SPELL)?)?;
    section(out, "data/monsters/<name>.ron", &parse::<Monster>(MONSTER)?)?;
    section(out, "data/rules/<name>.ron", &parse::<RulesFile>(RULES)?)?;
    Ok(())
}

fn world_sections(out: &mut String) -> Result<(), DataError> {
    let fingerprint = PackFingerprint {
        id: "example".into(),
        version: "0.1.0".into(),
        hash: 0x0123_4567_89ab_cdef,
    };
    let world = World {
        schema: SAVE_SCHEMA,
        seed: 1,
        packs: vec![fingerprint.clone()],
        rngs: BTreeMap::new(),
        clocks: BTreeMap::from([(PARTY, Clock::new(EraId(0)))]),
        position: Position {
            map: MapId(0),
            x: 0,
            y: 0,
            facing: Facing::East,
        },
        maps: BTreeMap::new(),
        automap: Default::default(),
        mode: omnis_sim::Mode::Explore,
        flags: BTreeMap::new(),
        settings: Default::default(),
        party: Default::default(),
        turn: 0,
        log: Vec::new(),
    };
    let replay = Replay {
        seed: 1,
        packs: vec![fingerprint],
        settings: Default::default(),
        commands: vec![Command::Step(omnis_sim::omnis_core::Direction::Forward)],
        fingerprint: 0,
    };
    let ops = vec![
        Op::GameStatus,
        Op::WorldQuery {
            path: "position.x".into(),
        },
        Op::SimCommand {
            command: Command::Interact,
        },
        Op::SimScript {
            commands: vec![Command::Step(omnis_sim::omnis_core::Direction::Forward)],
        },
        Op::EventsTail { count: 32 },
        Op::ViewportGet,
        Op::MapText { map: None },
        Op::AutomapGet { map: None },
        Op::SaveWrite {
            path: ".omnis/quick.ron".into(),
        },
        Op::SaveRead {
            path: ".omnis/quick.ron".into(),
            force: false,
        },
        Op::PackReload,
        Op::Screenshot { path: None },
        Op::PartyGet,
        Op::PartyCreate {
            character: omnis_sim::omnis_rules::Draft {
                name: "Brenna".into(),
                race: "example:race:human".into(),
                class: "example:class:fighter".into(),
                background: "example:background:acolyte".into(),
                alignment: omnis_data::Alignment::NeutralGood,
                scores: [15, 14, 13, 12, 10, 8],
                skills: vec![omnis_data::Skill::Athletics, omnis_data::Skill::Perception],
            },
        },
        Op::CombatGet,
        Op::RulesList,
        Op::RulesGet {
            slot: "spell_points.pool".into(),
        },
        Op::RulesSet {
            slot: "spell_points.pool".into(),
            source: "level * 10".into(),
        },
    ];
    section(out, &format!("save (schema {SAVE_SCHEMA})"), &world)?;
    section(out, "replay", &replay)?;
    section(out, "protocol ops (JSON on the dev socket; RON here)", &ops)?;
    Ok(())
}

fn section<T: serde::Serialize>(out: &mut String, title: &str, value: &T) -> Result<(), DataError> {
    out.push_str("# ");
    out.push_str(title);
    out.push('\n');
    out.push_str(&to_string(value)?);
    out.push_str("\n\n");
    Ok(())
}
