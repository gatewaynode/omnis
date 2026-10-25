//! The editor's columns as kit screens (M5.4): the tools on the left (with the brushes, Undo
//! and Redo, Save and Revert since M5.5), the attribute bar over the plan (M5.6) and the
//! inspector on the right, described like any screen (`crate::screen`) and drawn by the
//! screens' renderer under the editor's own mark. Bevy-free.

use super::brush::{self, Entry, Tool};
use super::files::Mode;
use super::forms::Placing;
use super::{EditAct, Editor, NoteOf};
use crate::screen::{Control, ControlId, Kind, Panel, Row, Screen, ScreenId, Span};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::EdgeKind;
use omnis_sim::omnis_data::limits::MAX_FILE_BYTES;
use omnis_sim::omnis_data::{Data, MapDef};

/// The most of the loader's refusals the inspector lists.
const ERRORS_SHOWN: usize = 6;

fn choice(id: ControlId, label: &str, options: Vec<String>, at: usize) -> Control {
    Control {
        kind: Kind::Choice { options, at },
        ..Control::button(id, label)
    }
}

fn dim(mut control: Control, why: Option<&str>) -> Control {
    control.why_not = why.map(str::to_owned);
    control
}

/// A level's name in the choice: its number, and its name when the map has several.
fn level_label(editor: &Editor, data: &Data, level: Level) -> String {
    let name = editor
        .resolved
        .as_ref()
        .and_then(|map| crate::labels::level(data, map, level));
    name.unwrap_or_else(|| level.0.to_string())
}

/// The tool grid (the owner's drawing-GUI column, 2026-10-25): an icon a tool, the current one
/// chosen, then Undo and Redo with their reasons; under it the current tool's name and key.
fn tool_rows(editor: &Editor, no_map: Option<&str>) -> Vec<Row> {
    let status = &editor.status;
    let edit = |act| ControlId::Edit(act);
    let icon = |act, name, chosen| Control {
        kind: Kind::Icon { name, chosen },
        ..Control::button(edit(act), name)
    };
    let mut grid: Vec<Control> = Tool::ALL
        .iter()
        .map(|t| dim(icon(EditAct::Tool(*t), t.icon(), *t == editor.tool), no_map))
        .collect();
    grid.push(dim(
        icon(EditAct::Undo, "undo-2", false),
        no_map.or((status.undo == 0).then_some("nothing to undo")),
    ));
    grid.push(dim(
        icon(EditAct::Redo, "redo-2", false),
        no_map.or((status.redo == 0).then_some("nothing to redo")),
    ));
    vec![
        Row::controls(grid),
        Row::text(Span::body(format!(
            "{}  {}",
            editor.tool.label(),
            editor.tool.key()
        ))),
    ]
}

/// The palette grid: a swatch a terrain (its colour and glyph), then the wall, the door, the
/// ramp and the height as icons, the current one chosen; under it the entry's details (its
/// words, with its digit while the keys reach it; the height's eighths choice; Edit terrain for
/// a terrain; Add terrain), then the run's rows while one is pending.
fn palette_rows(editor: &Editor, def: &MapDef) -> Vec<Row> {
    let edit = |act| ControlId::Edit(act);
    let palette = brush::palette(def);
    let grid: Vec<Control> = palette
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let chosen = *entry == editor.entry;
            let id = edit(EditAct::Brush(u8::try_from(i).unwrap_or(u8::MAX)));
            let kind = match entry {
                Entry::Terrain(t) => {
                    let terrain = def.terrains.get(usize::from(*t));
                    Kind::Swatch {
                        rgb: terrain.map_or((128, 128, 128), |t| t.color),
                        glyph: terrain.map_or('?', |t| t.glyph),
                        chosen,
                    }
                }
                Entry::Edge(_) | Entry::Height => Kind::Icon {
                    name: entry.icon().unwrap_or("square-dashed"),
                    chosen,
                },
            };
            Control {
                kind,
                ..Control::button(id, entry.label(def))
            }
        })
        .collect();
    let digit = palette
        .iter()
        .position(|e| *e == editor.entry)
        .filter(|i| *i < 9)
        .map_or_else(String::new, |i| format!("{}  ", i + 1));
    let mut rows = vec![
        Row::controls(grid),
        Row::text(Span::body(format!(
            "{digit}{}",
            editor.entry.describes(def)
        ))),
    ];
    let mut details = Vec::new();
    if editor.entry == Entry::Height {
        let options: Vec<String> = brush::EIGHTHS.map(|h| h.to_string()).collect();
        let at = usize::try_from(i32::from(editor.eighths) - i32::from(*brush::EIGHTHS.start()))
            .unwrap_or(0);
        details.push(choice(edit(EditAct::Eighths), "eighths", options, at));
    }
    if matches!(editor.entry, Entry::Terrain(_)) {
        details.push(Control::button(edit(EditAct::TerrainEdit), "Edit terrain"));
    }
    details.push(Control::button(edit(EditAct::TerrainAdd), "Add terrain"));
    rows.push(Row::controls(details));
    rows.extend(run_rows(editor));
    rows
}

/// The run's rows while a wall run is pending (the boundary tools): how many corners it has,
/// then Take run, Drop run and Drop corner, with their keys. Nothing while no run is drawn.
fn run_rows(editor: &Editor) -> Vec<Row> {
    let Some(run) = &editor.run else {
        return Vec::new();
    };
    let edit = |act| ControlId::Edit(act);
    let n = run.corners.len();
    let corners = if n == 1 { "corner" } else { "corners" };
    vec![
        Row::text(Span::body(format!("Run: {n} {corners}"))),
        Row::controls(vec![
            Control::button(edit(EditAct::TakeRun), "Take run  Enter"),
            Control::button(edit(EditAct::DropRun), "Drop run"),
        ]),
        Row::controls(vec![Control::button(
            edit(EditAct::DropCorner),
            "Drop corner  Backspace",
        )]),
    ]
}

/// The tools column (the owner's drawing-GUI column, 2026-10-25): the tool grid and the palette
/// grid with the entry's details (or the open file's rows in Files mode), then the files (the
/// map choice and Open; Plan, Text and Open file...) and under all Save and Revert, Playtest and
/// the menu. The view's controls are the bar's, the level's edits the inspector's.
#[must_use]
pub fn tools(editor: &Editor, data: &Data) -> Screen {
    let _ = data;
    let status = &editor.status;
    let def = editor.def();
    let open = def.is_some();
    let no_map = (!open).then_some("open a map first");
    let maps: Vec<String> = editor
        .maps
        .iter()
        .map(|m| match &m.error {
            Some(e) => format!("{} ({e})", m.file),
            None => m.id.clone(),
        })
        .collect();
    let bad = editor
        .maps
        .get(editor.chosen)
        .and_then(|m| m.error.as_deref())
        .map(|_| "that file does not read");
    let title = format!(
        "EDITOR  {}{}",
        status.root,
        if status.dirty { "  *" } else { "" }
    );
    let edit = |act| ControlId::Edit(act);
    let errors = &status.errors;
    let invalid = errors.first().map(|first| {
        format!(
            "{} error{}: {first}",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" }
        )
    });
    let open_label = if status.dirty {
        "Open (drops changes)"
    } else {
        "Open"
    };
    let marked = |mode: Mode, label: &str| {
        if editor.mode == mode {
            format!("> {label}")
        } else {
            label.to_owned()
        }
    };
    let mut rows = tool_rows(editor, no_map);
    if editor.mode == Mode::Files {
        rows.extend(file_rows(editor));
    } else if let Some(def) = &def {
        rows.extend(palette_rows(editor, def));
    } else {
        rows.push(Row::text(Span::body("No map is open.")));
    }
    rows.push(Row::controls(vec![
        dim(
            choice(edit(EditAct::Map), "Map", maps, editor.chosen),
            editor.maps.is_empty().then_some("the pack has no maps"),
        ),
        dim(Control::button(edit(EditAct::Open), open_label), bad),
    ]));
    rows.push(Row::controls(vec![
        Control::button(edit(EditAct::Plan), marked(Mode::Plan, "Plan")),
        dim(
            Control::button(edit(EditAct::Files), marked(Mode::Files, "Text")),
            editor.file.is_none().then_some("open a file first"),
        ),
        Control::button(edit(EditAct::OpenFile), "Open file..."),
    ]));
    rows.push(Row::controls(vec![
        dim(
            Control::button(edit(EditAct::Save), "Save  Cmd S"),
            no_map.or(invalid.as_deref()),
        ),
        dim(
            Control::button(edit(EditAct::Revert), "Revert"),
            no_map.or((!status.dirty).then_some("no changes")),
        ),
    ]));
    let blocked = super::playtest::blocked(editor);
    rows.push(Row::controls(vec![
        dim(
            Control::button(edit(EditAct::Playtest), "Playtest"),
            blocked.as_deref(),
        ),
        Control::button(edit(EditAct::Menu), "Menu  Esc"),
    ]));
    Screen {
        id: ScreenId::EditorTools,
        title: Span::title(title),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: editor.message.clone().map(Span::alert),
    }
}

/// The file's rows of the tools column (M5.9): the open file's name (`*` while its text is
/// unsaved), then Check, Save file and Revert file with their reasons. The list itself is the
/// picker's (M5.9b).
fn file_rows(editor: &Editor) -> Vec<Row> {
    let edit = |act| ControlId::Edit(act);
    let open = editor.file.as_ref().map(|f| f.rel.as_str());
    let mut rows = vec![Row::text(Span::body(match &editor.file {
        Some(f) if f.dirty() => format!("{}  *", f.rel),
        Some(f) => f.rel.clone(),
        None => "No file open.".to_owned(),
    }))];
    let none = open.is_none().then_some("choose a file");
    let file = editor.file.as_ref();
    let blocked = file.and_then(super::files::FileEdit::blocked_save);
    rows.push(Row::controls(vec![
        dim(Control::button(edit(EditAct::Check), "Check"), none),
        dim(
            Control::button(edit(EditAct::WriteFile), "Save file"),
            none.or(blocked.as_deref()),
        ),
        dim(
            Control::button(edit(EditAct::RevertFile), "Revert file"),
            none.or(file.is_some_and(|f| !f.dirty()).then_some("no changes")),
        ),
    ]));
    rows
}

/// The centre's text screen (M5.9): the open file as one document (numbered lines, both
/// scrollbars, M5.9c), or a word when none is open.
#[must_use]
pub fn text(editor: &Editor) -> Screen {
    let (title, rows) = match &editor.file {
        Some(file) => (
            file.rel.clone(),
            vec![Row::controls(vec![Control {
                kind: Kind::Document {
                    value: file.text.clone(),
                    max: usize::try_from(MAX_FILE_BYTES).unwrap_or(usize::MAX),
                },
                ..Control::button(ControlId::Edit(EditAct::FileText), "")
            }])],
        ),
        None => (
            "FILES".to_owned(),
            vec![Row::text(Span::body("Open a file from the tools column."))],
        ),
    };
    Screen {
        id: ScreenId::EditorText,
        title: Span::title(title),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// The attribute bar over the plan (M5.6): the cursor's cell and the attributes every cell has,
/// always in reach wherever the tools column is scrolled: its terrain (or no ground), its
/// height in eighths and, when it is a stair, the way it climbs. A step paints the cell, one
/// undo step each.
#[must_use]
pub fn bar(editor: &Editor, data: &Data) -> Screen {
    let def = editor.def();
    let no_map = def.is_none().then_some("open a map first");
    let (x, y) = editor.cursor;
    let title = match &def {
        Some(_) => format!(
            "Cell ({x}, {y})  level {}",
            level_label(editor, data, editor.level)
        ),
        None => "No map is open.".to_owned(),
    };
    let mut terrains: Vec<String> = def
        .as_ref()
        .map(|d| {
            d.terrains
                .iter()
                .map(|t| format!("{} {}", t.glyph, t.name))
                .collect()
        })
        .unwrap_or_default();
    let at = editor.cell_terrain().map_or(terrains.len(), usize::from);
    terrains.push("no ground".to_owned());
    let heights: Vec<String> = brush::EIGHTHS.map(|h| h.to_string()).collect();
    let height_at = usize::try_from(
        i32::from(brush::held(editor.cell_height())) - i32::from(*brush::EIGHTHS.start()),
    )
    .unwrap_or(0);
    let edit = |act| ControlId::Edit(act);
    let view = |act, name| {
        dim(
            Control {
                kind: Kind::Icon {
                    name,
                    chosen: false,
                },
                ..Control::button(edit(act), name)
            },
            no_map,
        )
    };
    // The view's controls (the owner's drawing-GUI column, 2026-10-25: the view is the bar's,
    // the column holds the tools): zoom out and in, centre, turn left and right, as icons, and
    // the level choice; the keys are their shortcuts.
    let levels = editor.levels();
    let level_names: Vec<String> = levels
        .iter()
        .map(|l| level_label(editor, data, *l))
        .collect();
    let at_level = levels.iter().position(|l| *l == editor.level).unwrap_or(0);
    let views = Row {
        controls: vec![
            view(EditAct::ZoomOut, "zoom-out"),
            view(EditAct::ZoomIn, "zoom-in"),
            view(EditAct::Centre, "focus"),
            view(EditAct::FaceLeft, "rotate-ccw"),
            view(EditAct::FaceRight, "rotate-cw"),
            dim(
                choice(edit(EditAct::Level), "Level", level_names, at_level),
                no_map.or((levels.len() < 2).then_some("one level")),
            ),
        ],
        after: vec![Span::body(format!(
            "{} px a cell   keys [ ] C Q E",
            editor.view.scale()
        ))],
        ..Row::default()
    };
    let mut controls = vec![
        dim(
            choice(edit(EditAct::CellTerrain), "Terrain", terrains, at),
            no_map,
        ),
        dim(
            choice(edit(EditAct::CellHeight), "Height", heights, height_at),
            no_map,
        ),
    ];
    if let Some(rise) = editor.cell_rise() {
        let options = Facing::ALL
            .iter()
            .map(|f| facing_name(*f).to_owned())
            .collect();
        let at = Facing::ALL.iter().position(|f| *f == rise).unwrap_or(0);
        controls.push(choice(edit(EditAct::CellRise), "Rise", options, at));
    }
    Screen {
        id: ScreenId::EditorBar,
        title: Span::title(title),
        panels: vec![Panel {
            rows: vec![views, Row::controls(controls)],
        }],
        keys: Vec::new(),
        message: None,
    }
}

/// A row for the author's note on `of`: `label: <its first line, or none>` and the button to
/// write it.
fn note_row(editor: &Editor, of: NoteOf, label: &str, button: &str) -> Row {
    let first = editor
        .note(of)
        .map_or("none", |n| n.lines().next().unwrap_or(""));
    Row {
        spans: vec![Span::body(format!("{label}: {first}"))],
        controls: vec![Control::button(ControlId::Edit(EditAct::Note(of)), button)],
        ..Row::default()
    }
}

const fn facing_name(facing: Facing) -> &'static str {
    match facing {
        Facing::North => "north",
        Facing::East => "east",
        Facing::South => "south",
        Facing::West => "west",
    }
}

const fn edge_name(kind: Option<EdgeKind>) -> &'static str {
    match kind {
        Some(EdgeKind::Wall) => "wall",
        Some(EdgeKind::Door) => "door",
        Some(EdgeKind::Ramp) => "ramp",
        Some(EdgeKind::Open) => "open",
        None => "-",
    }
}

/// The inspector column: the open form first (M5.6), then the map, the level, the cursor's cell
/// with its terrain, height and edges, the start, the cell's placements with Edit and Remove
/// and Add for more, the map's random table and rest events, and the loader's refusals.
#[must_use]
pub fn inspector(editor: &Editor, data: &Data) -> Screen {
    let mut rows = Vec::new();
    if editor.mode == Mode::Files {
        // The file's verdict (M5.9): the loader's words on the text as typed.
        match &editor.file {
            None => rows.push(Row::text(Span::body("No file is open."))),
            Some(file) => {
                rows.push(Row::text(Span::body(format!(
                    "{}  {} bytes{}",
                    file.rel,
                    file.text.len(),
                    if file.dirty() { "  *" } else { "" }
                ))));
                let verdict = file.verdict();
                match &file.errors {
                    Some(errors) if !errors.is_empty() => {
                        rows.push(Row::text(Span::alert(verdict)));
                        for e in errors.iter().take(ERRORS_SHOWN) {
                            rows.push(Row::text(Span::alert(e.clone())));
                        }
                    }
                    _ => rows.push(Row::text(Span::body(verdict))),
                }
            }
        }
        return Screen {
            id: ScreenId::EditorInspector,
            title: Span::title("INSPECTOR"),
            panels: vec![Panel { rows }],
            keys: Vec::new(),
            message: None,
        };
    }
    if let Some(form) = &editor.form {
        rows.extend(form.rows());
    }
    let Some(def) = editor.def() else {
        rows.push(Row::text(Span::body("No map is open.")));
        return Screen {
            id: ScreenId::EditorInspector,
            title: Span::title("INSPECTOR"),
            panels: vec![Panel { rows }],
            keys: Vec::new(),
            message: None,
        };
    };
    let (x, y) = editor.cursor;
    let level = editor.level;
    rows.push(Row::text(Span::body(format!(
        "{}  {} x {}",
        def.id, def.width, def.height
    ))));
    rows.push(Row {
        spans: vec![Span::body(format!(
            "Level {}",
            level_label(editor, data, level)
        ))],
        controls: vec![
            Control::button(ControlId::Edit(EditAct::LevelAdd), "Add level"),
            dim(
                Control::button(ControlId::Edit(EditAct::LevelEdit), "Level..."),
                level.is_ground().then_some("the ground level is the map"),
            ),
        ],
        ..Row::default()
    });
    rows.push(Row::text(Span::body(format!(
        "Cell ({x}, {y}) facing {}",
        facing_name(editor.facing)
    ))));
    if let Some(grid) = editor.grid() {
        match grid.tile(x, y) {
            Some(Some(t)) => {
                let terrain = def.terrains.get(usize::from(t));
                rows.push(Row::text(Span::body(format!(
                    "Terrain {} {}",
                    terrain.map_or('?', |t| t.glyph),
                    terrain.map_or("?", |t| t.name.as_str())
                ))));
                rows.push(Row::text(Span::body(format!(
                    "Height {} eighths",
                    grid.height_at(x, y).unwrap_or(0)
                ))));
            }
            _ => rows.push(Row::text(Span::body("No ground"))),
        }
        let edges: Vec<String> = Facing::ALL
            .iter()
            .map(|f| {
                let cliff = editor
                    .resolved
                    .as_ref()
                    .and_then(|m| m.cell(level, x, y))
                    .is_some_and(|c| c.cliffs.has(*f));
                format!(
                    "{} {}{}",
                    facing_name(*f)
                        .chars()
                        .next()
                        .unwrap_or('?')
                        .to_ascii_uppercase(),
                    edge_name(grid.edge(x, y, *f)),
                    if cliff { " cliff" } else { "" }
                )
            })
            .collect();
        rows.push(Row::text(Span::body(edges.join("  "))));
    }
    let edit = |act| ControlId::Edit(act);
    // The author's note on the cell: its first line, and the form to write it.
    rows.push(note_row(editor, NoteOf::Cell, "Note", "Edit note"));
    // The start, and the button that moves it here.
    let start_here = level.is_ground() && def.start.0 == x && def.start.1 == y;
    rows.push(Row {
        spans: vec![Span::body(format!(
            "Start ({}, {}) facing {}",
            def.start.0,
            def.start.1,
            facing_name(def.start.2)
        ))],
        controls: vec![dim(
            Control::button(edit(EditAct::SetStart), "Set start here"),
            (!level.is_ground())
                .then_some("the start is on the ground level")
                .or((start_here && def.start.2 == editor.facing).then_some("the start is here")),
        )],
        ..Row::default()
    });
    // What is placed on the cell, each with Edit and Remove, and Add for more.
    for (i, here) in editor.here().iter().enumerate() {
        let i = u8::try_from(i).unwrap_or(u8::MAX);
        rows.push(Row {
            spans: vec![Span::body(here.label.clone())],
            controls: vec![
                Control::button(edit(EditAct::EditAt(i)), "Edit"),
                Control::button(edit(EditAct::RemoveAt(i)), "Remove"),
            ],
            ..Row::default()
        });
    }
    rows.push(Row::controls(vec![
        choice(
            edit(EditAct::PlaceKind),
            "Add",
            Placing::ALL.iter().map(|k| k.label().to_owned()).collect(),
            editor.place_kind,
        ),
        Control::button(edit(EditAct::Add), "Add here"),
    ]));
    // The map's own: the random table and the rest events.
    rows.push(Row {
        spans: vec![Span::body(def.random.as_ref().map_or_else(
            || "No random encounters".to_owned(),
            |r| {
                format!(
                    "Random encounters: {} entr{} at {}%",
                    r.entries.len(),
                    if r.entries.len() == 1 { "y" } else { "ies" },
                    r.chance_percent
                )
            },
        ))],
        controls: vec![Control::button(edit(EditAct::Random), "Edit")],
        ..Row::default()
    });
    rows.push(note_row(editor, NoteOf::Map, "Map note", "Edit"));
    rows.push(note_row(editor, NoteOf::Level, "Level note", "Edit"));
    rows.push(Row {
        spans: vec![Span::body("Rest events")],
        controls: vec![Control::button(edit(EditAct::RestAdd), "Add")],
        ..Row::default()
    });
    for (i, r) in def.rest_events.iter().enumerate() {
        let i = u8::try_from(i).unwrap_or(u8::MAX);
        rows.push(Row {
            spans: vec![Span::body(format!(
                "{} on {:?} rest, {} per mille",
                r.terrain, r.rest, r.chance
            ))],
            controls: vec![
                Control::button(edit(EditAct::RestEdit(i)), "Edit"),
                Control::button(edit(EditAct::RestRemove(i)), "Remove"),
            ],
            ..Row::default()
        });
    }
    let errors = &editor.status.errors;
    if errors.is_empty() {
        rows.push(Row::text(Span::body("The loader takes this map.")));
    } else {
        rows.push(Row::text(Span::alert(format!(
            "{} error{}",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" }
        ))));
        for e in errors.iter().take(ERRORS_SHOWN) {
            rows.push(Row::text(Span::alert(e.clone())));
        }
    }
    Screen {
        id: ScreenId::EditorInspector,
        title: Span::title("INSPECTOR"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}
