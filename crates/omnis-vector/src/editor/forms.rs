//! The forms' kinds (M5.6): the fields each placement, terrain, level or note needs, filled
//! from what is there when one is edited, and the request its Apply makes (`Change`: a place or
//! an edit is one undo step in the document, a note is written at once). A placement keyed by
//! its cell (a portal, a site, a pile) is placed where the form was opened; one keyed by an id
//! (an encounter, an object, a lore) may be moved. Bevy-free.

use super::form::{self, At, Change, Form, FormKind, List, Value};
use omnis_sim::omnis_core::rng::Dice;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::{Edit, Place, Target};
use omnis_sim::omnis_data::encounter::{
    Disposition, FixedEncounter, RandomEncounters, RandomEntry, Stand,
};
use omnis_sim::omnis_data::levels::LevelDef;
use omnis_sim::omnis_data::limits::{
    MAX_HEIGHT, MAX_MAP_SIDE, MAX_STACKS, MAX_STRING_BYTES, MAX_VISIBILITY_DEPTH,
};
use omnis_sim::omnis_data::lore::{CellLore, Lore};
use omnis_sim::omnis_data::map::{Join, Terrain};
use omnis_sim::omnis_data::placement::{GroundDef, MAX_OFFSET, Placement};
use omnis_sim::omnis_data::rest_event::{RestEventDef, RestKind};
use omnis_sim::omnis_data::service::Site;
use omnis_sim::omnis_data::{Data, MapDef, Portal};

/// The kinds the inspector's Add places at the cursor, in the order of its choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placing {
    /// A portal to another map.
    Portal,
    /// A fixed encounter.
    Encounter,
    /// A town service's site.
    Site,
    /// A placed object.
    Object,
    /// A ground pile.
    Ground,
    /// A cell's lore.
    Lore,
}

impl Placing {
    /// Every kind, in the choice's order.
    pub const ALL: [Placing; 6] = [
        Placing::Portal,
        Placing::Encounter,
        Placing::Site,
        Placing::Object,
        Placing::Ground,
        Placing::Lore,
    ];

    /// Its name in the choice.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Placing::Portal => "Portal",
            Placing::Encounter => "Encounter",
            Placing::Site => "Site",
            Placing::Object => "Object",
            Placing::Ground => "Ground pile",
            Placing::Lore => "Lore",
        }
    }
}

/// The most bytes an id or a text key typed in a form takes.
const ID_MAX: usize = 96;
/// The most bytes a line of creatures and dice takes.
const CREATURES_MAX: usize = 400;
/// The most of a stack or an item.
const COUNT_MAX: i64 = 255;
/// The most of an item in a container or a pile.
const ITEMS_MAX: i64 = 65_535;
/// The highest lore DC.
const DC_MAX: i64 = 30;
/// The deepest a hidden object hides.
const HIDDEN_MAX: i64 = 30;
/// The most a random entry weighs.
const WEIGHT_MAX: i64 = 65_535;
/// The rest events' chance scale: per mille.
const PER_MILLE: i64 = 1000;
/// The most minutes a step on a terrain may take.
const STEP_MINUTES_MAX: i64 = 1440;
/// The most a terrain's climb may be, in eighths.
const CLIMB_MAX: i64 = 64;
/// The levels a map may have, above and below the ground.
const LEVEL_SPAN: i64 = 16;
/// The lines a note's field shows.
const NOTE_LINES: u8 = 4;

const JOINS: [Option<Join>; 5] = [
    None,
    Some(Join::Up),
    Some(Join::Down),
    Some(Join::Ladder),
    Some(Join::Hole),
];

const DISPOSITIONS: [Disposition; 4] = [
    Disposition::Hostile,
    Disposition::Wary,
    Disposition::Neutral,
    Disposition::Friendly,
];
const RESTS: [RestKind; 3] = [RestKind::Short, RestKind::Long, RestKind::Any];

fn facing_name(facing: Facing) -> String {
    format!("{facing:?}").to_lowercase()
}

fn facings(at: Facing) -> form::Field {
    let options = Facing::ALL.iter().map(|f| facing_name(*f)).collect();
    let i = Facing::ALL.iter().position(|f| *f == at).unwrap_or(0);
    form::choice("facing", options, i)
}

fn facing_of(form: &Form, label: &str) -> Facing {
    Facing::ALL
        .get(form.chosen_of(label))
        .copied()
        .unwrap_or(Facing::North)
}

fn dispositions(at: Disposition) -> form::Field {
    let options = DISPOSITIONS.iter().map(|d| format!("{d:?}")).collect();
    let i = DISPOSITIONS.iter().position(|d| *d == at).unwrap_or(0);
    form::choice("disposition", options, i)
}

/// `options` sorted, with `held` among them even when the loaded packs lack it (so editing keeps
/// what the file says), and where `held` stands.
fn among(mut options: Vec<String>, held: Option<&str>) -> (Vec<String>, usize) {
    options.sort();
    options.dedup();
    if let Some(h) = held
        && !options.iter().any(|o| o == h)
    {
        options.push(h.to_owned());
    }
    let at = held
        .and_then(|h| options.iter().position(|o| o == h))
        .unwrap_or(0);
    (options, at)
}

fn map_choice(data: &Data, held: Option<&str>) -> form::Field {
    let maps = data.maps.values().map(|m| m.def.id.clone()).collect();
    let (options, at) = among(maps, held);
    form::choice("to map", options, at)
}

fn coordinate(label: &'static str, value: u16, side: u16) -> form::Field {
    form::number(
        label,
        i64::from(value),
        0,
        i64::from(side.saturating_sub(1).max(1)),
    )
}

fn u16_of(n: i64) -> u16 {
    u16::try_from(n).unwrap_or(0)
}

fn u8_of(n: i64) -> u8 {
    u8::try_from(n).unwrap_or(0)
}

fn i8_of(n: i64) -> i8 {
    i8::try_from(n).unwrap_or(0)
}

/// A text that is none when empty.
fn optional(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The rows of a list of (id, count) pairs, as the template's values.
fn pairs<N: Copy + Into<i64>>(items: &[(String, N)]) -> Vec<Vec<Value>> {
    items
        .iter()
        .map(|(id, n)| {
            vec![
                Value::Text {
                    text: id.clone(),
                    max: ID_MAX,
                    lines: 1,
                },
                Value::Number {
                    value: (*n).into(),
                    min: 1,
                    max: ITEMS_MAX,
                },
            ]
        })
        .collect()
}

/// A list of (id, count) rows, at most `rows` of them.
fn pair_list(label: &'static str, what: &'static str, max: i64, rows: u8) -> List {
    List {
        label,
        template: vec![
            form::text(what, "", ID_MAX),
            form::number("count", 1, 1, max),
        ],
        max: rows,
    }
}

/// An id for a new thing at `(x, y)` that nothing on the map has yet: `<what>_<x>_<y>`, then
/// `_2`, `_3` and so on.
fn free_id(what: &str, (x, y): (u16, u16), taken: impl Fn(&str) -> bool) -> String {
    let base = format!("{what}_{x}_{y}");
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

/// The (id, count) pairs of list `l`, each id one word.
fn read_pairs<N: TryFrom<i64>>(form: &Form, l: u8) -> Result<Vec<(String, N)>, String> {
    form.rows_of(l)
        .iter()
        .map(|row| {
            let id = form::id(form.text(row[0]))?;
            let n = form.fields.get(row[1]).map_or(1, form::Field::number);
            let n = N::try_from(n).map_err(|_| format!("{n} is too many"))?;
            Ok((id, n))
        })
        .collect()
}

/// Creatures and their dice as one line: `goblin 1d4, wolf 2d6`.
fn creatures_line(stacks: &[(String, Dice)]) -> String {
    stacks
        .iter()
        .map(|(id, d)| format!("{id} {d}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The creatures and dice of a line.
fn read_creatures(line: &str) -> Result<Vec<(String, Dice)>, String> {
    line.split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let (id, dice) = part.trim().split_once(char::is_whitespace).ok_or_else(|| {
                format!("a creature and its dice, like goblin 1d4, not \"{part}\"")
            })?;
            Ok((form::id(id)?, form::dice(dice)?))
        })
        .collect()
}

/// The portal, encounter, site, rest event, object, pile or lore of `def` that a key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found<'a> {
    /// A portal.
    Portal(&'a Portal),
    /// A fixed encounter.
    Encounter(&'a FixedEncounter),
    /// A service's site.
    Site(&'a Site),
    /// A rest event.
    RestEvent(&'a RestEventDef),
    /// A placed object.
    Object(&'a Placement),
    /// A ground pile.
    Ground(&'a GroundDef),
    /// A cell's lore.
    Lore(&'a CellLore),
}

/// What `key` names in `def`, if it is there.
#[must_use]
pub fn find<'a>(def: &'a MapDef, key: &Target) -> Option<Found<'a>> {
    match key {
        Target::Portal { level, x, y } => def
            .portals
            .iter()
            .find(|p| p.level == *level && p.x == *x && p.y == *y)
            .map(Found::Portal),
        Target::Encounter { id } => def
            .encounters
            .iter()
            .find(|e| e.id == *id)
            .map(Found::Encounter),
        Target::Site { level, x, y } => def
            .sites
            .iter()
            .find(|s| s.level == *level && s.x == *x && s.y == *y)
            .map(Found::Site),
        Target::RestEvent { terrain, rest } => def
            .rest_events
            .iter()
            .find(|r| r.terrain == *terrain && r.rest == *rest)
            .map(Found::RestEvent),
        Target::Object { id } => def.objects.iter().find(|o| o.id == *id).map(Found::Object),
        Target::Ground { level, x, y } => def
            .ground
            .iter()
            .find(|g| g.level == *level && g.x == *x && g.y == *y)
            .map(Found::Ground),
        Target::Lore { id } => def.lore.iter().find(|l| l.id == *id).map(Found::Lore),
    }
}

impl Form {
    /// A form to place a new `kind` at `at`.
    #[must_use]
    pub fn placing(kind: Placing, def: &MapDef, data: &Data, at: At) -> Form {
        match kind {
            Placing::Portal => portal(data, at, None),
            Placing::Encounter => encounter(def, data, at, None),
            Placing::Site => site(data, at, None),
            Placing::Object => object(def, data, at, None),
            Placing::Ground => ground(at, None),
            Placing::Lore => lore(def, at, None),
        }
    }

    /// A form to edit what `key` names, filled from it; none when it is not there.
    #[must_use]
    pub fn existing(key: &Target, def: &MapDef, data: &Data, facing: Facing) -> Option<Form> {
        let at = |level: Level, x: u16, y: u16| At {
            level,
            x,
            y,
            facing,
        };
        let mut form = match find(def, key)? {
            Found::Portal(p) => portal(data, at(p.level, p.x, p.y), Some(p)),
            Found::Encounter(e) => encounter(def, data, at(e.level, e.x, e.y), Some(e)),
            Found::Site(s) => site(data, at(s.level, s.x, s.y), Some(s)),
            Found::RestEvent(r) => rest_event(def, at(Level::GROUND, 0, 0), Some(r)),
            Found::Object(o) => object(def, data, at(o.level, o.x, o.y), Some(o)),
            Found::Ground(g) => ground(at(g.level, g.x, g.y), Some(g)),
            Found::Lore(l) => lore(def, at(l.level, l.x, l.y), Some(l)),
        };
        form.remove = Some(Change::Place(Place::Remove(key.clone())));
        Some(form)
    }

    /// A form for a terrain of `def`: the one with `glyph`, or a new one.
    #[must_use]
    pub fn terrain(def: &MapDef, glyph: Option<char>, at: At) -> Form {
        let was = glyph.and_then(|g| def.terrains.iter().find(|t| t.glyph == g));
        terrain(was, at)
    }

    /// A form for a new level of `def`: the first free number above the ground offered.
    #[must_use]
    pub fn level_add(def: &MapDef, at: At) -> Form {
        let free = (1..=i8::try_from(LEVEL_SPAN).unwrap_or(i8::MAX))
            .find(|n| !def.levels.iter().any(|l| l.level == *n))
            .unwrap_or(1);
        Form::new(
            FormKind::LevelAdd,
            "New level".to_owned(),
            at,
            vec![
                form::number("level", i64::from(free), -LEVEL_SPAN, LEVEL_SPAN),
                form::text("name", "", ID_MAX),
                form::number("base", 0, -i64::from(MAX_HEIGHT), i64::from(MAX_HEIGHT)),
            ],
            Vec::new(),
        )
    }

    /// A form for level `level` of `def` (never the ground, which is the map itself); none when
    /// the map has no such level.
    #[must_use]
    pub fn level(def: &MapDef, level: i8, at: At) -> Option<Form> {
        let was: &LevelDef = def.levels.iter().find(|l| l.level == level)?;
        let mut form = Form::new(
            FormKind::Level(level),
            format!("Level {level}"),
            at,
            vec![
                form::text("name", &was.name, ID_MAX),
                form::number(
                    "base",
                    i64::from(was.base),
                    -i64::from(MAX_HEIGHT),
                    i64::from(MAX_HEIGHT),
                ),
            ],
            Vec::new(),
        );
        form.remove = Some(Change::Edit(Edit::LevelRemove { level }));
        Some(form)
    }

    /// A form for the author's note on `key`, headed `heading`, holding `text` when there is
    /// one.
    #[must_use]
    pub fn note(key: String, heading: String, text: Option<&str>, at: At) -> Form {
        let mut form = Form::new(
            FormKind::Note(key.clone()),
            heading,
            at,
            vec![form::lines(
                "note",
                text.unwrap_or(""),
                MAX_STRING_BYTES,
                NOTE_LINES,
            )],
            Vec::new(),
        );
        form.hint = Some("Written to the pack at once; not undone.");
        if text.is_some() {
            form.remove = Some(Change::Note { key, text: None });
        }
        form
    }

    /// The form of the map's random encounter table, filled from it when there is one.
    #[must_use]
    pub fn random(def: &MapDef, at: At) -> Form {
        random(def.random.as_ref(), at)
    }

    /// A form for a new rest event of the map.
    #[must_use]
    pub fn rest_event(def: &MapDef, at: At) -> Form {
        rest_event(def, at, None)
    }

    /// The id of what the form edits, when it edits something named (from what Remove sends).
    fn original_id(&self) -> Option<&str> {
        match &self.remove {
            Some(Change::Place(Place::Remove(
                Target::Encounter { id } | Target::Object { id } | Target::Lore { id },
            ))) => Some(id),
            _ => None,
        }
    }

    /// The loader's keys, checked before the document's upsert would replace something
    /// silently: an id another encounter, object or lore has, a second encounter on a cell.
    fn keys(&self, def: &MapDef) -> Result<(), String> {
        let original = self.original_id();
        let other = |id: &str| original != Some(id);
        match self.kind {
            FormKind::Encounter => {
                let id = form::id(self.text_of("id"))?;
                if def.encounters.iter().any(|e| e.id == id && other(&e.id)) {
                    return Err(format!("an encounter {id} is already on the map"));
                }
                let (x, y) = (u16_of(self.number_of("x")), u16_of(self.number_of("y")));
                if def
                    .encounters
                    .iter()
                    .any(|e| other(&e.id) && e.level == self.at.level && e.x == x && e.y == y)
                {
                    return Err(format!("cell ({x}, {y}) already has an encounter"));
                }
            }
            FormKind::Object => {
                let id = form::id(self.text_of("id"))?;
                if def.objects.iter().any(|o| o.id == id && other(&o.id)) {
                    return Err(format!("an object {id} is already on the map"));
                }
            }
            FormKind::Lore => {
                let id = form::id(self.text_of("id"))?;
                if def.lore.iter().any(|l| l.id == id && other(&l.id)) {
                    return Err(format!("a lore {id} is already on the map"));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The request this form's Apply makes.
    ///
    /// # Errors
    /// A field that does not read, or a key the map has: said in one line, the form standing.
    pub fn request(&self, def: &MapDef) -> Result<Change, String> {
        let at = self.at;
        self.keys(def)?;
        match &self.kind {
            FormKind::Terrain(glyph) => {
                let terrain = self.terrain_fields()?;
                return Ok(Change::Edit(match glyph {
                    Some(glyph) => Edit::TerrainSet {
                        glyph: *glyph,
                        terrain,
                    },
                    None => Edit::TerrainAdd(terrain),
                }));
            }
            FormKind::LevelAdd => {
                return Ok(Change::Edit(Edit::LevelAdd {
                    level: i8_of(self.number_of("level")),
                    name: optional(self.text_of("name")).ok_or("a level needs its name key")?,
                    base: i16::try_from(self.number_of("base")).unwrap_or(0),
                }));
            }
            FormKind::Level(level) => {
                return Ok(Change::Edit(Edit::LevelSet {
                    level: *level,
                    name: Some(optional(self.text_of("name")).ok_or("a level needs its name key")?),
                    base: Some(i16::try_from(self.number_of("base")).unwrap_or(0)),
                }));
            }
            FormKind::Note(key) => {
                return Ok(Change::Note {
                    key: key.clone(),
                    text: optional(self.text_of("note")),
                });
            }
            _ => {}
        }
        Ok(Change::Place(match self.kind {
            FormKind::Portal => Place::Portal(Portal {
                level: at.level,
                x: at.x,
                y: at.y,
                to_map: self.choice_of("to map").to_owned(),
                to_level: Level(i8_of(self.number_of("to level"))),
                to_x: u16_of(self.number_of("to x")),
                to_y: u16_of(self.number_of("to y")),
                to_facing: facing_of(self, "facing"),
                marker: Some(optional(self.text_of("marker")).ok_or("a portal needs a marker")?),
            }),
            FormKind::Encounter => Place::Encounter(FixedEncounter {
                id: form::id(self.text_of("id"))?,
                level: at.level,
                x: u16_of(self.number_of("x")),
                y: u16_of(self.number_of("y")),
                stacks: read_pairs(self, 0)?,
                disposition: DISPOSITIONS
                    .get(self.chosen_of("disposition"))
                    .copied()
                    .unwrap_or_default(),
                once: self.flag_of("once"),
                stand: self.flag_of("stands apart").then(|| Stand {
                    pos: (
                        i8_of(self.number_of("stand x")),
                        i8_of(self.number_of("stand y")),
                    ),
                    facing: facing_of(self, "facing"),
                }),
            }),
            FormKind::Random => {
                let entries = self
                    .rows_of(0)
                    .iter()
                    .map(|row| {
                        let field = |i: usize| self.fields.get(row[i]);
                        Ok(RandomEntry {
                            weight: u16_of(field(0).map_or(1, form::Field::number)),
                            disposition: DISPOSITIONS
                                .get(field(1).map_or(0, form::Field::chosen))
                                .copied()
                                .unwrap_or_default(),
                            stacks: read_creatures(field(2).map_or("", form::Field::text))?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Place::Random(Some(RandomEncounters {
                    chance_percent: u8_of(self.number_of("chance percent")),
                    entries,
                }))
            }
            FormKind::Site => Place::Site(Site {
                level: at.level,
                x: at.x,
                y: at.y,
                service: self.choice_of("service").to_owned(),
            }),
            FormKind::RestEvent => {
                let terrain = def
                    .terrains
                    .get(self.chosen_of("terrain"))
                    .map(|t| t.name.clone())
                    .or_else(|| optional(self.text_of("terrain")))
                    .ok_or("the map has no terrains")?;
                Place::RestEvent(RestEventDef {
                    terrain,
                    rest: RESTS
                        .get(self.chosen_of("rest"))
                        .copied()
                        .unwrap_or(RestKind::Any),
                    chance: u16_of(self.number_of("chance per mille")),
                    text: optional(self.text_of("text")).ok_or("a rest event needs its text")?,
                })
            }
            FormKind::Object => {
                let dc = u8_of(self.number_of("lore DC"));
                Place::Object(Placement {
                    id: form::id(self.text_of("id"))?,
                    kind: self.choice_of("kind").to_owned(),
                    level: at.level,
                    x: u16_of(self.number_of("x")),
                    y: u16_of(self.number_of("y")),
                    pos: (
                        i8_of(self.number_of("pos x")),
                        i8_of(self.number_of("pos y")),
                    ),
                    facing: facing_of(self, "facing"),
                    state: optional(self.text_of("state")),
                    hidden: Some(u8_of(self.number_of("hidden"))).filter(|h| *h > 0),
                    name: optional(self.text_of("name")),
                    contents: read_pairs(self, 0)?,
                    lore: optional(self.text_of("lore")).map(|text| Lore {
                        text,
                        dc: (dc > 0).then_some(dc),
                        once: self.flag_of("lore once"),
                    }),
                })
            }
            FormKind::Ground => Place::Ground(GroundDef {
                level: at.level,
                x: at.x,
                y: at.y,
                pos: (
                    i8_of(self.number_of("pos x")),
                    i8_of(self.number_of("pos y")),
                ),
                items: read_pairs(self, 0)?,
            }),
            FormKind::Lore => {
                let dc = u8_of(self.number_of("DC"));
                Place::Lore(CellLore {
                    id: form::id(self.text_of("id"))?,
                    level: at.level,
                    x: u16_of(self.number_of("x")),
                    y: u16_of(self.number_of("y")),
                    text: optional(self.text_of("text")).ok_or("a lore needs its text")?,
                    dc: (dc > 0).then_some(dc),
                    once: self.flag_of("once"),
                })
            }
            FormKind::Terrain(_) | FormKind::LevelAdd | FormKind::Level(_) | FormKind::Note(_) => {
                unreachable!("answered above")
            }
        }))
    }

    /// The terrain the form's fields describe.
    fn terrain_fields(&self) -> Result<Terrain, String> {
        let mut glyphs = self.text_of("glyph").trim().chars();
        let glyph = match (glyphs.next(), glyphs.next()) {
            (Some(g), None) => g,
            _ => return Err("a glyph is one character".to_owned()),
        };
        let colour = |c: &str| u8_of(self.number_of(c));
        Ok(Terrain {
            glyph,
            name: optional(self.text_of("name")).ok_or("a terrain needs a name")?,
            floor: optional(self.text_of("floor")).ok_or("a terrain needs its floor surface")?,
            ceiling: optional(self.text_of("ceiling")),
            block: optional(self.text_of("block")),
            passable: self.flag_of("passable"),
            opaque: self.flag_of("opaque"),
            color: (colour("red"), colour("green"), colour("blue")),
            visibility_depth: u8_of(self.number_of("visibility depth")),
            step_minutes: u32::try_from(self.number_of("step minutes")).unwrap_or(1),
            climb: u16_of(self.number_of("climb")),
            join: JOINS.get(self.chosen_of("join")).copied().flatten(),
        })
    }
}

fn terrain(was: Option<&Terrain>, at: At) -> Form {
    let join_at = JOINS
        .iter()
        .position(|j| *j == was.and_then(|t| t.join))
        .unwrap_or(0);
    let colour = |pick: fn(&Terrain) -> u8| was.map_or(128, |t| i64::from(pick(t)));
    let mut form = Form::new(
        FormKind::Terrain(was.map(|t| t.glyph)),
        was.map_or_else(
            || "New terrain".to_owned(),
            |t| format!("Terrain {} {}", t.glyph, t.name),
        ),
        at,
        vec![
            form::text(
                "glyph",
                &was.map_or(String::new(), |t| t.glyph.to_string()),
                4,
            ),
            form::text("name", was.map_or("", |t| t.name.as_str()), ID_MAX),
            form::text("floor", was.map_or("", |t| t.floor.as_str()), ID_MAX),
            form::text(
                "ceiling",
                was.and_then(|t| t.ceiling.as_deref()).unwrap_or(""),
                ID_MAX,
            ),
            form::text(
                "block",
                was.and_then(|t| t.block.as_deref()).unwrap_or(""),
                ID_MAX,
            ),
            form::flag("passable", was.is_none_or(|t| t.passable)),
            form::flag("opaque", was.is_some_and(|t| t.opaque)),
            form::number("red", colour(|t| t.color.0), 0, 255),
            form::number("green", colour(|t| t.color.1), 0, 255),
            form::number("blue", colour(|t| t.color.2), 0, 255),
            form::number(
                "visibility depth",
                was.map_or(6, |t| i64::from(t.visibility_depth)),
                1,
                i64::from(MAX_VISIBILITY_DEPTH),
            ),
            form::number(
                "step minutes",
                was.map_or(1, |t| i64::from(t.step_minutes)),
                0,
                STEP_MINUTES_MAX,
            ),
            form::number("climb", was.map_or(2, |t| i64::from(t.climb)), 0, CLIMB_MAX),
            form::choice(
                "join",
                JOINS
                    .iter()
                    .map(|j| j.map_or_else(|| "none".to_owned(), |j| format!("{j:?}")))
                    .collect(),
                join_at,
            ),
        ],
        Vec::new(),
    );
    if let Some(t) = was {
        form.remove = Some(Change::Edit(Edit::TerrainRemove { glyph: t.glyph }));
    }
    form
}

fn portal(data: &Data, at: At, was: Option<&Portal>) -> Form {
    let side = i64::from(MAX_MAP_SIDE - 1);
    Form::new(
        FormKind::Portal,
        format!("Portal at ({}, {})", at.x, at.y),
        at,
        vec![
            map_choice(data, was.map(|p| p.to_map.as_str())),
            form::number(
                "to level",
                was.map_or(0, |p| i64::from(p.to_level.0)),
                -16,
                16,
            ),
            form::number("to x", was.map_or(0, |p| i64::from(p.to_x)), 0, side),
            form::number("to y", was.map_or(0, |p| i64::from(p.to_y)), 0, side),
            facings(was.map_or(at.facing, |p| p.to_facing)),
            form::text(
                "marker",
                was.and_then(|p| p.marker.as_deref()).unwrap_or(""),
                ID_MAX,
            ),
        ],
        Vec::new(),
    )
}

/// An encounter's form. A new one starts valid (owner, 2026-10-24): an id nothing on the map
/// has, one stack of the first loaded monster, hostile; Apply at once gives a map the loader
/// takes.
fn encounter(def: &MapDef, data: &Data, at: At, was: Option<&FixedEncounter>) -> Form {
    let stand = was.and_then(|e| e.stand.as_ref());
    let offset = i64::from(MAX_OFFSET);
    let id = was.map_or_else(
        || {
            free_id("encounter", (at.x, at.y), |id| {
                def.encounters.iter().any(|e| e.id == id)
            })
        },
        |e| e.id.clone(),
    );
    let mut form = Form::new(
        FormKind::Encounter,
        was.map_or_else(
            || format!("Encounter at ({}, {})", at.x, at.y),
            |e| format!("Encounter {}", e.id),
        ),
        at,
        vec![
            form::text("id", &id, ID_MAX),
            coordinate("x", at.x, def.width),
            coordinate("y", at.y, def.height),
            dispositions(was.map_or(Disposition::default(), |e| e.disposition)),
            form::flag("once", was.is_some_and(|e| e.once)),
            form::flag("stands apart", stand.is_some()),
            form::number(
                "stand x",
                stand.map_or(0, |s| i64::from(s.pos.0)),
                -offset,
                offset,
            ),
            form::number(
                "stand y",
                stand.map_or(0, |s| i64::from(s.pos.1)),
                -offset,
                offset,
            ),
            facings(stand.map_or(at.facing, |s| s.facing)),
        ],
        vec![pair_list(
            "Stacks",
            "creature",
            COUNT_MAX,
            u8::try_from(MAX_STACKS).unwrap_or(u8::MAX),
        )],
    );
    match was {
        Some(e) => form.fill(0, pairs(&e.stacks)),
        None => {
            if let Some(first) = data.monsters.values().map(|m| m.id.clone()).min() {
                form.fill(0, pairs(&[(first, 1_u8)]));
            }
        }
    }
    form
}

fn random(was: Option<&RandomEncounters>, at: At) -> Form {
    let mut form = Form::new(
        FormKind::Random,
        "Random encounters".to_owned(),
        at,
        vec![form::number(
            "chance percent",
            was.map_or(0, |r| i64::from(r.chance_percent)),
            0,
            100,
        )],
        vec![List {
            label: "Entries",
            template: vec![
                form::number("weight", 1, 1, WEIGHT_MAX),
                dispositions(Disposition::default()),
                form::text("creatures", "", CREATURES_MAX),
            ],
            max: form::MAX_ROWS,
        }],
    );
    if let Some(r) = was {
        let rows = r
            .entries
            .iter()
            .map(|e| {
                vec![
                    Value::Number {
                        value: i64::from(e.weight),
                        min: 1,
                        max: WEIGHT_MAX,
                    },
                    dispositions(e.disposition).value,
                    Value::Text {
                        text: creatures_line(&e.stacks),
                        max: CREATURES_MAX,
                        lines: 1,
                    },
                ]
            })
            .collect();
        form.fill(0, rows);
        form.remove = Some(Change::Place(Place::Random(None)));
    }
    form
}

fn site(data: &Data, at: At, was: Option<&Site>) -> Form {
    let services = data.services.values().map(|s| s.id.clone()).collect();
    let (options, chosen) = among(services, was.map(|s| s.service.as_str()));
    Form::new(
        FormKind::Site,
        format!("Site at ({}, {})", at.x, at.y),
        at,
        vec![form::choice("service", options, chosen)],
        Vec::new(),
    )
}

fn rest_event(def: &MapDef, at: At, was: Option<&RestEventDef>) -> Form {
    let terrains: Vec<String> = def.terrains.iter().map(|t| t.name.clone()).collect();
    let terrain_at = was
        .and_then(|r| terrains.iter().position(|t| *t == r.terrain))
        .unwrap_or(0);
    let rest_at = was
        .and_then(|r| RESTS.iter().position(|k| *k == r.rest))
        .unwrap_or(2);
    Form::new(
        FormKind::RestEvent,
        was.map_or_else(
            || "Rest event".to_owned(),
            |r| format!("Rest event: {} on {:?}", r.terrain, r.rest),
        ),
        at,
        vec![
            form::choice("terrain", terrains, terrain_at),
            form::choice(
                "rest",
                RESTS.iter().map(|k| format!("{k:?}")).collect(),
                rest_at,
            ),
            form::number(
                "chance per mille",
                was.map_or(0, |r| i64::from(r.chance)),
                0,
                PER_MILLE,
            ),
            form::text("text", was.map_or("", |r| r.text.as_str()), ID_MAX),
        ],
        Vec::new(),
    )
}

fn object(def: &MapDef, data: &Data, at: At, was: Option<&Placement>) -> Form {
    let kinds = data.objects.values().map(|k| k.id.clone()).collect();
    let (options, chosen) = among(kinds, was.map(|o| o.kind.as_str()));
    let lore = was.and_then(|o| o.lore.as_ref());
    let offset = i64::from(MAX_OFFSET);
    let mut form = Form::new(
        FormKind::Object,
        was.map_or_else(
            || format!("Object at ({}, {})", at.x, at.y),
            |o| format!("Object {}", o.id),
        ),
        at,
        vec![
            form::text("id", was.map_or("", |o| o.id.as_str()), ID_MAX),
            form::choice("kind", options, chosen),
            coordinate("x", at.x, def.width),
            coordinate("y", at.y, def.height),
            form::number(
                "pos x",
                was.map_or(0, |o| i64::from(o.pos.0)),
                -offset,
                offset,
            ),
            form::number(
                "pos y",
                was.map_or(0, |o| i64::from(o.pos.1)),
                -offset,
                offset,
            ),
            facings(was.map_or(at.facing, |o| o.facing)),
            form::text(
                "state",
                was.and_then(|o| o.state.as_deref()).unwrap_or(""),
                ID_MAX,
            ),
            form::number(
                "hidden",
                was.and_then(|o| o.hidden).map_or(0, i64::from),
                0,
                HIDDEN_MAX,
            ),
            form::text(
                "name",
                was.and_then(|o| o.name.as_deref()).unwrap_or(""),
                ID_MAX,
            ),
            form::text("lore", lore.map_or("", |l| l.text.as_str()), ID_MAX),
            form::number(
                "lore DC",
                lore.and_then(|l| l.dc).map_or(0, i64::from),
                0,
                DC_MAX,
            ),
            form::flag("lore once", lore.is_some_and(|l| l.once)),
        ],
        vec![pair_list("Contents", "item", ITEMS_MAX, form::MAX_ROWS)],
    );
    if let Some(o) = was {
        form.fill(0, pairs(&o.contents));
    }
    form
}

fn ground(at: At, was: Option<&GroundDef>) -> Form {
    let offset = i64::from(MAX_OFFSET);
    let mut form = Form::new(
        FormKind::Ground,
        format!("Ground pile at ({}, {})", at.x, at.y),
        at,
        vec![
            form::number(
                "pos x",
                was.map_or(0, |g| i64::from(g.pos.0)),
                -offset,
                offset,
            ),
            form::number(
                "pos y",
                was.map_or(0, |g| i64::from(g.pos.1)),
                -offset,
                offset,
            ),
        ],
        vec![pair_list("Items", "item", ITEMS_MAX, form::MAX_ROWS)],
    );
    if let Some(g) = was {
        form.fill(0, pairs(&g.items));
    }
    form
}

fn lore(def: &MapDef, at: At, was: Option<&CellLore>) -> Form {
    Form::new(
        FormKind::Lore,
        was.map_or_else(
            || format!("Lore at ({}, {})", at.x, at.y),
            |l| format!("Lore {}", l.id),
        ),
        at,
        vec![
            form::text("id", was.map_or("", |l| l.id.as_str()), ID_MAX),
            coordinate("x", at.x, def.width),
            coordinate("y", at.y, def.height),
            form::text("text", was.map_or("", |l| l.text.as_str()), ID_MAX),
            form::number("DC", was.and_then(|l| l.dc).map_or(0, i64::from), 0, DC_MAX),
            form::flag("once", was.is_some_and(|l| l.once)),
        ],
        Vec::new(),
    )
}
