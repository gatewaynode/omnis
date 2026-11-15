//! The words that explain the editor (owner, 2026-11-13): the groups a form's fields stand
//! under, and what each field is for, shown as its tip. Bevy-free.

use super::brush::{Entry, Tool};
use super::form::FormKind;
use super::forms::Placing;
use super::{EditAct, NoteOf};
use crate::screen::{ControlId, Screen};
use omnis_sim::omnis_data::edit::EdgeKind;

/// The heading of the group that starts at the field named `label` of a `kind` form, when one
/// starts there.
#[must_use]
pub fn section(kind: &FormKind, label: &str) -> Option<&'static str> {
    Some(match (kind, label) {
        (FormKind::Terrain(_) | FormKind::Encounter | FormKind::Object | FormKind::Lore, "id")
        | (FormKind::Terrain(_), "glyph") => "IDENTITY",
        (FormKind::Terrain(_), "floor") => "TILES",
        (FormKind::Terrain(_), "passable") => "MOVEMENT",
        (FormKind::Terrain(_), "role") => "LOOK",
        (FormKind::Terrain(_), "red") => "MAP COLOUR",
        (FormKind::Encounter, "disposition") => "BEHAVIOUR",
        (FormKind::Encounter, "stands apart") => "STAND",
        (FormKind::Object, "pos x") => "PLACE",
        (FormKind::Object, "state") => "STATE",
        (FormKind::Object | FormKind::Lore, "lore" | "text") => "LORE",
        _ => return None,
    })
}

/// What the field named `label` of a `kind` form is for: what it means, its range, and what
/// leaving it empty means. `list` names the list when the field is in one of its rows.
#[must_use]
pub fn field(kind: &FormKind, list: Option<&str>, label: &str) -> Option<&'static str> {
    Some(match (kind, list, label) {
        (FormKind::Terrain(_), None, "glyph") => {
            "The one character that stands for this terrain in the map's layout. Not a space, \
             and not one another terrain of the map uses."
        }
        (FormKind::Terrain(_), None, "name") => {
            "The terrain's name in the editor and in rest events. One word."
        }
        (FormKind::Terrain(_), None, "floor") => "The tileset surface drawn as its ground. Needed.",
        (FormKind::Terrain(_), None, "ceiling") => {
            "The tileset surface overhead. Empty is open sky."
        }
        (FormKind::Terrain(_), None, "block") => {
            "The tileset surface of a solid block that fills the cell: rock, trees, a pillar. \
             Empty is no block."
        }
        (FormKind::Terrain(_), None, "passable") => "Whether the party may step onto it.",
        (FormKind::Terrain(_), None, "opaque") => {
            "Whether it blocks the line of sight: rock and dense trees do, water does not."
        }
        (FormKind::Terrain(_), None, "visibility depth") => {
            "How many cells the party sees while standing on it."
        }
        (FormKind::Terrain(_), None, "step minutes") => {
            "The minutes one step onto it costs. 0 is free."
        }
        (FormKind::Terrain(_), None, "climb") => {
            "The largest height difference a step onto it may cross, in eighths of a cell (a \
             wall is 8). A larger difference is a cliff."
        }
        (FormKind::Terrain(_), None, "join") => {
            "How a cell of it joins its level to the next: stairs up or down, a ladder, a hole. \
             None for plain ground."
        }
        (FormKind::Terrain(_), None, "role") => {
            "What its lines mean in the 3D view, and so their colour from the biosphere's \
             palette. Default is ground underfoot and structure where it is a block."
        }
        (FormKind::Terrain(_), None, "face") => {
            "How its block is drawn in the 3D view: plain is its box alone, hatched adds a \
             cliff's upright lines, which tells trees or rock from a wall."
        }
        (FormKind::Terrain(_), None, "red" | "green" | "blue") => {
            "Its colour on the automap and on this plan, 0 to 255. The 3D view takes its \
             colour from the role."
        }
        // A portal.
        (FormKind::Portal, None, "id") => {
            "The portal's name, which another portal's link names. Unlike any other portal's \
             on any loaded map."
        }
        (FormKind::Portal, None, "link") => {
            "The id of the portal this one leads to; choose it in the list under the field. \
             Empty is a portal that leads nowhere yet."
        }
        (FormKind::Portal, None, "facing") => {
            "The way the party faces coming out of it: it arrives on the cell ahead of the \
             portal, facing this way."
        }
        (FormKind::Portal, None, "marker") => {
            "The tileset object drawn on the portal's cell: stairs, a signpost, a gate."
        }
        // An encounter.
        (FormKind::Encounter, None, "id") => {
            "The encounter's name on this map. One word; encounters and objects share their \
             names, so no object may have it."
        }
        (FormKind::Encounter | FormKind::Object | FormKind::Lore, None, "x") => {
            "Its cell's column, counted from 0 at the west side. Changing it moves it."
        }
        (FormKind::Encounter | FormKind::Object | FormKind::Lore, None, "y") => {
            "Its cell's row, counted from 0 at the north side. Changing it moves it."
        }
        (FormKind::Encounter, None, "disposition") => {
            "How the creatures meet the party: hostile attack on sight, wary fight when \
             pressed, neutral can be bought or slipped past, friendly let it pass."
        }
        (FormKind::Encounter, None, "once") => {
            "Ticked, once the party clears it, it is gone for the rest of the game."
        }
        (FormKind::Encounter, None, "stands apart") => {
            "Ticked, the creatures stand in view at the place the three fields under this \
             give; unticked, they are unseen until the party steps onto the cell."
        }
        (FormKind::Encounter, None, "stand x") => {
            "With stands apart: how far east of the cell's middle they stand, in eighths of a \
             cell, -4 to 4."
        }
        (FormKind::Encounter, None, "stand y") => {
            "With stands apart: how far south of the cell's middle they stand, in eighths of a \
             cell, -4 to 4."
        }
        (FormKind::Encounter, None, "facing") => "With stands apart: the way they face.",
        (FormKind::Encounter, Some(_), "creature") => {
            "The id of a monster of the loaded packs, like base:monster:goblin."
        }
        (FormKind::Encounter, Some(_), "count") => "How many of that creature, 1 to 255.",
        // The random encounter table.
        (FormKind::Random, None, "chance percent") => {
            "The chance in a hundred that a step onto a cell of this map meets one of the \
             entries. 0 is never."
        }
        (FormKind::Random, Some(_), "weight") => {
            "How often this entry is the one met, against the other entries' weights."
        }
        (FormKind::Random, Some(_), "disposition") => {
            "How these creatures meet the party: hostile attack on sight, wary fight when \
             pressed, neutral can be bought or slipped past, friendly let it pass."
        }
        (FormKind::Random, Some(_), "creatures") => {
            "The creatures and how many, each an id and its dice, separated by commas: \
             base:monster:goblin 1d4, base:monster:wolf 1."
        }
        // An activity.
        (FormKind::Site, None, "kind") => {
            "The kind of activity. A service's site is the one kind so far."
        }
        (FormKind::Site, None, "service") => {
            "The service the party finds on this cell: an inn, a temple, a smith."
        }
        // A rest event.
        (FormKind::RestEvent, None, "terrain") => {
            "The terrain the party must be resting on for this to happen."
        }
        (FormKind::RestEvent, None, "rest") => {
            "The kind of rest it may happen in: a short one, a long one, or any."
        }
        (FormKind::RestEvent, None, "chance per mille") => {
            "The chance in a thousand that it happens in one such rest. 0 is never."
        }
        (FormKind::RestEvent, None, "text") => {
            "The text key of what the party is told when it happens."
        }
        // An object.
        (FormKind::Object, None, "id") => {
            "The object's name on this map. One word; objects and encounters share their \
             names, so no encounter may have it."
        }
        (FormKind::Object, None, "kind") => {
            "What it is: an object kind of the loaded packs, which gives its shape, its \
             states and what can be done with it."
        }
        (FormKind::Object | FormKind::Ground, None, "pos x") => {
            "How far east of the cell's middle it lies, in eighths of a cell, -4 to 4."
        }
        (FormKind::Object | FormKind::Ground, None, "pos y") => {
            "How far south of the cell's middle it lies, in eighths of a cell, -4 to 4."
        }
        (FormKind::Object, None, "facing") => "The way its front faces.",
        (FormKind::Object, None, "state") => {
            "The state it starts in, one of its kind's. Empty is its kind's own first state."
        }
        (FormKind::Object, None, "hidden") => {
            "The difficulty of finding it, 1 to 30. 0 is in plain sight."
        }
        (FormKind::Object, None, "name") => {
            "The text key of a name of its own. Empty is its kind's name."
        }
        (FormKind::Object, None, "lore") => {
            "The text key of what a search of it may tell. Empty is nothing to learn."
        }
        (FormKind::Object, None, "lore DC") => {
            "The Investigation roll its lore needs, 1 to 30. At 0 every search tells it."
        }
        (FormKind::Object, None, "lore once") => {
            "Ticked, its lore is told on the first search that meets it and never again."
        }
        (FormKind::Object | FormKind::Ground, Some(_), "item") => {
            "The id of an item of the loaded packs, like base:item:torch."
        }
        (FormKind::Object, Some(_), "count") => "How many of that item it holds, 1 to 65535.",
        (FormKind::Ground, Some(_), "count") => "How many of that item lie there, 1 to 65535.",
        // A lore.
        (FormKind::Lore, None, "id") => {
            "The lore's name on this map. One word, unlike any other lore's here."
        }
        (FormKind::Lore, None, "text") => {
            "The text key of what a search of this cell may tell, one of the loaded packs' keys."
        }
        (FormKind::Lore, None, "DC") => {
            "The Investigation roll it needs, 1 to 30. At 0 every search tells it."
        }
        (FormKind::Lore, None, "once") => {
            "Ticked, it is told on the first search that meets it and never again."
        }
        // A level.
        (FormKind::LevelAdd, None, "level") => {
            "The new level's number: above the ground is 1 and up, under it -1 and down. The \
             ground is 0."
        }
        (FormKind::LevelAdd | FormKind::Level(_), None, "name") => {
            "The text key of the level's name. Needed."
        }
        (FormKind::LevelAdd | FormKind::Level(_), None, "base") => {
            "The height added to every cell of the level, in eighths of a cell (a wall is 8). \
             Negative for a level under the ground."
        }
        // A note.
        (FormKind::Note(_), None, "note") => {
            "Your own note on this place. Kept in the pack's notes, never shown in the game; \
             written at once, outside undo."
        }
        _ => return None,
    })
}

/// What a tool of the grid does.
#[must_use]
pub fn tool(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => {
            "Select (V): a click puts the cursor on a cell and paints nothing. The inspector \
             shows what is there."
        }
        Tool::Brush => {
            "Brush (B): a click paints the palette's entry on the cell or the side under the \
             pointer; a drag is one stroke and one undo step."
        }
        Tool::Move => {
            "Move (M): drag the selection, or the thing under the pointer, to another cell. A \
             drag on nothing pans the plan."
        }
        Tool::Lasso => {
            "Lasso (L): drag a loop; the cells inside it are selected. A click clears the \
             selection."
        }
        Tool::Fill => {
            "Fill (F): a click fills the region of one ground under the pointer with the \
             palette's entry, stopping at every closed side."
        }
        Tool::Corners => {
            "Run by corners (P): click corner after corner, then the last one again to take the \
             run: walls, doors or ramps along it, or ground and height inside it."
        }
        Tool::Freehand => "Freehand run (N): drag a run; it is taken when the button is let go.",
        Tool::Eraser => {
            "Eraser (X): a click opens the side, removes the first thing placed on the cell, or \
             clears its ground."
        }
    }
}

/// What a palette entry paints.
#[must_use]
pub fn entry(entry: Entry) -> &'static str {
    match entry {
        Entry::Terrain(_) => {
            "A terrain of this map: the brush paints it as a cell's ground. Edit terrain under \
             the palette changes what it is."
        }
        Entry::Edge(EdgeKind::Wall) => "Wall: closes the side of a cell under the pointer.",
        Entry::Edge(EdgeKind::Door) => "Door: a door in the side of a cell under the pointer.",
        Entry::Edge(EdgeKind::Ramp) => {
            "Ramp: a walkable slope across the side of a cell, between two heights that would be \
             a cliff."
        }
        Entry::Edge(EdgeKind::Open) => "Open edge: takes a wall, door or ramp off a side.",
        Entry::Height => {
            "Height: sets a cell's height to the eighths chosen under the palette (a wall is 8 \
             high)."
        }
        Entry::Stamp(Placing::Portal) => {
            "Portal: a way to another place. Select... chooses the portal it links to."
        }
        Entry::Stamp(Placing::Encounter) => {
            "Encounter: creatures on a cell. Select... chooses the monster."
        }
        Entry::Stamp(Placing::Activity) => {
            "Activity: a town service's site. Select... chooses the service."
        }
        Entry::Stamp(Placing::Object) => {
            "Object: a thing standing on a cell. Select... chooses its kind."
        }
        Entry::Stamp(Placing::Ground) => {
            "Ground pile: items lying on a cell. Select... chooses the item."
        }
        Entry::Stamp(Placing::Lore) => {
            "Lore: something a search of the cell may tell. Select... chooses its text."
        }
    }
}

/// What a control of the editor does, by its act: the buttons, choices and icons of the tools
/// column, the bar and the inspector. A tool, a palette entry and a form's field have their
/// own (`tool`, `entry`, `field`); a row of a list (a file, a rule, a step of the history) has
/// none.
#[must_use]
pub fn act(act: EditAct) -> Option<&'static str> {
    Some(match act {
        EditAct::Tool(t) => tool(t),
        EditAct::Map => "The pack's maps. Step to one, then press Open.",
        EditAct::Open => "Open the chosen map. Unsaved changes to the map now open are dropped.",
        EditAct::Level => "The level shown on the plan and edited. 0 is the ground.",
        EditAct::ZoomIn => "Zoom the plan in (]). The wheel over the plan zooms too.",
        EditAct::ZoomOut => "Zoom the plan out ([).",
        EditAct::Centre => "Put the cursor at the plan's middle (C).",
        EditAct::FaceLeft => {
            "Turn the cursor's facing a quarter left (Q). A placement takes the cursor's facing, \
             and an edge is painted on the side it faces."
        }
        EditAct::FaceRight => "Turn the cursor's facing a quarter right (E).",
        EditAct::TakeRun => "Take the run: what it draws is made, in one undo step.",
        EditAct::DropCorner => "Take back the run's last corner.",
        EditAct::DropRun => "Give the run up; nothing changes.",
        EditAct::Eighths => {
            "The height the brush sets, in eighths of a cell. A wall is 8 high; a step of more \
             than a terrain's climb is a cliff."
        }
        EditAct::Stamp | EditAct::Pick => {
            "Choose what the stamp places: opens the list of the loaded packs' own."
        }
        EditAct::Props => {
            "Set what each thing the stamp places starts with: its place on the cell, its state, \
             its counts."
        }
        EditAct::Unlink => "Clear the link: the portal leads nowhere until it is linked.",
        EditAct::CellTerrain => {
            "The terrain of the cursor's cell. Steps through the map's terrains and no ground."
        }
        EditAct::CellHeight => {
            "The height of the cursor's cell, in eighths of a cell. Steps by one."
        }
        EditAct::CellRise => "The way the stair on the cursor's cell climbs.",
        EditAct::SetStart => "Put the map's start on the cursor's cell, facing as the cursor does.",
        EditAct::EditAt(_) => "Open this thing's form.",
        EditAct::RemoveAt(_) => "Remove this thing from the cell. One undo step.",
        EditAct::Random => {
            "The map's random encounters: the chance at each step and what may be met."
        }
        EditAct::RestAdd => "Add something that may happen while the party rests on a terrain.",
        EditAct::RestEdit(_) => "Open this rest event's form.",
        EditAct::RestRemove(_) => "Remove this rest event. One undo step.",
        EditAct::ListAdd(_) => "Add a row to the list.",
        EditAct::ListRemove(..) => "Take this row out of the list.",
        EditAct::Apply => "Make the form's change. One undo step.",
        EditAct::Cancel => "Close the form; nothing changes.",
        EditAct::Discard => "Remove what the form edits, and close it.",
        EditAct::TerrainAdd => "Add a terrain to this map's list.",
        EditAct::TerrainEdit => {
            "Change the chosen terrain: its tiles, how it is walked, its look and colour."
        }
        EditAct::LevelAdd => "Add a level over or under the ground.",
        EditAct::LevelEdit => "Change the shown level's name and base height, or remove it.",
        EditAct::Note(NoteOf::Cell) => {
            "Your own note on the cursor's cell. Never shown in the game."
        }
        EditAct::Note(NoteOf::Level) => {
            "Your own note on the shown level. Never shown in the game."
        }
        EditAct::Note(NoteOf::Map) => "Your own note on the map. Never shown in the game.",
        EditAct::Undo => "Undo the last step (Cmd Z).",
        EditAct::Redo => "Do the undone step again (Shift Cmd Z).",
        EditAct::History => "The list of every step made; a click goes back to it.",
        EditAct::Rules => {
            "Every rule the loaded packs set: its value, the pack it came from and what it \
             replaced."
        }
        EditAct::Resize => {
            "Add cells on any side of the map, or cut them off. Everything on the map moves \
             with its cell."
        }
        EditAct::ResizeSide(_) => {
            "Cells to add on this side. A negative number cuts that many off; empty is none."
        }
        EditAct::ResizeApply => {
            "Resize the map. A cut that would destroy anything asks first; Undo brings it back."
        }
        EditAct::ResizeYes => "Make the cut and lose what is listed. Undo brings it back.",
        EditAct::ResizeNo => "Leave the map as it is and return to the numbers.",
        EditAct::Save => "Write the map to its file and reseal the pack (Cmd S).",
        EditAct::Revert => "Open the map's file again, dropping every unsaved change.",
        EditAct::Menu => "The editor's menu: back to the title, or quit (Esc).",
        EditAct::Playtest => {
            "Start a game on this map at the cursor, as the editor holds it, saved or not. The \
             map is not changed and the game is never saved."
        }
        EditAct::Plan => "Show the map's plan.",
        EditAct::Files => "Show the open file's text.",
        EditAct::OpenFile => "Open one of the pack's files as text.",
        EditAct::Check => "Ask the loader whether the text is a file it takes; nothing is written.",
        EditAct::WriteFile => "Write the text to the file exactly as typed and reseal the pack.",
        EditAct::RevertFile => "Put the text back as the file holds it.",
        EditAct::Brush(_)
        | EditAct::PickFilter
        | EditAct::PickToggle(_)
        | EditAct::PickFinish
        | EditAct::Link(_)
        | EditAct::Prop(..)
        | EditAct::PropsFinish
        | EditAct::Field(_)
        | EditAct::HistoryTo(_)
        | EditAct::RulesFilter
        | EditAct::RuleFile(_)
        | EditAct::FileSaveClose
        | EditAct::FileDiscard
        | EditAct::File(_)
        | EditAct::FileText => return None,
    })
}

/// Give every control of an editor's column that has no tip yet the one its act has; a palette
/// entry takes its own, by its place in `palette`.
pub fn explain(screen: &mut Screen, palette: &[Entry]) {
    let controls = screen
        .panels
        .iter_mut()
        .flat_map(|p| &mut p.rows)
        .flat_map(|r| &mut r.controls);
    for control in controls {
        let ControlId::Edit(a) = control.id else {
            continue;
        };
        if control.tip.is_some() {
            continue;
        }
        let tip = match a {
            EditAct::Brush(i) => palette.get(usize::from(i)).map(|e| entry(*e)),
            a => act(a),
        };
        control.tip = tip.map(str::to_owned);
    }
}
