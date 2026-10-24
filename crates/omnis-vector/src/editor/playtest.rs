//! Playtest (M5.8, `tasks/plans/m5-editor-v1.md` §Client side "Playtest"): a game on the edited
//! map as the editor holds it, with the fixed party standing on the cursor's cell facing the
//! cursor's way; the editor's document is untouched and the game is never saved. Bevy-free: what
//! stops a playtest, the data it plays on, and the step that puts the party on the cursor; the
//! shell starts the session and parks the editor (`shell/editor.rs`).

use super::Editor;
use omnis_sim::DevCommand;
use omnis_sim::omnis_data::Data;
use std::sync::Arc;

/// Why a playtest cannot start now, in one line; none when it can: a map is open and loads, the
/// cursor stands on passable ground, and the build has the dev tools (the teleport is theirs).
#[must_use]
pub fn blocked(editor: &Editor) -> Option<String> {
    if !cfg!(feature = "devtools") {
        return Some("needs the dev tools build".to_owned());
    }
    let def = editor.def()?;
    let errors = &editor.status.errors;
    if let Some(first) = errors.first() {
        return Some(format!(
            "{} error{}: {first}",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" }
        ));
    }
    if editor.resolved.is_none() {
        return Some("the map has not loaded".to_owned());
    }
    let (x, y) = editor.cursor;
    match editor.grid().and_then(|g| g.tile(x, y).flatten()) {
        None => Some("the cursor stands on no ground".to_owned()),
        Some(t) => {
            let terrain = def.terrains.get(usize::from(t))?;
            (!terrain.passable).then(|| {
                format!(
                    "the cursor stands on {}, which is not passable",
                    terrain.name
                )
            })
        }
    }
}

/// The loaded data with the edited map in it, as the game will read it: a copy, the map given
/// its id (the loaded one, or a fresh one for a map the packs lack) and the editor's resolve put
/// in its place. The packs themselves are never touched. None with no resolved map.
#[must_use]
pub fn data_for(editor: &Editor, data: &Data) -> Option<Arc<Data>> {
    let map = editor.resolved.as_ref()?;
    let mut temp = data.clone();
    let id = temp.registry.maps.intern(&map.def.id);
    temp.maps.insert(id, map.clone());
    Some(Arc::new(temp))
}

/// The step that puts the party on the cursor's cell, facing as the cursor does. None with no
/// map open.
#[must_use]
pub fn teleport(editor: &Editor) -> Option<DevCommand> {
    let (x, y) = editor.cursor;
    Some(DevCommand::Teleport {
        map: editor.map_id()?,
        level: editor.level,
        x,
        y,
        facing: editor.facing,
    })
}
