//! The editor's columns as kit screens (M5.4): the tools on the left (with the brushes, Undo
//! and Redo, Save and Revert since M5.5), the attribute bar over the plan (M5.6) and the
//! inspector on the right, described like any screen (`crate::screen`) and drawn by the
//! screens' renderer under the editor's own mark. Bevy-free.

use super::brush::{self, Tool};
use super::forms::Placing;
use super::{EditAct, Editor, NoteOf};
use crate::screen::{Control, ControlId, Kind, Panel, Row, Screen, ScreenId, Span};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::EdgeKind;
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

/// The brush rows: the palette two to a row, the current one marked and the first nine
/// numbered for the digit keys, then the height's choice.
fn brush_rows(editor: &Editor, def: Option<&MapDef>, no_map: Option<&str>) -> Vec<Row> {
    let Some(def) = def else {
        return vec![Row::controls(vec![dim(
            Control::button(ControlId::Edit(EditAct::Brush(0)), "Brush"),
            no_map,
        )])];
    };
    let palette = brush::palette(def);
    let button = |(i, tool): (usize, &Tool)| {
        let name = tool.label(def);
        let digit = if i < 9 {
            format!("{}  ", i + 1)
        } else {
            String::new()
        };
        let mark = if *tool == editor.tool { "> " } else { "" };
        let id = ControlId::Edit(EditAct::Brush(u8::try_from(i).unwrap_or(u8::MAX)));
        Control::button(id, format!("{mark}{digit}{name}"))
    };
    let mut rows: Vec<Row> = palette
        .iter()
        .enumerate()
        .filter(|(_, t)| **t != Tool::Height)
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|pair| Row::controls(pair.iter().map(|p| button(*p)).collect()))
        .collect();
    if let Some(at) = palette.iter().position(|t| *t == Tool::Height) {
        let options: Vec<String> = brush::EIGHTHS.map(|h| h.to_string()).collect();
        let chosen =
            usize::try_from(i32::from(editor.eighths) - i32::from(*brush::EIGHTHS.start()))
                .unwrap_or(0);
        rows.push(Row::controls(vec![
            button((at, &Tool::Height)),
            choice(
                ControlId::Edit(EditAct::Eighths),
                "eighths",
                options,
                chosen,
            ),
        ]));
    }
    rows
}

/// The tools column: the map and level choices, Open, the zoom, the facing, the brushes, the
/// history, Save and Revert, and the menu.
#[must_use]
pub fn tools(editor: &Editor, data: &Data) -> Screen {
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
    let levels = editor.levels();
    let at = levels.iter().position(|l| *l == editor.level).unwrap_or(0);
    let level_names: Vec<String> = levels
        .iter()
        .map(|l| level_label(editor, data, *l))
        .collect();
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
    let mut rows = vec![
        Row::controls(vec![
            dim(
                choice(edit(EditAct::Map), "Map", maps, editor.chosen),
                editor.maps.is_empty().then_some("the pack has no maps"),
            ),
            dim(Control::button(edit(EditAct::Open), open_label), bad),
        ]),
        Row::controls(vec![
            dim(
                choice(edit(EditAct::Level), "Level", level_names, at),
                no_map.or((levels.len() < 2).then_some("one level")),
            ),
            dim(
                Control::button(edit(EditAct::LevelAdd), "Add level"),
                no_map,
            ),
            dim(
                Control::button(edit(EditAct::LevelEdit), "Level..."),
                no_map.or(editor
                    .level
                    .is_ground()
                    .then_some("the ground level is the map")),
            ),
        ]),
        Row::controls(vec![
            dim(Control::button(edit(EditAct::ZoomOut), "Zoom -  ["), no_map),
            dim(Control::button(edit(EditAct::ZoomIn), "Zoom +  ]"), no_map),
            dim(Control::button(edit(EditAct::Centre), "Centre  C"), no_map),
        ]),
        Row::controls(vec![
            dim(
                Control::button(edit(EditAct::FaceLeft), "Face left  Q"),
                no_map,
            ),
            dim(
                Control::button(edit(EditAct::FaceRight), "Face right  E"),
                no_map,
            ),
        ]),
        Row::text(Span::body(format!(
            "Zoom {} px a cell",
            editor.view.scale()
        ))),
    ];
    rows.extend(brush_rows(editor, def.as_ref(), no_map));
    rows.push(Row::controls(vec![
        dim(
            Control::button(edit(EditAct::TerrainAdd), "Add terrain"),
            no_map,
        ),
        dim(
            Control::button(edit(EditAct::TerrainEdit), "Edit terrain"),
            no_map
                .or((!matches!(editor.tool, Tool::Terrain(_))).then_some("choose a terrain brush")),
        ),
    ]));
    rows.push(Row::controls(vec![
        dim(
            Control::button(edit(EditAct::Undo), "Undo  Cmd Z"),
            no_map.or((status.undo == 0).then_some("nothing to undo")),
        ),
        dim(
            Control::button(edit(EditAct::Redo), "Redo  Cmd Y"),
            no_map.or((status.redo == 0).then_some("nothing to redo")),
        ),
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
    rows.push(Row::controls(vec![dim(
        Control::button(edit(EditAct::Playtest), "Playtest"),
        blocked.as_deref(),
    )]));
    rows.push(Row::controls(vec![Control::button(
        edit(EditAct::Menu),
        "Menu  Esc",
    )]));
    Screen {
        id: ScreenId::EditorTools,
        title: Span::title(title),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: editor.message.clone().map(Span::alert),
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
            rows: vec![Row::controls(controls)],
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
    rows.push(Row::text(Span::body(format!(
        "Level {}",
        level_label(editor, data, level)
    ))));
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
