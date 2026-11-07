//! The editor's model (M5 Editor v1 from M5.4; ARCHITECTURE §8.2, `tasks/plans/m5-editor-v1.md`
//! §Client side): the document session over the pack being edited, the open map's level,
//! cursor and facing, the plan view, the brush (M5.5), the cursor's cell as the attribute bar
//! edits it (M5.6) and the acts the layer's controls ask for. Bevy-free: the shell
//! (`shell/editor.rs`) draws it and routes the pointer and the keys to it.

pub mod acts;
pub mod brush;
pub mod files;
pub mod fill;
pub mod form;
pub mod forms;
pub mod grab;
pub mod links;
pub mod pick;
pub mod plan;
pub mod playtest;
pub mod preview;
pub mod run;
pub mod select;
pub mod stamp;
pub mod view;

pub use acts::EditAct;
use brush::{Entry, Target, Tool};
use files::{FileEdit, Mode};
use form::{At, Change, Form};
use grab::Grab;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::{
    Brush, EdgeKind, Edit, EditError, EditorStatus, Grid, MapEntry, Place, Session,
    Target as PlaceKey,
};
use omnis_sim::omnis_data::{Data, MapData, MapDef};
use pick::Picking;
use run::{Corner, Run};
use select::Selection;
use stamp::Stamps;
use std::path::PathBuf;

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

/// What the layer does after an act: nothing more, open the menu or the file picker, close the
/// picker, reload the packs, or start a playtest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// Open the history list over the editor (step 5).
    History,
    /// Nothing beyond the model's change.
    Nothing,
    /// Open the editor's menu.
    Menu,
    /// The map was written: the packs are read from disk again, so a new game walks it.
    Saved,
    /// Start a playtest: the shell builds the game and parks the editor (M5.8).
    Playtest,
    /// Open the file picker over the editor (M5.9b).
    Pick,
    /// A file was picked, or a stamp's detail: the modal on top closes (step 3b).
    Picked,
    /// Open the stamp's detail picker over the editor (step 3b).
    PickDetail,
    /// Open the stamp's properties over the editor, in the picker's place when it is open.
    Props,
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
    /// The revision at which `resolved` last changed: the preview's scene is rebuilt on it, not
    /// on every move of the cursor (M5.7).
    pub resolved_at: u64,
    /// What the pointer does: the tool grid's state (the owner's column, 2026-10-25).
    pub tool: Tool,
    /// What the brush paints: the palette's entry.
    pub entry: Entry,
    /// The height brush's height, in eighths.
    pub eighths: i16,
    /// The last target painted in the stroke the held button keeps open; none between strokes.
    pub stroke: Option<Target>,
    /// What a click would act on now: the aim under the pointer (`aim`), shown over the plan;
    /// none while the pointer is off the plan or over a control, and with a run tool.
    pub hover: Option<Target>,
    /// With a run tool, the grid corner nearest the pointer (`run`); none otherwise.
    pub corner: Option<Corner>,
    /// The wall run being drawn, if one is: pending by corners, or under a freehand drag.
    pub run: Option<Run>,
    /// The drag under the Move tool or the middle button, while a button holds it (`grab`).
    pub grab: Option<Grab>,
    /// The lasso's loop while the button draws it: the cells passed, in order (`select`).
    pub lasso: Option<Vec<(u16, u16)>>,
    /// The cells selected by the lasso, on their level, until cleared or moved (`select`).
    pub selection: Option<Selection>,
    /// The history list (step 5): every step's words, oldest first, and how many of them the
    /// map stands after; read from the document whenever it changes.
    history: Vec<String>,
    history_at: usize,
    /// The session's state against the packs, as of the last change (the loader's verdict is
    /// costly, so it is read here, never asked for each frame).
    pub status: EditorStatus,
    /// The form being filled at the top of the inspector, if one is (M5.6).
    pub form: Option<Form>,
    /// What each stamp of the palette drops (the owner's column, step 3; `stamp`).
    pub stamps: Stamps,
    /// A stamp's detail being chosen in the picker modal, and its properties (step 3b; `pick`).
    pub picking: Option<Picking>,
    /// What the centre shows: the plan or a file's text (M5.9).
    pub mode: Mode,
    /// The pack's files, listed when Files mode was entered.
    pub files: Vec<String>,
    /// The file open as text, if one is.
    pub file: Option<FileEdit>,
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
            resolved_at: 0,
            tool: Tool::Brush,
            entry: Entry::Edge(EdgeKind::Wall),
            eighths: 0,
            stroke: None,
            hover: None,
            corner: None,
            run: None,
            grab: None,
            lasso: None,
            selection: None,
            history: Vec::new(),
            history_at: 0,
            status,
            form: None,
            stamps: Stamps::default(),
            picking: None,
            mode: Mode::Plan,
            files: Vec::new(),
            file: None,
        })
    }

    /// Open the map `id`, putting the cursor at its start, fitting the plan to the pane and
    /// taking its first terrain as the brush. The plan shows, whatever the centre showed before;
    /// an open text file stays open behind it.
    ///
    /// # Errors
    /// The session's: no such map, or its layout does not read.
    pub fn open(&mut self, id: &str, data: &Data) -> Result<(), EditError> {
        self.session.open(id)?;
        if let Some(i) = self.maps.iter().position(|m| m.id == id) {
            self.chosen = i;
        }
        let def = self.def();
        // The map opens on its start's level, the cursor on the start (W1 step 3).
        let (start, level, size) = def
            .as_ref()
            .map_or(((0, 0, Facing::North), Level::GROUND, (0, 0)), |d| {
                (d.start, d.start_level, (d.width, d.height))
            });
        self.level = level;
        self.cursor = (start.0, start.1);
        self.facing = start.2;
        self.view = plan::View::fitted(size, self.pane);
        self.tool = Tool::Brush;
        self.entry = def
            .as_ref()
            .map_or(Entry::Edge(EdgeKind::Wall), Entry::first);
        self.stroke = None;
        self.grab = None;
        self.lasso = None;
        self.selection = None;
        self.message = None;
        self.resolved = None;
        self.form = None;
        self.mode = Mode::Plan;
        self.changed(data);
        Ok(())
    }

    /// The document changed (by the layer or over the socket): read its status, resolve it
    /// again, keeping the last valid resolve and saying why when the loader refuses it, and
    /// ask for a redraw.
    pub fn changed(&mut self, data: &Data) {
        self.revision = self.revision.wrapping_add(1);
        self.status = self.session.status(data);
        self.read_history();
        let Some(def) = self.def() else {
            self.resolved = None;
            return;
        };
        match MapData::resolve(&def, data) {
            Ok(map) => {
                self.resolved = Some(map);
                self.resolved_at = self.revision;
            }
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

    /// The history list (step 5): the words of every step, oldest first, and how many of them
    /// the map stands after; None with no map.
    #[must_use]
    pub fn history(&self) -> Option<(&[String], usize)> {
        self.session
            .document()
            .is_some()
            .then_some((self.history.as_slice(), self.history_at))
    }

    /// Read the history list from the document.
    fn read_history(&mut self) {
        (self.history, self.history_at) = self.session.document().map_or_else(
            || (Vec::new(), 0),
            |d| (d.steps().iter().map(|s| (*s).to_owned()).collect(), d.at()),
        );
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
        self.here_at(self.cursor)
    }

    /// The placements on `cell` of the shown level, as the inspector would list them.
    #[must_use]
    pub fn here_at(&self, cell: (u16, u16)) -> Vec<Here> {
        let Some(def) = self.def() else {
            return Vec::new();
        };
        let (level, (x, y)) = (self.level, cell);
        let is_here = |l: Level, px: u16, py: u16| l == level && px == x && py == y;
        let mut out = Vec::new();
        for p in def.portals.iter().filter(|p| is_here(p.level, p.x, p.y)) {
            out.push(Here {
                label: match &p.link {
                    Some(link) => format!("Portal '{}' to '{link}'", p.id),
                    None => format!("Portal '{}'", p.id),
                },
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
                label: format!("Service {}", s.service),
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
            // A selection is its level's: another level shown, it is gone.
            self.selection = None;
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

    /// What a click at pane pixel `(px, py)` acts on: with the brush and an edge entry the cell
    /// under it and its nearest side, wherever in the cell the pixel lies (never the facing: the
    /// mouse shows what it will do); with the brush otherwise and the eraser the cell, or its
    /// edge when the pixel is within a quarter of the side of that border; with select, move,
    /// fill and a stamp the cell. None off the map, and with a run tool, which aims at a corner
    /// (`run::Editor::pointer`).
    #[must_use]
    pub fn aim(&self, px: f32, py: f32) -> Option<Target> {
        let size = self.size();
        match (self.tool, self.entry) {
            (Tool::Select | Tool::Move | Tool::Lasso | Tool::Fill, _)
            | (Tool::Brush, Entry::Stamp(_)) => self
                .view
                .hit(size, px, py)
                .map(|cell| Target { cell, edge: None }),
            (Tool::Brush, Entry::Edge(_)) => self.view.edge_target(size, px, py),
            (Tool::Brush | Tool::Eraser, _) => self.view.target(size, px, py),
            (Tool::Corners | Tool::Freehand, _) => None,
        }
    }

    /// Zoom one step in (`by` 1) or out (-1) about pane pixel `(px, py)`, the cell under it kept
    /// there (M5.9d: the wheel over the plan). Nothing without a map.
    pub fn zoom_at(&mut self, by: i8, px: f32, py: f32) {
        if self.def().is_none() {
            return;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let at = (px.max(0.0) as u32, py.max(0.0) as u32);
        let view = self.view.zoomed_at(by, at);
        if view != self.view {
            self.view = view;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Use the tool at `target` on the shown level and put the cursor on its cell: the brush
    /// paints the palette's entry, or places its stamp (`stamp`, one step of its own, once a
    /// click); the fill paints it over the cell's region (`fill`, one step of its own, once a
    /// click); the eraser opens the edge, removes the first placement on the cell (one step of
    /// its own) or clears its ground; select only moves the cursor. With `stroke` a paint joins
    /// the stroke the held button keeps open, so a drag undoes as one. A refusal is said and
    /// changes nothing. Whether the map changed.
    pub fn paint(&mut self, data: &Data, target: Target, stroke: bool) -> bool {
        let Some(def) = self.def() else {
            self.message = Some("open a map first".to_owned());
            return false;
        };
        let brush = match self.tool {
            Tool::Select => {
                self.point(target.cell);
                return false;
            }
            Tool::Fill => return self.fill(data, target.cell),
            Tool::Brush if self.entry.stamp().is_some() => return self.stamp(data, target.cell),
            Tool::Brush => self.entry.brush(&def, target, self.facing, self.eighths),
            Tool::Eraser => match target.edge {
                Some(side) => Some(Brush::Edge {
                    side,
                    kind: EdgeKind::Open,
                }),
                None => match self.here_at(target.cell).into_iter().next() {
                    Some(here) => {
                        self.point(target.cell);
                        return self.placed(data, Place::Remove(here.key));
                    }
                    None => Some(Brush::Clear),
                },
            },
            // Move's and the lasso's drags never come this way (`grab::Editor::grabbing`).
            Tool::Move | Tool::Lasso | Tool::Corners | Tool::Freehand => None,
        };
        let Some(brush) = brush else {
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

    /// The open file's text was typed into (M5.9): the model holds it, the verdict no longer
    /// holds, and the tools column's Save file follows.
    pub fn typed_file(&mut self, text: String) {
        if let Some(file) = &mut self.file
            && file.typed(text)
        {
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Open the nth listed file as text, fresh (its field takes the keyboard), and show it. A
    /// file is refused while the open one has unsaved text (the picker asks about it first, so
    /// this guards the socket and the tests). Whether a file is shown.
    fn open_file(&mut self, n: usize) -> bool {
        let Some(rel) = self.files.get(n).cloned() else {
            return false;
        };
        if let Some(file) = &self.file
            && file.dirty()
        {
            self.message = Some(format!("save or revert {} first", file.rel));
            return false;
        }
        match self.session.read(&rel) {
            Ok(text) => {
                self.file = Some(FileEdit::read(rel, text));
                self.message = None;
                self.mode = Mode::Files;
                true
            }
            Err(e) => {
                self.message = Some(format!("Cannot read {rel}: {e}"));
                false
            }
        }
    }

    /// Ask the loader about the open file's text as typed (M5.9): the verdict in the message and
    /// on the file. Whether the loader takes it; none with no file open or when the check itself
    /// could not run.
    fn check_file(&mut self, data: &Data) -> Option<bool> {
        let file = self.file.as_ref()?;
        match self.session.check(&file.rel, &file.text, data) {
            Ok(errors) => {
                let words: Vec<String> = errors.iter().map(ToString::to_string).collect();
                self.message = Some(match words.first() {
                    None => format!("The loader takes {}", file.rel),
                    Some(first) => format!("{} error(s); the first: {first}", words.len()),
                });
                let clean = words.is_empty();
                if let Some(file) = &mut self.file {
                    file.checked(words);
                }
                Some(clean)
            }
            Err(e) => {
                self.message = Some(format!("Not checked: {e}"));
                None
            }
        }
    }

    /// Write the open file exactly as typed, reseal the pack, and reopen the document when the
    /// file is the open map's own, so the plan shows what was written.
    fn write_file(&mut self, data: &Data) -> Asked {
        let Some(file) = self.file.as_ref() else {
            return Asked::Nothing;
        };
        if let Some(why) = file.blocked_save() {
            self.message = Some(format!("Not written: {why}"));
            return Asked::Nothing;
        }
        let (rel, text) = (file.rel.clone(), file.text.clone());
        match self.session.write(&rel, &text, data) {
            Ok(sealed) => {
                if let Some(file) = &mut self.file {
                    file.written();
                }
                self.message = Some(match sealed {
                    Some(s) => format!(
                        "Wrote {rel}; {} sealed at {}",
                        self.session.pack(),
                        s.version
                    ),
                    None => format!("Wrote {rel} (a note: not sealed)"),
                });
                let own = self.session.document().is_some_and(|d| d.file() == rel);
                if own {
                    self.reopen(data);
                }
                Asked::Saved
            }
            Err(EditError::Invalid(errors)) => {
                if let Some(file) = &mut self.file {
                    file.checked(errors.iter().map(ToString::to_string).collect());
                }
                self.message = Some("Not written: the loader refuses it".to_owned());
                Asked::Nothing
            }
            Err(e) => {
                self.message = Some(format!("Not written: {e}"));
                Asked::Nothing
            }
        }
    }

    /// The open map read again from its file, keeping the cursor, the level, the view, the
    /// facing, the brush and what the centre shows.
    fn reopen(&mut self, data: &Data) {
        let Some(id) = self.map_id() else {
            return;
        };
        let (cursor, level, facing, view, tool, entry, eighths, mode) = (
            self.cursor,
            self.level,
            self.facing,
            self.view,
            self.tool,
            self.entry,
            self.eighths,
            self.mode,
        );
        if let Err(e) = self.open(&id, data) {
            self.message = Some(format!("The map did not reopen: {e}"));
            return;
        }
        self.cursor = cursor;
        self.facing = facing;
        self.view = view;
        self.tool = tool;
        self.entry = entry;
        self.eighths = eighths;
        self.mode = mode;
        self.show_level(level);
        self.changed(data);
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
    /// and the form stands. A placement edited under a new key (an id typed anew) takes its old
    /// entry out in the same step: the document puts by key and would keep both.
    fn apply_form(&mut self, data: &Data) {
        let Some((form, def)) = self.form.as_ref().zip(self.def()) else {
            return;
        };
        let request = form.request(&def);
        let old = match &form.remove {
            Some(Change::Place(Place::Remove(key))) => Some(key.clone()),
            _ => None,
        };
        match request {
            Ok(change) => {
                let what = match &change {
                    Change::Place(_) => "Placed",
                    Change::Edit(_) => "Changed",
                    Change::Note { .. } => "Written",
                };
                let taken = match (change, old) {
                    (Change::Place(new), Some(old)) if new.key().is_some_and(|k| k != old) => {
                        let places = vec![Place::Remove(old), new];
                        let result = self
                            .session
                            .document_mut()
                            .and_then(|d| d.place_many(places));
                        self.took(data, result)
                    }
                    (change, _) => self.change(data, change),
                };
                if taken {
                    self.form = None;
                    self.report(what.to_owned());
                }
            }
            Err(why) => self.message = Some(why),
        }
    }

    /// Say `what` was made, with the loader's first word on the map as it now stands (owner,
    /// 2026-10-24: the verdict at once, so what was just made saves; Save is dim with the same
    /// words meanwhile).
    fn report(&mut self, what: String) {
        self.name_step(&what);
        self.message = Some(match self.status.errors.first() {
            Some(first) => format!("{what}; the loader says: {first}"),
            None => what,
        });
    }

    /// Name the newest step of the history with the editor's own words for what it did (step
    /// 5): the history list reads as the messages did.
    pub(super) fn name_step(&mut self, words: &str) {
        if let Ok(doc) = self.session.document_mut() {
            doc.name_step(words);
            self.read_history();
        }
    }

    /// Jump to where the map stood after step `at` of the history (step 5); past the end is
    /// said and nothing moves.
    fn history_to(&mut self, data: &Data, at: usize) {
        let result = self.session.document_mut().and_then(|d| d.goto(at));
        self.stroke = None;
        match result {
            Ok(()) => {
                let n = self.session.document().map_or(0, |d| d.steps().len());
                self.message = Some(if at == 0 {
                    "the map as opened".to_owned()
                } else {
                    format!("at step {at} of {n}")
                });
                self.changed(data);
            }
            Err(e) => self.message = Some(e.to_string()),
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

    /// Take what is under `target` as the brush and its entry: the edge's kind, or the first
    /// placement the inspector lists on the cell as its stamp holding that placement whole
    /// (`stamp::of_here`), or the cell's terrain, and the cell's height; an open edge or no
    /// ground is the eraser. The cursor goes there. Never changes the map. A pending run is
    /// dropped with its tool.
    pub fn pick(&mut self, target: Target) {
        self.run = None;
        let (x, y) = target.cell;
        let stamped = match (target.edge, self.def()) {
            (None, Some(def)) => self
                .here_at((x, y))
                .iter()
                .find_map(|here| stamp::of_here(&def, here)),
            _ => None,
        };
        let Some(grid) = self.grid() else {
            return;
        };
        let (tool, entry) = match target.edge {
            Some(side) => match grid.edge(x, y, side).unwrap_or_default() {
                EdgeKind::Open => (Tool::Eraser, self.entry),
                kind => (Tool::Brush, Entry::Edge(kind)),
            },
            None => match (&stamped, grid.tile(x, y).flatten()) {
                (Some((kind, _)), _) => (Tool::Brush, Entry::Stamp(*kind)),
                (None, Some(t)) => (Tool::Brush, Entry::Terrain(t)),
                (None, None) => (Tool::Eraser, self.entry),
            },
        };
        let height = grid.height_at(x, y);
        if let Some(height) = height {
            self.eighths = brush::held(height);
        }
        if let Some((kind, places)) = stamped {
            self.stamps.set(kind, places);
        }
        self.tool = tool;
        self.entry = entry;
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
        self.reopen(data);
        if self.message.is_none() {
            self.message = Some(format!("Reverted {id} to its file"));
        }
    }
}
