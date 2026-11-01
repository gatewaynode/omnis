//! The portal's form (the entrances and exits enhancement, W1 step 2): an id one across the
//! loaded maps, the link to its partner, the way the party comes out, the marker.

use super::fields::{among, facings, free_id};
use super::{Placing, id_taken};
use crate::editor::form::{self, At, Form, FormKind};
use omnis_sim::omnis_data::{Data, MapDef, Portal, SlotKind};

/// A portal's id as typed: trimmed, 1 to 32 printable ASCII characters.
pub(super) fn portal_id(text: &str) -> Result<String, String> {
    let text = text.trim();
    match Portal::id_fault(text) {
        None => Ok(text.to_owned()),
        Some("has no id") => Err("a portal needs an id".to_owned()),
        Some(_) => Err(format!(
            "a portal id is 1 to {} printable ASCII characters",
            Portal::ID_MAX
        )),
    }
}

/// Whether `id` is a portal's on the map or on any loaded map (ids are one across the world).
fn portal_id_taken(def: &MapDef, data: &Data, id: &str) -> bool {
    id_taken(def, Placing::Portal, id)
        || data
            .maps
            .values()
            .any(|m| m.def.id != def.id && m.def.portals.iter().any(|p| p.id == id))
}

/// A portal's form (the entrances and exits enhancement, W1 step 2, links): its id (a `fresh`
/// one minted free of every loaded map's), the id of the portal it links to (empty: unlinked;
/// a stamp's detail), the way the party faces coming out of it (a fresh one faces the author:
/// the cursor's opposite, so the party comes back out the way it went in), and its marker, a
/// surface of the map's tileset the game draws on its cell.
pub(super) fn portal(
    def: &MapDef,
    data: &Data,
    at: At,
    was: Option<&Portal>,
    detail: Option<&str>,
    fresh: bool,
) -> Form {
    let id = match was.filter(|_| !fresh) {
        Some(p) => p.id.clone(),
        None => free_id("portal", (at.x, at.y), |id| portal_id_taken(def, data, id)),
    };
    let facing = match was.filter(|_| !fresh) {
        Some(p) => p.facing,
        None => at.facing.opposite(),
    };
    let link = was
        .and_then(|p| p.link.clone())
        .or_else(|| detail.map(str::to_owned))
        .unwrap_or_default();
    let markers: Vec<String> = data
        .registry
        .tilesets
        .get(&def.tileset)
        .and_then(|id| data.tilesets.get(&id))
        .map(|t| {
            t.surfaces
                .iter()
                .filter(|(_, s)| matches!(s.kind, SlotKind::Object))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default();
    let (markers, marker) = among(markers, was.and_then(|p| p.marker.as_deref()));
    Form::new(
        FormKind::Portal,
        format!("Portal at ({}, {})", at.x, at.y),
        at,
        vec![
            form::text("id", &id, Portal::ID_MAX),
            form::text("link", &link, Portal::ID_MAX),
            facings(facing),
            form::choice("marker", markers, marker),
        ],
        Vec::new(),
    )
}
