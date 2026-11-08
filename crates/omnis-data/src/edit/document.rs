//! A map under edit: its definition with the grids held apart, one per level, every change a
//! step that can be undone. A change works on a copy and lands whole or not at all; a stroke
//! of paints is one step. Snapshots share the levels a change did not touch, so a hundred of
//! them on a large map cost a hundred grids, not a hundred maps. Every step carries the words
//! of what it did (`words`), for the history list (the editor's column, step 5): `steps`, `at`,
//! `goto` and `name_step`.

use crate::edit::grid::Grid;
use crate::edit::kinds::{Brush, Edit, EditError, MapFields, NewMap, Place, Target};
use crate::edit::words;
use crate::error::DataError;
use crate::levels::{self, LevelDef};
use crate::limits::{MAX_COLLECTION, MAX_LEVELS, MAX_MAP_SIDE, MAX_OBJECTS_PER_MAP};
use crate::loader::Data;
use crate::map::{Join, MapDef};
use crate::map_data::MapData;
use crate::stairs::StairDef;
use omnis_core::{Facing, Level};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// How many steps back an editor keeps.
pub const UNDO_CAP: usize = 100;

/// The map at one moment: its definition with every `layout` and `heights` empty, and the
/// grids that fill them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    def: MapDef,
    grids: BTreeMap<Level, Arc<Grid>>,
}

impl State {
    fn glyphs(&self) -> BTreeMap<char, u8> {
        self.def
            .terrains
            .iter()
            .enumerate()
            .filter_map(|(i, t)| u8::try_from(i).ok().map(|i| (t.glyph, i)))
            .collect()
    }

    fn grid_mut(&mut self, level: Level) -> Result<&mut Grid, EditError> {
        self.grids
            .get_mut(&level)
            .map(Arc::make_mut)
            .ok_or(EditError::NoLevel(level.0))
    }

    /// The definition as the file writes it.
    fn def(&self) -> MapDef {
        let order: Vec<char> = self.def.terrains.iter().map(|t| t.glyph).collect();
        let mut def = self.def.clone();
        if let Some(grid) = self.grids.get(&Level::GROUND) {
            def.layout = grid.layout(&order);
            def.heights = grid.heights();
        }
        for level in &mut def.levels {
            if let Some(grid) = self.grids.get(&Level(level.level)) {
                level.layout = grid.layout(&order);
                level.heights = grid.heights();
            }
        }
        def
    }
}

/// A map being edited.
#[derive(Debug, Clone)]
pub struct Document {
    file: String,
    state: State,
    saved: State,
    /// The state before each step done, with the step's words, oldest first.
    undo: Vec<(State, String)>,
    /// The state after each step undone, with its words; the next to redo last.
    redo: Vec<(State, String)>,
    /// The stroke of paints open for the next paint to join: its first paint's words and how
    /// many cells it has painted.
    stroke: Option<Stroke>,
}

/// An open stroke of paints: one step, its words counting the cells.
#[derive(Debug, Clone)]
struct Stroke {
    first: String,
    len: usize,
}

impl Document {
    /// A map read from its file (`file` is root-relative, for messages and the save). The
    /// loader's shape checks run first, so a map that does not lay out is refused in the
    /// loader's words; what needs other files (text keys, the tileset) is `validate`'s.
    ///
    /// # Errors
    /// The loader's errors.
    pub fn from_def(file: &str, mut def: MapDef) -> Result<Document, Vec<DataError>> {
        let path = Path::new(file);
        let mut errors = Vec::new();
        def.validate(path, &mut errors);
        levels::parse(&def, path, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }
        let glyphs: BTreeMap<char, u8> = def
            .terrains
            .iter()
            .enumerate()
            .filter_map(|(i, t)| u8::try_from(i).ok().map(|i| (t.glyph, i)))
            .collect();
        let mut grids = BTreeMap::new();
        let mut read = |level: Level, layout: &[String], heights: &[String]| match Grid::from_text(
            def.width, def.height, layout, heights, &glyphs,
        ) {
            Ok(grid) => {
                grids.insert(level, Arc::new(grid));
            }
            Err(message) => errors.push(DataError::new(path, format!("level {level}: {message}"))),
        };
        read(Level::GROUND, &def.layout, &def.heights);
        for level in &def.levels {
            read(Level(level.level), &level.layout, &level.heights);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        def.layout.clear();
        def.heights.clear();
        for level in &mut def.levels {
            level.layout.clear();
            level.heights.clear();
        }
        let state = State { def, grids };
        Ok(Document {
            file: file.to_owned(),
            saved: state.clone(),
            state,
            undo: Vec::new(),
            redo: Vec::new(),
            stroke: None,
        })
    }

    /// A new map, every tile its first terrain, walls round it, the start at the top-left
    /// facing east. Not yet saved: `saved` is empty so the document is dirty from the start.
    #[must_use]
    pub fn blank(file: &str, new: NewMap) -> Document {
        let def = MapDef {
            schema: crate::SCHEMA,
            id: new.id,
            name: new.name,
            kind: new.kind,
            tileset: new.tileset,
            width: new.width,
            height: new.height,
            start: (0, 0, Facing::East),
            start_level: Level::GROUND,
            wall: new.wall,
            door: new.door,
            door_open: new.door_open,
            terrains: vec![new.terrain],
            layout: Vec::new(),
            heights: Vec::new(),
            levels: Vec::new(),
            stairs: Vec::new(),
            portals: Vec::new(),
            encounters: Vec::new(),
            random: None,
            sites: Vec::new(),
            rest_events: Vec::new(),
            objects: Vec::new(),
            ground: Vec::new(),
            lore: Vec::new(),
        };
        let mut grids = BTreeMap::new();
        grids.insert(
            Level::GROUND,
            Arc::new(Grid::blank(new.width, new.height, Some(0))),
        );
        let state = State { def, grids };
        let mut saved = state.clone();
        saved.def.id.clear();
        Document {
            file: file.to_owned(),
            state,
            saved,
            undo: Vec::new(),
            redo: Vec::new(),
            stroke: None,
        }
    }

    /// The file the map is read from and saved to, root-relative.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// The map as the file would hold it now.
    #[must_use]
    pub fn def(&self) -> MapDef {
        self.state.def()
    }

    /// The grid of a level.
    #[must_use]
    pub fn grid(&self, level: Level) -> Option<&Grid> {
        self.state.grids.get(&level).map(Arc::as_ref)
    }

    /// The levels, lowest first.
    #[must_use]
    pub fn levels(&self) -> Vec<Level> {
        self.state.grids.keys().copied().collect()
    }

    /// Whether the map differs from what its file holds.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.state != self.saved
    }

    /// The file now holds the map as it is.
    pub fn mark_saved(&mut self) {
        self.saved = self.state.clone();
    }

    /// How many steps can be undone.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// How many steps can be redone.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    /// The bytes the grids of every snapshot hold, each grid counted once however many
    /// snapshots share it.
    #[must_use]
    pub fn grid_bytes(&self) -> usize {
        let mut seen: Vec<*const Grid> = Vec::new();
        let mut bytes = 0;
        for state in self
            .undo
            .iter()
            .chain(self.redo.iter())
            .map(|(state, _)| state)
            .chain([&self.state])
        {
            for grid in state.grids.values() {
                let ptr = Arc::as_ptr(grid);
                if !seen.contains(&ptr) {
                    seen.push(ptr);
                    bytes += grid.bytes();
                }
            }
        }
        bytes
    }

    /// What the loader would refuse: the map's own shape and layout, and with `data`, every
    /// reference against the loaded packs.
    #[must_use]
    pub fn validate(&self, data: Option<&Data>) -> Vec<DataError> {
        let def = self.def();
        match data {
            Some(data) => MapData::resolve(&def, Path::new(&self.file), data)
                .err()
                .unwrap_or_default(),
            None => {
                let path = Path::new(&self.file);
                let mut errors = Vec::new();
                crate::loader::check_id(&def.id, "map", path, &mut errors);
                def.validate(path, &mut errors);
                levels::parse(&def, path, &mut errors);
                errors
            }
        }
    }

    /// Apply `change` to a copy of the state; keep it as a new step named `words` (or as part
    /// of the open stroke, whose words become `words`) when it succeeds.
    fn change(
        &mut self,
        coalesce: bool,
        words: String,
        change: impl FnOnce(&mut State) -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        let mut next = self.state.clone();
        change(&mut next)?;
        if !coalesce {
            self.undo
                .push((std::mem::replace(&mut self.state, next), words));
            if self.undo.len() > UNDO_CAP {
                self.undo.remove(0);
            }
            self.redo.clear();
        } else {
            self.state = next;
            if let Some((_, top)) = self.undo.last_mut() {
                *top = words;
            }
        }
        Ok(())
    }

    /// Paint one tile or edge of `level`. With `stroke`, the paint joins the step the last paint
    /// opened, so a drag undoes as one; without it, the paint is a new step.
    ///
    /// # Errors
    /// The brush's rules: `NoLevel`, `Outside`, `UnknownGlyph`, `NeedsRise`, `BoundaryEdge`,
    /// `OutOfRange`.
    pub fn paint(
        &mut self,
        level: Level,
        x: u16,
        y: u16,
        brush: &Brush,
        stroke: bool,
    ) -> Result<(), EditError> {
        let own = words::brush(&self.state.def, level, x, y, brush);
        let open = self.stroke.take().filter(|_| stroke);
        let (first, len) = match open {
            Some(open) => (open.first, open.len + 1),
            None => (own, 1),
        };
        let words = if len > 1 {
            words::stroke(len, &first)
        } else {
            first.clone()
        };
        let result = self.change(len > 1, words, |state| {
            apply_brush(state, level, x, y, brush)
        });
        // A paint leaves a stroke open for the next paint to join; anything else closes it.
        self.stroke = result.is_ok().then_some(Stroke { first, len });
        result
    }

    /// Paint many tiles or edges of `level` as one step: every paint lands or none does (a
    /// wall run drawn by the editor's boundary tools). Closes any open stroke. An empty list
    /// changes nothing and is no step.
    ///
    /// # Errors
    /// The brush's rules, as `paint`.
    pub fn paint_many(
        &mut self,
        level: Level,
        paints: &[((u16, u16), Brush)],
    ) -> Result<(), EditError> {
        self.stroke = None;
        if paints.is_empty() {
            return Ok(());
        }
        self.change(false, words::many(paints.len(), 0), |state| {
            paints
                .iter()
                .try_for_each(|((x, y), brush)| apply_brush(state, level, *x, *y, brush))
        })
    }

    /// Put a placement on the map, replacing one with the same key, or remove one.
    ///
    /// # Errors
    /// `Outside` for a start off the map, `TooMany` at a collection's limit, `NoSuchTarget`
    /// for a removal that matches nothing.
    pub fn place(&mut self, place: Place) -> Result<(), EditError> {
        self.stroke = None;
        let words = words::place(&place);
        self.change(false, words, |state| apply_place(&mut state.def, place))
    }

    /// Place, replace or remove with every `Place` of `places` in order, as one step: every
    /// one lands or none does (a placement moved by the editor is a removal and a placement).
    /// Closes any open stroke. An empty list changes nothing and is no step.
    ///
    /// # Errors
    /// The places' rules, as `place`.
    pub fn place_many(&mut self, places: Vec<Place>) -> Result<(), EditError> {
        self.stroke = None;
        if places.is_empty() {
            return Ok(());
        }
        let words = match places.as_slice() {
            [one] => words::place(one),
            many => words::many(0, many.len()),
        };
        self.change(false, words, |state| {
            places
                .into_iter()
                .try_for_each(|place| apply_place(&mut state.def, place))
        })
    }

    /// Paint `paints` on `level` and then place every `Place` of `places`, in order, as one
    /// step: every one lands or none does (a selection moved by the editor: its cells' terrain,
    /// heights and sides, then the placements on them). Closes any open stroke. Nothing to do is
    /// no step.
    ///
    /// # Errors
    /// The brushes' rules, as `paint`, and the places', as `place`.
    pub fn batch(
        &mut self,
        level: Level,
        paints: &[((u16, u16), Brush)],
        places: Vec<Place>,
    ) -> Result<(), EditError> {
        self.stroke = None;
        if paints.is_empty() && places.is_empty() {
            return Ok(());
        }
        let words = words::many(paints.len(), places.len());
        self.change(false, words, |state| {
            paints
                .iter()
                .try_for_each(|((x, y), brush)| apply_brush(state, level, *x, *y, brush))?;
            places
                .into_iter()
                .try_for_each(|place| apply_place(&mut state.def, place))
        })
    }

    /// Change the map's structure: its size, its fields, a level or a terrain.
    ///
    /// # Errors
    /// The edit's rules, named by the variants of [`EditError`].
    pub fn edit(&mut self, edit: Edit) -> Result<(), EditError> {
        self.stroke = None;
        let words = words::edit(&edit);
        self.change(false, words, |state| apply_edit(state, edit))
    }

    /// Back one step.
    ///
    /// # Errors
    /// `NothingToUndo`.
    pub fn undo(&mut self) -> Result<(), EditError> {
        self.stroke = None;
        let (previous, words) = self.undo.pop().ok_or(EditError::NothingToUndo)?;
        self.redo
            .push((std::mem::replace(&mut self.state, previous), words));
        Ok(())
    }

    /// Forward one step.
    ///
    /// # Errors
    /// `NothingToUndo`.
    pub fn redo(&mut self) -> Result<(), EditError> {
        self.stroke = None;
        let (next, words) = self.redo.pop().ok_or(EditError::NothingToUndo)?;
        self.undo
            .push((std::mem::replace(&mut self.state, next), words));
        Ok(())
    }

    /// The words of every step the history holds, oldest first: the steps done, then the
    /// steps undone (the next to redo first).
    #[must_use]
    pub fn steps(&self) -> Vec<&str> {
        self.undo
            .iter()
            .map(|(_, words)| words.as_str())
            .chain(self.redo.iter().rev().map(|(_, words)| words.as_str()))
            .collect()
    }

    /// How many of the steps are done: the map stands after step `at` (0 is the map as
    /// opened, or as it stood before the oldest step kept).
    #[must_use]
    pub fn at(&self) -> usize {
        self.undo.len()
    }

    /// Undo or redo until the map stands after step `at` of `steps`.
    ///
    /// # Errors
    /// `NothingToUndo` when `at` is past the last step; nothing moves.
    pub fn goto(&mut self, at: usize) -> Result<(), EditError> {
        if at > self.undo.len() + self.redo.len() {
            return Err(EditError::NothingToUndo);
        }
        while self.undo.len() > at {
            self.undo()?;
        }
        while self.undo.len() < at {
            self.redo()?;
        }
        Ok(())
    }

    /// Name the newest step done `words` (the editor's own words for what it did), clipped as
    /// every step's are; nothing when no step is done.
    pub fn name_step(&mut self, words: &str) {
        if let Some((_, top)) = self.undo.last_mut() {
            *top = words::clip(words);
        }
    }
}

/// Paint one tile or edge of `level` on `state`: a tile (a stair rising `rise`, the stairs
/// list following), an edge, a height or no ground.
fn apply_brush(
    state: &mut State,
    level: Level,
    x: u16,
    y: u16,
    brush: &Brush,
) -> Result<(), EditError> {
    match brush {
        Brush::Tile { glyph, rise } => {
            let index = *state
                .glyphs()
                .get(glyph)
                .ok_or(EditError::UnknownGlyph(*glyph))?;
            let stair = matches!(
                state.def.terrains[usize::from(index)].join,
                Some(Join::Up | Join::Down)
            );
            let rise = match (stair, rise) {
                (true, None) => return Err(EditError::NeedsRise),
                (true, Some(rise)) => Some(*rise),
                (false, _) => None,
            };
            state.grid_mut(level)?.set_tile(x, y, Some(index))?;
            drop_stair(&mut state.def, level, x, y);
            if let Some(rise) = rise {
                state.def.stairs.push(StairDef {
                    level: level.0,
                    x,
                    y,
                    rise,
                });
            }
            Ok(())
        }
        Brush::Edge { side, kind } => state.grid_mut(level)?.set_edge(x, y, *side, *kind),
        Brush::Height { eighths } => state.grid_mut(level)?.set_height(x, y, *eighths),
        Brush::Clear => {
            state.grid_mut(level)?.set_tile(x, y, None)?;
            drop_stair(&mut state.def, level, x, y);
            Ok(())
        }
    }
}

/// Forget the stair entry on a tile, if any.
fn drop_stair(def: &mut MapDef, level: Level, x: u16, y: u16) {
    def.stairs
        .retain(|s| !(Level(s.level) == level && s.x == x && s.y == y));
}

/// Put or replace one entry of a list by its key, within a limit.
fn upsert<T>(
    list: &mut Vec<T>,
    item: T,
    same: impl Fn(&T, &T) -> bool,
    limit: usize,
    what: &'static str,
) -> Result<(), EditError> {
    if let Some(slot) = list.iter_mut().find(|t| same(t, &item)) {
        *slot = item;
        Ok(())
    } else if list.len() >= limit {
        Err(EditError::TooMany(what))
    } else {
        list.push(item);
        Ok(())
    }
}

/// Take the entries matching `gone` out of a list; `NoSuchTarget` when none did.
fn take<T>(list: &mut Vec<T>, gone: impl Fn(&T) -> bool, what: String) -> Result<(), EditError> {
    let before = list.len();
    list.retain(|t| !gone(t));
    if list.len() == before {
        Err(EditError::NoSuchTarget(what))
    } else {
        Ok(())
    }
}

fn apply_place(def: &mut MapDef, place: Place) -> Result<(), EditError> {
    let many = MAX_COLLECTION;
    match place {
        Place::Start {
            level,
            x,
            y,
            facing,
        } => {
            if x >= def.width || y >= def.height {
                return Err(EditError::Outside { x, y });
            }
            def.start = (x, y, facing);
            def.start_level = level;
            Ok(())
        }
        Place::Portal(p) => upsert(
            &mut def.portals,
            p,
            |a, b| (a.level, a.x, a.y) == (b.level, b.x, b.y),
            many,
            "portals",
        ),
        Place::Encounter(e) => upsert(
            &mut def.encounters,
            e,
            |a, b| a.id == b.id,
            many,
            "encounters",
        ),
        Place::Random(random) => {
            def.random = random;
            Ok(())
        }
        Place::Site(s) => upsert(
            &mut def.sites,
            s,
            |a, b| (a.level, a.x, a.y) == (b.level, b.x, b.y),
            many,
            "sites",
        ),
        Place::RestEvent(r) => upsert(
            &mut def.rest_events,
            r,
            |a, b| a.terrain == b.terrain && a.rest == b.rest,
            many,
            "rest events",
        ),
        Place::Object(o) => upsert(
            &mut def.objects,
            o,
            |a, b| a.id == b.id,
            MAX_OBJECTS_PER_MAP,
            "objects",
        ),
        Place::Ground(g) => upsert(
            &mut def.ground,
            g,
            |a, b| (a.level, a.x, a.y) == (b.level, b.x, b.y),
            many,
            "ground piles",
        ),
        Place::Lore(l) => upsert(&mut def.lore, l, |a, b| a.id == b.id, many, "lore"),
        Place::Remove(target) => remove(def, target),
    }
}

fn remove(def: &mut MapDef, target: Target) -> Result<(), EditError> {
    match target {
        Target::Portal { level, x, y } => take(
            &mut def.portals,
            |p| (p.level, p.x, p.y) == (level, x, y),
            format!("a portal at level {level} ({x}, {y})"),
        ),
        Target::Encounter { id } => take(
            &mut def.encounters,
            |e| e.id == id,
            format!("encounter '{id}'"),
        ),
        Target::Site { level, x, y } => take(
            &mut def.sites,
            |s| (s.level, s.x, s.y) == (level, x, y),
            format!("a site at level {level} ({x}, {y})"),
        ),
        Target::RestEvent { terrain, rest } => take(
            &mut def.rest_events,
            |r| r.terrain == terrain && r.rest == rest,
            format!("a {rest:?} rest event on '{terrain}'"),
        ),
        Target::Object { id } => take(&mut def.objects, |o| o.id == id, format!("object '{id}'")),
        Target::Ground { level, x, y } => take(
            &mut def.ground,
            |g| (g.level, g.x, g.y) == (level, x, y),
            format!("a pile at level {level} ({x}, {y})"),
        ),
        Target::Lore { id } => take(&mut def.lore, |l| l.id == id, format!("lore '{id}'")),
    }
}

const RESERVED: [char; 9] = ['-', '=', '|', ':', '/', '+', ' ', '*', '_'];

fn apply_edit(state: &mut State, edit: Edit) -> Result<(), EditError> {
    match edit {
        Edit::Resize { width, height } => resize(state, width, height),
        Edit::Map(fields) => {
            set_fields(&mut state.def, fields);
            Ok(())
        }
        Edit::LevelAdd { level, name, base } => {
            if level == 0 {
                return Err(EditError::LevelZero);
            }
            if state.grids.contains_key(&Level(level)) {
                return Err(EditError::LevelExists(level));
            }
            if state.grids.len() >= MAX_LEVELS {
                return Err(EditError::TooMany("levels"));
            }
            let (w, h) = (state.def.width, state.def.height);
            state
                .grids
                .insert(Level(level), Arc::new(Grid::blank(w, h, None)));
            state.def.levels.push(LevelDef {
                level,
                name,
                base,
                layout: Vec::new(),
                heights: Vec::new(),
            });
            state.def.levels.sort_by_key(|l| l.level);
            Ok(())
        }
        Edit::LevelSet { level, name, base } => {
            let l = state
                .def
                .levels
                .iter_mut()
                .find(|l| l.level == level)
                .ok_or(EditError::NoLevel(level))?;
            if let Some(name) = name {
                l.name = name;
            }
            if let Some(base) = base {
                l.base = base;
            }
            Ok(())
        }
        Edit::LevelRemove { level } => remove_level(state, level),
        Edit::TerrainAdd(terrain) => {
            check_glyph(state, terrain.glyph, None)?;
            if state.def.terrains.len() >= 256 {
                return Err(EditError::TooMany("terrains"));
            }
            state.def.terrains.push(terrain);
            Ok(())
        }
        Edit::TerrainSet { glyph, terrain } => set_terrain(state, glyph, terrain),
        Edit::TerrainRemove { glyph } => remove_terrain(state, glyph),
        Edit::Rise { level, x, y, rise } => {
            let stair = state
                .def
                .stairs
                .iter_mut()
                .find(|s| Level(s.level) == level && s.x == x && s.y == y)
                .ok_or(EditError::NotAStair)?;
            stair.rise = rise;
            Ok(())
        }
    }
}

fn set_fields(def: &mut MapDef, fields: MapFields) {
    if let Some(name) = fields.name {
        def.name = name;
    }
    if let Some(kind) = fields.kind {
        def.kind = kind;
    }
    if let Some(tileset) = fields.tileset {
        def.tileset = tileset;
    }
    if let Some(wall) = fields.wall {
        def.wall = wall;
    }
    if let Some(door) = fields.door {
        def.door = door;
    }
    if let Some(door_open) = fields.door_open {
        def.door_open = door_open;
    }
}

/// A glyph a terrain may take: not reserved, not another terrain's (`keep` is the index whose
/// glyph it may already be).
fn check_glyph(state: &State, glyph: char, keep: Option<u8>) -> Result<(), EditError> {
    if RESERVED.contains(&glyph) {
        return Err(EditError::Reserved(glyph));
    }
    match state.glyphs().get(&glyph) {
        Some(&index) if Some(index) != keep => Err(EditError::GlyphTaken(glyph)),
        _ => Ok(()),
    }
}

fn index_of(state: &State, glyph: char) -> Result<u8, EditError> {
    state
        .glyphs()
        .get(&glyph)
        .copied()
        .ok_or(EditError::UnknownGlyph(glyph))
}

fn in_use(state: &State, index: u8) -> bool {
    state.grids.values().any(|g| g.uses(index))
}

fn set_terrain(
    state: &mut State,
    glyph: char,
    terrain: crate::map::Terrain,
) -> Result<(), EditError> {
    let index = index_of(state, glyph)?;
    check_glyph(state, terrain.glyph, Some(index))?;
    let old = state.def.terrains[usize::from(index)].clone();
    if old.join != terrain.join && in_use(state, index) {
        return Err(EditError::TerrainInUse(glyph));
    }
    if old.name != terrain.name {
        for event in state
            .def
            .rest_events
            .iter_mut()
            .filter(|r| r.terrain == old.name)
        {
            event.terrain = terrain.name.clone();
        }
    }
    state.def.terrains[usize::from(index)] = terrain;
    Ok(())
}

fn remove_terrain(state: &mut State, glyph: char) -> Result<(), EditError> {
    let index = index_of(state, glyph)?;
    if in_use(state, index) {
        return Err(EditError::TerrainInUse(glyph));
    }
    let gone = state.def.terrains.remove(usize::from(index));
    for grid in state.grids.values_mut() {
        Arc::make_mut(grid).reindex_above(index);
    }
    state.def.rest_events.retain(|r| r.terrain != gone.name);
    Ok(())
}

fn remove_level(state: &mut State, level: i8) -> Result<(), EditError> {
    if level == 0 {
        return Err(EditError::LevelZero);
    }
    if state.grids.remove(&Level(level)).is_none() {
        return Err(EditError::NoLevel(level));
    }
    let gone = Level(level);
    let def = &mut state.def;
    def.levels.retain(|l| l.level != level);
    def.stairs.retain(|s| Level(s.level) != gone);
    def.portals.retain(|p| p.level != gone);
    def.encounters.retain(|e| e.level != gone);
    def.sites.retain(|s| s.level != gone);
    def.objects.retain(|o| o.level != gone);
    def.ground.retain(|g| g.level != gone);
    def.lore.retain(|l| l.level != gone);
    // The start cannot go with its level: it returns to the ground at its cell (the loader's
    // standing check names the cell if the ground has none there; W1 step 3).
    if def.start_level == gone {
        def.start_level = Level::GROUND;
    }
    Ok(())
}

fn resize(state: &mut State, width: u16, height: u16) -> Result<(), EditError> {
    if !(1..=MAX_MAP_SIDE).contains(&width) || !(1..=MAX_MAP_SIDE).contains(&height) {
        return Err(EditError::OutOfRange);
    }
    for (level, grid) in &mut state.grids {
        let fill = level.is_ground().then_some(0);
        Arc::make_mut(grid).resize(width, height, fill);
    }
    let def = &mut state.def;
    def.width = width;
    def.height = height;
    let inside = |x: u16, y: u16| x < width && y < height;
    def.start.0 = def.start.0.min(width - 1);
    def.start.1 = def.start.1.min(height - 1);
    def.stairs.retain(|s| inside(s.x, s.y));
    def.portals.retain(|p| inside(p.x, p.y));
    def.encounters.retain(|e| inside(e.x, e.y));
    def.sites.retain(|s| inside(s.x, s.y));
    def.objects.retain(|o| inside(o.x, o.y));
    def.ground.retain(|g| inside(g.x, g.y));
    def.lore.retain(|l| inside(l.x, l.y));
    Ok(())
}
