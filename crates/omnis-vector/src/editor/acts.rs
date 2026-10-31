//! The acts the editor's controls ask for (M5.4 to M5.9c) and `Editor::apply`, which carries
//! each out: a press or a choice's arrow on the tools, the bar, the inspector, a form or the
//! file picker. Bevy-free, as the rest of the model (`mod.rs`, which held both until M5.10).

use super::brush::{self, Entry, Tool};
use super::files::Mode;
use super::form::Form;
use super::{Asked, Editor, NoteOf};
use omnis_sim::omnis_core::Level;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::{Brush, EdgeKind, Edit, Place, Target as PlaceKey};

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
    /// Choose a tool of the tool grid (the owner's column, 2026-10-25).
    Tool(Tool),
    /// Choose an entry of the palette, by its position (M5.5; the digits).
    Brush(u8),
    /// Take the pending wall run: its sides become walls, one undo step (`run`).
    TakeRun,
    /// Drop the pending run's last corner.
    DropCorner,
    /// Drop the pending run, changing nothing.
    DropRun,
    /// The height choice: steps the height brush's eighths.
    Eighths,
    /// The chosen stamp's icon under the palette (the owner's column, step 3b): opens the
    /// picker modal for its detail, as Select... does (two controls, two acts: the layer syncs
    /// a control's words by its id).
    Stamp,
    /// Select... under the palette: open the picker modal for the chosen stamp's detail.
    Pick,
    /// The picker's search field, as typed.
    PickFilter,
    /// A row of the picker, by its position among the kind's options: chosen or let go.
    PickToggle(u16),
    /// The picker's Finish: the stamp holds the choice; a kind with properties goes on to them.
    PickFinish,
    /// Open the properties modal for the chosen stamp: the sliders icon under the palette.
    Props,
    /// Field `.1` of properties form `.0`.
    Prop(u8, u8),
    /// The properties modal's Finish: the forms read into the stamp.
    PropsFinish,
    /// The bar's terrain choice (M5.6): steps the cursor's cell through the terrains and no
    /// ground.
    CellTerrain,
    /// The bar's height choice: steps the cursor's cell's height by an eighth.
    CellHeight,
    /// The bar's rise choice: turns the way the stair on the cursor's cell climbs.
    CellRise,
    /// Put the map's start on the cursor's cell, facing as the cursor does (M5.6).
    SetStart,
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
    /// Start a game on the edited map at the cursor (M5.8).
    Playtest,
    /// Show the map's plan (M5.9).
    Plan,
    /// Show the open file's text (M5.9).
    Files,
    /// List the pack's files and open the picker over the editor (M5.9b); an open file with
    /// unsaved text is asked about first, a clean one is closed (M5.9c).
    OpenFile,
    /// The picker's Save and continue: check and write the open file, then close it.
    FileSaveClose,
    /// The picker's Discard changes: close the open file as it stands.
    FileDiscard,
    /// Open the nth listed file as text, or show the open one again.
    File(u16),
    /// The open file's text, as typed.
    FileText,
    /// Ask the loader about the open file's text.
    Check,
    /// Write the open file's text exactly as typed and reseal the pack.
    WriteFile,
    /// Put the file's text back as the file holds it.
    RevertFile,
}

impl Editor {
    /// Use a control of the layer: a press (`step` 0) or a choice's arrow (`step` −1 or 1). A
    /// pending wall run is dropped when the act changed the tool, the level, the map or its
    /// history: a run belongs to the place and the tool it was drawn with.
    pub fn apply(&mut self, data: &Data, act: EditAct, step: i8) -> Asked {
        // Any act ends a drag: what it held is let go where it is.
        self.grab = None;
        let before = self.run_holds();
        let asked = self.act(data, act, step);
        if self.run.is_some() && self.run_holds() != before {
            self.run = None;
        }
        asked
    }

    /// What a pending run holds to: the tool and its entry, the level, the map and its
    /// history's depths.
    fn run_holds(&self) -> (Tool, Entry, Level, Option<String>, usize, usize) {
        let depths = self
            .session
            .document()
            .map_or((0, 0), |d| (d.undo_depth(), d.redo_depth()));
        (
            self.tool,
            self.entry,
            self.level,
            self.map_id(),
            depths.0,
            depths.1,
        )
    }

    fn act(&mut self, data: &Data, act: EditAct, step: i8) -> Asked {
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
            EditAct::Tool(tool) => {
                self.tool = tool;
                // A run takes an edge: choosing a run tool with a terrain or the height chosen
                // chooses the wall.
                if tool.is_run() && !self.entry.is_edge() {
                    self.entry = Entry::Edge(EdgeKind::Wall);
                }
                // A fill takes a terrain or the height: choosing it with an edge or a stamp
                // chosen chooses the first terrain (the height on a map without one).
                if tool == Tool::Fill && !self.entry.fills() {
                    self.entry = match self.def().map(|def| Entry::first(&def)) {
                        Some(first) if !first.is_edge() => first,
                        _ => Entry::Height,
                    };
                }
            }
            EditAct::Brush(i) => {
                if let Some(entry) = self
                    .def()
                    .and_then(|def| brush::palette(&def).get(usize::from(i)).copied())
                {
                    self.entry = entry;
                    // Choosing what to paint is choosing to paint, unless the tool can take
                    // the entry: a run an edge, the fill a terrain or the height.
                    let keeps = (self.tool.is_run() && entry.is_edge())
                        || (self.tool == Tool::Fill && entry.fills());
                    if !keeps {
                        self.tool = Tool::Brush;
                    }
                }
            }
            EditAct::TakeRun => self.take_run(data),
            EditAct::DropCorner => self.drop_corner(),
            EditAct::DropRun => self.drop_run(),
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
            EditAct::Stamp | EditAct::Pick => return self.pick_open(),
            // The search's text comes through `typed_in`.
            EditAct::PickFilter => {}
            EditAct::PickToggle(i) => self.pick_toggle(data, i),
            EditAct::PickFinish => return self.pick_finish(data),
            EditAct::Props => return self.props_open(data),
            EditAct::Prop(n, i) => self.prop(n, i, step),
            EditAct::PropsFinish => return self.props_finish(),
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
                if let (Some(def), Entry::Terrain(i)) = (self.def(), self.entry)
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
            EditAct::Playtest => return Asked::Playtest,
            EditAct::Plan => self.mode = Mode::Plan,
            EditAct::Files => {
                if self.file.is_some() {
                    self.mode = Mode::Files;
                }
            }
            EditAct::OpenFile => match self.session.files() {
                Ok(files) => {
                    self.files = files;
                    // A clean file is closed here (the owner, 2026-10-24: "finish with clearing
                    // out the existing editor load"); an unsaved one is asked about in the picker.
                    if self.file.as_ref().is_some_and(|f| !f.dirty()) {
                        self.file = None;
                    }
                    return Asked::Pick;
                }
                Err(e) => self.message = Some(format!("Cannot list the files: {e}")),
            },
            EditAct::FileSaveClose => {
                // Check, then write: the write refuses on its own while the verdict has errors.
                self.check_file(data);
                let asked = self.write_file(data);
                if asked == Asked::Saved {
                    self.file = None;
                }
                return asked;
            }
            EditAct::FileDiscard => {
                if let Some(file) = self.file.take() {
                    self.message = Some(format!("{} left as the file holds it", file.rel));
                }
            }
            EditAct::File(n) => {
                if self.open_file(usize::from(n)) {
                    return Asked::Picked;
                }
            }
            EditAct::FileText => {}
            EditAct::Check => {
                self.check_file(data);
            }
            EditAct::WriteFile => return self.write_file(data),
            EditAct::RevertFile => {
                if let Some(file) = &mut self.file {
                    file.revert();
                    self.message = Some(format!("{} as the file holds it", file.rel));
                }
            }
        }
        Asked::Nothing
    }
}
