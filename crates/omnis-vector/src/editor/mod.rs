//! The editor's model (M5 Editor v1 from M5.4; ARCHITECTURE §8.2, `tasks/plans/m5-editor-v1.md`
//! §Client side): the document session over the pack being edited, the open map's level,
//! cursor and facing, the plan view, the brush (M5.5), the cursor's cell as the attribute bar
//! edits it (M5.6) and the acts the layer's controls ask for. Bevy-free: the shell
//! (`shell/editor.rs`) draws it and routes the pointer and the keys to it.

pub mod brush;
pub mod form;
pub mod forms;
pub mod plan;
pub mod view;

use brush::{Target, Tool};
use form::{At, Change, Form};
use forms::Placing;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::{
    Brush, Edit, EditError, EditorStatus, Grid, MapEntry, Place, Session, Target as PlaceKey,
};
use omnis_sim::omnis_data::{Data, MapData, MapDef};
use std::path::PathBuf;

/// What a control of the editor's layer asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditAct {
    /// The map choice: steps through the pack's map files.
    Map,
    /// Open the chosen map, dropping any unsaved change to the open one.
    Open,
    /// The level choice: steps through the open map's levels.
    Level,
    /// Zoom the plan in.
    ZoomIn,
    /// Zoom the plan out.
    ZoomOut,
    /// Put the cursor at the plan's middle.
    Centre,
    /// Turn the cursor's facing a quarter left.
    FaceLeft,
    /// Turn the cursor's facing a quarter right.
    FaceRight,
    /// Choose a brush of the palette, by its position (M5.5).
    Brush(u8),
    /// The height choice: steps the height brush's eighths.
    Eighths,
    /// The bar's terrain choice (M5.6): steps the cursor's cell through the terrains and no
    /// ground.
    CellTerrain,
    /// The bar's height choice: steps the cursor's cell's height by an eighth.
    CellHeight,
    /// The bar's rise choice: turns the way the stair on the cursor's cell climbs.
    CellRise,
    /// Put the map's start on the cursor's cell, facing as the cursor does (M5.6).
    SetStart,
    /// The inspector's choice of what Add places.
    PlaceKind,
    /// Open the form to place the chosen kind at the cursor.
    Add,
    /// Open the form of the placement at the cursor at this position of the inspector's list.
    EditAt(u8),
    /// Remove the placement at the cursor at this position of the inspector's list.
    RemoveAt(u8),
    /// Open the form of the map's random encounter table.
    Random,
    /// Open the form for a new rest event of the map.
    RestAdd,
    /// Open the form of the map's rest event at this position.
    RestEdit(u8),
    /// Remove the map's rest event at this position.
    RestRemove(u8),
    /// A field of the open form, by its position (`form::Form::fields`).
    Field(u8),
    /// Add a row to this list of the open form.
    ListAdd(u8),
    /// Remove this row of this list of the open form.
    ListRemove(u8, u8),
    /// Make the open form's change, one undo step.
    Apply,
    /// Close the open form, changing nothing.
    Cancel,
    /// Remove what the open form edits, one undo step, and close it.
    Discard,
    /// Open the form for a new terrain of the map (6c).
    TerrainAdd,
    /// Open the form of the brush's terrain.
    TerrainEdit,
    /// Open the form for a new level of the map.
    LevelAdd,
    /// Open the form of the shown level (never the ground, which is the map itself).
    LevelEdit,
    /// Open the form of the author's note on the cell, the level or the map.
    Note(NoteOf),
    /// Back one step.
    Undo,
    /// Forward one step.
    Redo,
    /// Write the map to its file and reseal the pack.
    Save,
    /// Reopen the map's file, dropping every unsaved change.
    Revert,
    /// Open the editor's menu.
    Menu,
}

/// Whose note: the cursor's cell's, the shown level's or the map's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteOf {
    /// The cursor's cell on the shown level: `<map>/<level>/<x>,<y>`.
    Cell,
    /// The shown level: `<map>/level/<n>`.
    Level,
    /// The map: its id.
    Map,
}

/// What the layer does after an act: nothing more, open the menu, or reload the packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// Nothing beyond the model's change.
    Nothing,
    /// Open the editor's menu.
    Menu,
    /// The map was written: the packs are read from disk again, so a new game walks it.
    Saved,
}

/// The editor as the layer reads and changes it.
#[derive(Debug)]
pub struct Editor {
    /// The document session: the pack, the open map, the notes.
    pub session: Session,
    /// The pack's map files, as listed when the editor opened.
    pub maps: Vec<MapEntry>,
    /// The map chosen in the tools column, an index into `maps`.
    pub chosen: usize,
    /// The level shown.
    pub level: Level,
    /// The cell the inspector describes and the preview looks from.
    pub cursor: (u16, u16),
    /// The way the preview looks, and the side an edge brush paints from the keys.
    pub facing: Facing,
    /// Where the plan looks.
    pub view: plan::View,
    /// The pane the plan was last laid out for, in pixels.
    pub pane: (u32, u32),
    /// The last refusal or outcome to show.
    pub message: Option<String>,
    /// The map as the loader last took it; what is derived (cliffs, a stair's rise) is read
    /// from here, and it stays while the document does not resolve.
    pub resolved: Option<MapData>,
    /// Counts every change the layer should redraw for.
    pub revision: u64,
    /// The brush a click paints with.
    pub tool: Tool,
    /// The height brush's height, in eighths.
    pub eighths: i16,
    /// The last target painted in the stroke the held button keeps open; none between strokes.
    pub stroke: Option<Target>,
    /// What a click would act on now: the aim under the pointer (`aim`), shown over the plan;
    /// none while the pointer is off the plan or over a control.
    pub hover: Option<Target>,
    /// The session's state against the packs, as of the last change (the loader's verdict is
    /// costly, so it is read here, never asked for each frame).
    pub status: EditorStatus,
    /// The form being filled at the top of the inspector, if one is (M5.6).
    pub form: Option<Form>,
    /// What the inspector's Add places: an index into `forms::Placing::ALL`.
    pub place_kind: usize,
}

/// A placement at the cursor, as the inspector lists it: its words and the key that names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Here {
    /// What it is, in a line.
    pub label: String,
    /// The key `Place::Remove` and the forms find it by.
    pub key: PlaceKey,
}

impl Editor {
    /// The editor over `roots` (the last is edited), listing its maps.
    ///
    /// # Errors
    /// The pack's manifest or notes do not read, or its maps cannot be listed.
    pub fn new(roots: Vec<PathBuf>) -> Result<Editor, EditError> {
        let session = Session::new(roots)?;
        let maps = session.maps()?;
        let status = EditorStatus {
            root: session.root().display().to_string(),
            ..EditorStatus::default()
        };
        Ok(Editor {
            session,
            maps,
            chosen: 0,
            level: Level::GROUND,
            cursor: (0, 0),
            facing: Facing::North,
            view: plan::View::default(),
            pane: (800, 600),
            message: None,
            resolved: None,
            revision: 0,
            tool: Tool::Clear,
            eighths: 0,
            stroke: None,
            hover: None,
            status,
            form: None,
            place_kind: 0,
        })
    }

    /// Open the map `id`, putting the cursor at its start, fitting the plan to the pane and
    /// taking its first terrain as the brush.
    ///
    /// # Errors
    /// The session's: no such map, or its layout does not read.
    pub fn open(&mut self, id: &str, data: &Data) -> Result<(), EditError> {
        self.session.open(id)?;
        if let Some(i) = self.maps.iter().position(|m| m.id == id) {
            self.chosen = i;
        }
        let def = self.def();
        let (start, size) = def.as_ref().map_or(((0, 0, Facing::North), (0, 0)), |d| {
            (d.start, (d.width, d.height))
        });
        self.level = Level::GROUND;
        self.cursor = (start.0, start.1);
        self.facing = start.2;
        self.view = plan::View::fitted(size, self.pane);
        self.tool = def.as_ref().map_or(Tool::Clear, Tool::first);
        self.stroke = None;
        self.message = None;
        self.resolved = None;
        self.form = None;
        self.changed(data);
        Ok(())
    }

    /// The document changed (by the layer or over the socket): read its status, resolve it
    /// again, keeping the last valid resolve and saying why when the loader refuses it, and
    /// ask for a redraw.
    pub fn changed(&mut self, data: &Data) {
        self.revision = self.revision.wrapping_add(1);
        self.status = self.session.status(data);
        let Some(def) = self.def() else {
            self.resolved = None;
            return;
        };
        match MapData::resolve(&def, data) {
            Ok(map) => self.resolved = Some(map),
            Err(errors) => {
                if let Some(first) = errors.first() {
                    self.message = Some(first.to_string());
                }
            }
        }
        if !self.levels().contains(&self.level) {
            self.level = Level::GROUND;
        }
        self.cursor = (
            self.cursor.0.min(def.width.saturating_sub(1)),
            self.cursor.1.min(def.height.saturating_sub(1)),
        );
    }

    /// The open map as its file would hold it.
    #[must_use]
    pub fn def(&self) -> Option<MapDef> {
        self.session.document().map(|d| d.def())
    }

    /// The open map's id.
    #[must_use]
    pub fn map_id(&self) -> Option<String> {
        self.session.document().map(|d| d.def().id)
    }

    /// The open map's size.
    #[must_use]
    pub fn size(&self) -> (u16, u16) {
        self.def().map_or((0, 0), |d| (d.width, d.height))
    }

    /// The open map's levels, lowest first.
    #[must_use]
    pub fn levels(&self) -> Vec<Level> {
        self.session
            .document()
            .map(|d| d.levels())
            .unwrap_or_default()
    }

    /// The shown level's grid.
    #[must_use]
    pub fn grid(&self) -> Option<&Grid> {
        self.session.document().and_then(|d| d.grid(self.level))
    }

    /// The cursor's cell's terrain, as an index into the map's list; none on no ground or
    /// with no map.
    #[must_use]
    pub fn cell_terrain(&self) -> Option<u8> {
        let (x, y) = self.cursor;
        self.grid().and_then(|g| g.tile(x, y).flatten())
    }

    /// The cursor's cell's height in eighths; 0 with no ground or no map.
    #[must_use]
    pub fn cell_height(&self) -> i16 {
        let (x, y) = self.cursor;
        self.grid().and_then(|g| g.height_at(x, y)).unwrap_or(0)
    }

    /// Where a form opened now places: the shown level, the cursor and its facing.
    #[must_use]
    pub fn at(&self) -> At {
        At {
            level: self.level,
            x: self.cursor.0,
            y: self.cursor.1,
            facing: self.facing,
        }
    }

    /// The placements on the cursor's cell of the shown level, in the file's order: portals,
    /// encounters, sites, objects, piles, lore.
    #[must_use]
    pub fn here(&self) -> Vec<Here> {
        let Some(def) = self.def() else {
            return Vec::new();
        };
        let (level, (x, y)) = (self.level, self.cursor);
        let is_here = |l: Level, px: u16, py: u16| l == level && px == x && py == y;
        let mut out = Vec::new();
        for p in def.portals.iter().filter(|p| is_here(p.level, p.x, p.y)) {
            out.push(Here {
                label: format!("Portal to {}", p.to_map),
                key: PlaceKey::Portal { level, x, y },
            });
        }
        for e in def.encounters.iter().filter(|e| is_here(e.level, e.x, e.y)) {
            out.push(Here {
                label: format!("Encounter {}", e.id),
                key: PlaceKey::Encounter { id: e.id.clone() },
            });
        }
        for s in def.sites.iter().filter(|s| is_here(s.level, s.x, s.y)) {
            out.push(Here {
                label: format!("Site {}", s.service),
                key: PlaceKey::Site { level, x, y },
            });
        }
        for o in def.objects.iter().filter(|o| is_here(o.level, o.x, o.y)) {
            out.push(Here {
                label: format!("Object {} ({})", o.id, o.kind),
                key: PlaceKey::Object { id: o.id.clone() },
            });
        }
        for _ in def.ground.iter().filter(|g| is_here(g.level, g.x, g.y)) {
            out.push(Here {
                label: "Ground pile".to_owned(),
                key: PlaceKey::Ground { level, x, y },
            });
        }
        for l in def.lore.iter().filter(|l| is_here(l.level, l.x, l.y)) {
            out.push(Here {
                label: format!("Lore {}", l.id),
                key: PlaceKey::Lore { id: l.id.clone() },
            });
        }
        out
    }

    /// The key of the author's note on `of`; none with no map open.
    #[must_use]
    pub fn note_key(&self, of: NoteOf) -> Option<String> {
        let id = self.map_id()?;
        let (x, y) = self.cursor;
        let level = self.level.0;
        Some(match of {
            NoteOf::Cell => format!("{id}/{level}/{x},{y}"),
            NoteOf::Level => format!("{id}/level/{level}"),
            NoteOf::Map => id,
        })
    }

    /// The author's note on `of`, if one is written.
    #[must_use]
    pub fn note(&self, of: NoteOf) -> Option<&str> {
        let key = self.note_key(of)?;
        self.session.notes().get(&key)
    }

    /// The way the stair on the cursor's cell climbs, when the cell is one.
    #[must_use]
    pub fn cell_rise(&self) -> Option<Facing> {
        let (x, y) = self.cursor;
        self.def().and_then(|def| {
            def.stairs
                .iter()
                .find(|s| Level(s.level) == self.level && s.x == x && s.y == y)
                .map(|s| s.rise)
        })
    }

    /// Put the cursor on `cell` when it is on the map.
    pub fn point(&mut self, cell: (u16, u16)) {
        let (w, h) = self.size();
        if cell.0 < w && cell.1 < h && self.cursor != cell {
            self.cursor = cell;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Move the cursor one cell `facing`, when that stays on the map.
    pub fn step(&mut self, facing: Facing) {
        let (dx, dy) = facing.delta();
        let next = (
            u16::try_from(i32::from(self.cursor.0) + dx).ok(),
            u16::try_from(i32::from(self.cursor.1) + dy).ok(),
        );
        if let (Some(x), Some(y)) = next {
            self.point((x, y));
        }
    }

    /// Show `level`, when the map has it.
    pub fn show_level(&mut self, level: Level) {
        if self.levels().contains(&level) && self.level != level {
            self.level = level;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// The plan's pane changed size: a map just opened is fitted to it.
    pub fn laid_out(&mut self, pane: (u32, u32)) {
        if pane != self.pane && pane.0 > 0 && pane.1 > 0 {
            let fit = self.view == plan::View::fitted(self.size(), self.pane);
            self.pane = pane;
            if fit {
                self.view = plan::View::fitted(self.size(), pane);
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// What a click at pane pixel `(px, py)` acts on: with an edge brush the cell under it and
    /// its nearest side, wherever in the cell the pixel lies (never the facing: the mouse shows
    /// what it will do); otherwise the cell, or its edge when the pixel is within a quarter of
    /// the side of that border. None off the map.
    #[must_use]
    pub fn aim(&self, px: f32, py: f32) -> Option<Target> {
        let size = self.size();
        match self.tool {
            Tool::Edge(_) => self.view.edge_target(size, px, py),
            _ => self.view.target(size, px, py),
        }
    }

    /// Paint the brush at `target` on the shown level and put the cursor on its cell. With
    /// `stroke` the paint joins the stroke the held button keeps open, so a drag undoes as
    /// one. A refusal is said and changes nothing. Whether the map changed.
    pub fn paint(&mut self, data: &Data, target: Target, stroke: bool) -> bool {
        let Some(def) = self.def() else {
            self.message = Some("open a map first".to_owned());
            return false;
        };
        let Some(brush) = self.tool.brush(&def, target, self.facing, self.eighths) else {
            return false;
        };
        let joined = stroke && self.stroke.is_some();
        // The same target is not painted again while the button is held, even one refused.
        self.stroke = Some(target);
        self.brushed(data, target.cell, &brush, joined)
    }

    /// Paint `brush` on `cell` of the shown level and put the cursor there, joined to the open
    /// stroke when `joined`. A refusal is said and changes nothing. Whether the map changed.
    fn brushed(&mut self, data: &Data, cell: (u16, u16), brush: &Brush, joined: bool) -> bool {
        let (level, (x, y)) = (self.level, cell);
        let result = self
            .session
            .document_mut()
            .and_then(|d| d.paint(level, x, y, brush, joined));
        match result {
            Ok(()) => {
                self.message = None;
                self.cursor = cell;
                self.changed(data);
                true
            }
            Err(e) => {
                self.message = Some(e.to_string());
                false
            }
        }
    }

    /// Make `edit` to the map, one undo step. A refusal is said and changes nothing. Whether
    /// the map changed.
    fn edited(&mut self, data: &Data, edit: Edit) -> bool {
        let result = self.session.document_mut().and_then(|d| d.edit(edit));
        self.took(data, result)
    }

    /// Place, replace or remove with `place`, one undo step. A refusal is said and changes
    /// nothing. Whether the map changed.
    fn placed(&mut self, data: &Data, place: Place) -> bool {
        let result = self.session.document_mut().and_then(|d| d.place(place));
        self.took(data, result)
    }

    /// A change's outcome: taken, the map is read again; refused, the refusal is said.
    fn took(&mut self, data: &Data, result: Result<(), EditError>) -> bool {
        self.stroke = None;
        match result {
            Ok(()) => {
                self.message = None;
                self.changed(data);
                true
            }
            Err(e) => {
                self.message = Some(e.to_string());
                false
            }
        }
    }

    /// Field `i` of the open form was typed into: the form holds `text` now. The words on
    /// screen do not change (the field shows them), so nothing asks for a redraw.
    pub fn typed(&mut self, i: u8, text: String) {
        if let Some(form) = &mut self.form {
            form.typed(usize::from(i), text);
        }
    }

    /// Number field `i` of the open form was set outright (a slider).
    pub fn set_field(&mut self, i: u8, n: i64) {
        if let Some(form) = &mut self.form {
            form.set(usize::from(i), n);
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Open `form` at the top of the inspector, in place of any open one.
    fn open_form(&mut self, form: Form) {
        self.message = self
            .form
            .as_ref()
            .map(|_| "the open form was replaced".to_owned());
        self.form = Some(form);
    }

    /// The open form's Apply: its request made (a place or an edit one undo step, a note
    /// written at once) and the form closed; a field that does not read, or a refusal, is said
    /// and the form stands.
    fn apply_form(&mut self, data: &Data) {
        let Some((form, def)) = self.form.as_ref().zip(self.def()) else {
            return;
        };
        match form.request(&def) {
            Ok(change) => {
                if self.change(data, change) {
                    self.form = None;
                }
            }
            Err(why) => self.message = Some(why),
        }
    }

    /// Make `change`: whether it was taken.
    fn change(&mut self, data: &Data, change: Change) -> bool {
        match change {
            Change::Place(place) => self.placed(data, place),
            Change::Edit(edit) => self.edited(data, edit),
            Change::Note { key, text } => {
                let result = self.session.set_note(&key, text.as_deref());
                self.revision = self.revision.wrapping_add(1);
                match result {
                    Ok(()) => {
                        self.message = Some(if text.is_some() {
                            format!("Note on {key} written")
                        } else {
                            format!("Note on {key} cleared")
                        });
                        true
                    }
                    Err(e) => {
                        self.message = Some(format!("Note not written: {e}"));
                        false
                    }
                }
            }
        }
    }

    /// The button is up: the next paint is a new stroke.
    pub fn end_stroke(&mut self) {
        self.stroke = None;
    }

    /// Take what is under `target` as the brush: the edge's kind, the cell's terrain or no
    /// ground, and the cell's height; the cursor goes there. Never changes the map.
    pub fn pick(&mut self, target: Target) {
        let Some(grid) = self.grid() else {
            return;
        };
        let (x, y) = target.cell;
        let tool = match target.edge {
            Some(side) => Tool::Edge(grid.edge(x, y, side).unwrap_or_default()),
            None => match grid.tile(x, y).flatten() {
                Some(t) => Tool::Terrain(t),
                None => Tool::Clear,
            },
        };
        if let Some(height) = grid.height_at(x, y) {
            self.eighths = brush::held(height);
        }
        self.tool = tool;
        self.message = None;
        self.point(target.cell);
        self.revision = self.revision.wrapping_add(1);
    }

    /// Back or forward one step of the document's history.
    fn walk(&mut self, data: &Data, back: bool) {
        let result = self
            .session
            .document_mut()
            .and_then(|d| if back { d.undo() } else { d.redo() });
        self.stroke = None;
        match result {
            Ok(()) => {
                self.message = None;
                self.changed(data);
            }
            Err(e) => self.message = Some(e.to_string()),
        }
    }

    /// Write the map and reseal the pack, or say why not.
    fn save(&mut self, data: &Data) -> Asked {
        let file = self
            .session
            .document()
            .map(|d| d.file().to_owned())
            .unwrap_or_default();
        match self.session.save(data) {
            Ok(sealed) => {
                self.message = Some(format!(
                    "Saved {file}; {} sealed at {}",
                    self.session.pack(),
                    sealed.version
                ));
                self.stroke = None;
                self.changed(data);
                Asked::Saved
            }
            Err(EditError::Invalid(errors)) => {
                let first = errors.first().map(ToString::to_string).unwrap_or_default();
                self.message = Some(format!(
                    "Not saved: {} error{}; the first: {first}",
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" }
                ));
                Asked::Nothing
            }
            Err(e) => {
                self.message = Some(format!("Not saved: {e}"));
                Asked::Nothing
            }
        }
    }

    /// Reopen the map's file as it is on disk, keeping the cursor, the level and the view.
    fn revert(&mut self, data: &Data) {
        let Some(id) = self.map_id() else {
            return;
        };
        let (cursor, level, facing, view, tool, eighths) = (
            self.cursor,
            self.level,
            self.facing,
            self.view,
            self.tool,
            self.eighths,
        );
        match self.open(&id, data) {
            Ok(()) => {
                self.cursor = cursor;
                self.facing = facing;
                self.view = view;
                self.tool = tool;
                self.eighths = eighths;
                self.show_level(level);
                self.changed(data);
                self.message = Some(format!("Reverted {id} to its file"));
            }
            Err(e) => self.message = Some(format!("Not reverted: {e}")),
        }
    }

    /// Use a control of the layer: a press (`step` 0) or a choice's arrow (`step` −1 or 1).
    pub fn apply(&mut self, data: &Data, act: EditAct, step: i8) -> Asked {
        self.revision = self.revision.wrapping_add(1);
        match act {
            EditAct::Map => {
                let n = self.maps.len().max(1);
                let at = i64::try_from(self.chosen).unwrap_or(0) + i64::from(step);
                self.chosen =
                    usize::try_from(at.rem_euclid(i64::try_from(n).unwrap_or(1))).unwrap_or(0);
            }
            EditAct::Open => {
                let id = self.maps.get(self.chosen).map(|m| m.id.clone());
                match id {
                    Some(id) if !id.is_empty() => {
                        if let Err(e) = self.open(&id, data) {
                            self.message = Some(e.to_string());
                        }
                    }
                    _ => self.message = Some("that file declares no map".to_owned()),
                }
            }
            EditAct::Level => {
                let levels = self.levels();
                if let Some(i) = levels.iter().position(|l| *l == self.level) {
                    let n = i64::try_from(levels.len()).unwrap_or(1);
                    let at = (i64::try_from(i).unwrap_or(0) + i64::from(step)).rem_euclid(n);
                    if let Some(level) = usize::try_from(at).ok().and_then(|i| levels.get(i)) {
                        self.level = *level;
                    }
                }
            }
            EditAct::ZoomIn => self.view = self.view.zoomed(1, self.pane),
            EditAct::ZoomOut => self.view = self.view.zoomed(-1, self.pane),
            EditAct::Centre => self.view = self.view.centred_on(self.cursor, self.pane),
            EditAct::FaceLeft => self.facing = self.facing.left(),
            EditAct::FaceRight => self.facing = self.facing.right(),
            EditAct::Brush(i) => {
                if let Some(tool) = self
                    .def()
                    .and_then(|def| brush::palette(&def).get(usize::from(i)).copied())
                {
                    self.tool = tool;
                }
            }
            EditAct::Eighths => {
                // A press steps right, as a choice does.
                let by = if step == 0 { 1 } else { i16::from(step) };
                self.eighths = brush::held(self.eighths.saturating_add(by));
            }
            EditAct::CellTerrain => {
                let Some(def) = self.def() else {
                    return Asked::Nothing;
                };
                // The terrains in their order, then no ground, round.
                let n = def.terrains.len();
                let at = self.cell_terrain().map_or(n, usize::from);
                let at =
                    i64::try_from(at).unwrap_or(0) + i64::from(if step == 0 { 1 } else { step });
                let at = at.rem_euclid(i64::try_from(n + 1).unwrap_or(1));
                let brush = match usize::try_from(at).ok().and_then(|i| def.terrains.get(i)) {
                    Some(t) => Brush::Tile {
                        glyph: t.glyph,
                        rise: Some(self.facing),
                    },
                    None => Brush::Clear,
                };
                self.brushed(data, self.cursor, &brush, false);
            }
            EditAct::CellHeight => {
                let by = if step == 0 { 1 } else { i16::from(step) };
                let eighths = brush::held(self.cell_height().saturating_add(by));
                self.brushed(data, self.cursor, &Brush::Height { eighths }, false);
            }
            EditAct::CellRise => {
                if let Some(rise) = self.cell_rise() {
                    let rise = if step < 0 { rise.left() } else { rise.right() };
                    let (x, y) = self.cursor;
                    let level = self.level;
                    self.edited(data, Edit::Rise { level, x, y, rise });
                }
            }
            EditAct::SetStart => {
                let (x, y) = self.cursor;
                let facing = self.facing;
                self.placed(data, Place::Start { x, y, facing });
            }
            EditAct::PlaceKind => {
                let n = i64::try_from(Placing::ALL.len()).unwrap_or(1);
                let at = i64::try_from(self.place_kind).unwrap_or(0)
                    + i64::from(if step == 0 { 1 } else { step });
                self.place_kind = usize::try_from(at.rem_euclid(n)).unwrap_or(0);
            }
            EditAct::Add => {
                if let Some(def) = self.def()
                    && let Some(kind) = Placing::ALL.get(self.place_kind)
                {
                    self.open_form(Form::placing(*kind, &def, data, self.at()));
                }
            }
            EditAct::EditAt(i) => {
                let found = self
                    .here()
                    .get(usize::from(i))
                    .zip(self.def())
                    .and_then(|(here, def)| Form::existing(&here.key, &def, data, self.facing));
                if let Some(form) = found {
                    self.open_form(form);
                }
            }
            EditAct::RemoveAt(i) => {
                if let Some(here) = self.here().get(usize::from(i)) {
                    let key = here.key.clone();
                    self.placed(data, Place::Remove(key));
                }
            }
            EditAct::Random => {
                if let Some(def) = self.def() {
                    self.open_form(Form::random(&def, self.at()));
                }
            }
            EditAct::RestAdd => {
                if let Some(def) = self.def() {
                    self.open_form(Form::rest_event(&def, self.at()));
                }
            }
            EditAct::RestEdit(i) => {
                let found = self.def().and_then(|def| {
                    let r = def.rest_events.get(usize::from(i))?;
                    let key = PlaceKey::RestEvent {
                        terrain: r.terrain.clone(),
                        rest: r.rest,
                    };
                    Form::existing(&key, &def, data, self.facing)
                });
                if let Some(form) = found {
                    self.open_form(form);
                }
            }
            EditAct::RestRemove(i) => {
                let key = self.def().and_then(|def| {
                    def.rest_events
                        .get(usize::from(i))
                        .map(|r| PlaceKey::RestEvent {
                            terrain: r.terrain.clone(),
                            rest: r.rest,
                        })
                });
                if let Some(key) = key {
                    self.placed(data, Place::Remove(key));
                }
            }
            EditAct::Field(i) => {
                if let Some(form) = &mut self.form {
                    form.step(usize::from(i), step);
                }
            }
            EditAct::ListAdd(l) => {
                if let Some(form) = &mut self.form
                    && let Err(why) = form.add_row(l)
                {
                    self.message = Some(why);
                }
            }
            EditAct::ListRemove(l, r) => {
                if let Some(form) = &mut self.form {
                    form.remove_row(l, r);
                }
            }
            EditAct::Apply => self.apply_form(data),
            EditAct::Cancel => self.form = None,
            EditAct::Discard => {
                if let Some(change) = self.form.as_ref().and_then(|f| f.remove.clone())
                    && self.change(data, change)
                {
                    self.form = None;
                }
            }
            EditAct::TerrainAdd => {
                if let Some(def) = self.def() {
                    self.open_form(Form::terrain(&def, None, self.at()));
                }
            }
            EditAct::TerrainEdit => {
                if let (Some(def), Tool::Terrain(i)) = (self.def(), self.tool)
                    && let Some(glyph) = def.terrains.get(usize::from(i)).map(|t| t.glyph)
                {
                    self.open_form(Form::terrain(&def, Some(glyph), self.at()));
                }
            }
            EditAct::LevelAdd => {
                if let Some(def) = self.def() {
                    self.open_form(Form::level_add(&def, self.at()));
                }
            }
            EditAct::LevelEdit => {
                if let Some(form) = self
                    .def()
                    .and_then(|def| Form::level(&def, self.level.0, self.at()))
                {
                    self.open_form(form);
                }
            }
            EditAct::Note(of) => {
                if let Some(key) = self.note_key(of) {
                    let heading = match of {
                        NoteOf::Cell => format!("Note on ({}, {})", self.cursor.0, self.cursor.1),
                        NoteOf::Level => format!("Note on level {}", self.level.0),
                        NoteOf::Map => "Note on the map".to_owned(),
                    };
                    let text = self.note(of).map(str::to_owned);
                    self.open_form(Form::note(key, heading, text.as_deref(), self.at()));
                }
            }
            EditAct::Undo => self.walk(data, true),
            EditAct::Redo => self.walk(data, false),
            EditAct::Save => return self.save(data),
            EditAct::Revert => self.revert(data),
            EditAct::Menu => return Asked::Menu,
        }
        Asked::Nothing
    }
}
