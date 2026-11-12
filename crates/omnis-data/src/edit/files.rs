//! Every other pack file as text (ARCHITECTURE §8.2): checked by directory with the loader's
//! own functions before it is written as typed. The directory says the type; a path the loader
//! never reads is refused, since nothing would check it.

use crate::character::{Background, Class, Race};
use crate::condition::Condition;
use crate::content::Content;
use crate::edit::kinds::EditError;
use crate::error::DataError;
use crate::flag::FlagDef;
use crate::item::Item;
use crate::levels;
use crate::loader::{self, Data, HasSchema};
use crate::loot::LootTable;
use crate::manifest::PackManifest;
use crate::map::MapDef;
use crate::map_data::MapData;
use crate::migration::Migration;
use crate::monster::Monster;
use crate::notes::NotesFile;
use crate::object::ObjectKind;
use crate::region::RegionDef;
use crate::rules::RulesFile;
use crate::service::ServiceDef;
use crate::spell::Spell;
use crate::text::TextFile;
use crate::tileset::Tileset;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::path::Path;

/// The directories the loader reads, root-relative, which `files()` lists.
pub const DIRECTORIES: [&str; 4] = ["data", "text", "notes", "migrations"];

/// What the loader would say of `text` were it the file at `rel`. Empty means it loads, as far
/// as one file can tell: a content file's references to other files are checked when the pack
/// loads again.
///
/// # Errors
/// `UnknownDirectory`: the loader reads nothing at `rel`.
pub fn check_text(rel: &str, text: &str, data: &Data) -> Result<Vec<DataError>, EditError> {
    let path = Path::new(rel);
    let mut errors = Vec::new();
    let dir = match rel.rsplit_once('/') {
        Some((dir, _)) => dir,
        None => "",
    };
    match dir {
        "" if rel == "pack.ron" => {
            if let Some(m) = loader::parse_checked::<PackManifest>(text, path, &mut errors) {
                loader::check_manifest(&m, path, &data.packs, &mut errors);
            }
        }
        "data/tiles" => typed::<Tileset>("tileset", text, path, &mut errors),
        "data/maps" => {
            if let Some(def) = loader::parse_checked::<MapDef>(text, path, &mut errors) {
                loader::check_id(&def.id, "map", path, &mut errors);
                def.validate(path, &mut errors);
                levels::parse(&def, path, &mut errors);
                if errors.is_empty() {
                    errors.extend(MapData::resolve(&def, path, data).err().unwrap_or_default());
                }
            }
        }
        "data/races" => typed::<Race>("race", text, path, &mut errors),
        "data/classes" => typed::<Class>("class", text, path, &mut errors),
        "data/backgrounds" => typed::<Background>("background", text, path, &mut errors),
        "data/items" => typed::<Item>("item", text, path, &mut errors),
        "data/conditions" => typed::<Condition>("condition", text, path, &mut errors),
        "data/spells" => typed::<Spell>("spell", text, path, &mut errors),
        "data/monsters" => typed::<Monster>("monster", text, path, &mut errors),
        "data/rules" => typed::<RulesFile>("rules", text, path, &mut errors),
        "data/services" => typed::<ServiceDef>("service", text, path, &mut errors),
        "data/regions" => typed::<RegionDef>("region", text, path, &mut errors),
        "data/objects" => typed::<ObjectKind>("object", text, path, &mut errors),
        "data/loot" => typed::<LootTable>("loot", text, path, &mut errors),
        "data/flags" => typed::<FlagDef>("flag", text, path, &mut errors),
        "notes" => {
            if let Some(notes) = loader::parse_checked::<NotesFile>(text, path, &mut errors) {
                notes.validate(path, &mut errors);
            }
        }
        "migrations" => {
            loader::parse_checked::<Migration>(text, path, &mut errors);
        }
        lang if lang
            .strip_prefix("text/")
            .is_some_and(|l| !l.is_empty() && !l.contains('/')) =>
        {
            if let Some(file) = loader::parse_checked::<TextFile>(text, path, &mut errors) {
                loader::check_text_file(file, path, &mut BTreeMap::new(), &mut errors);
            }
        }
        _ => return Err(EditError::UnknownDirectory(rel.to_owned())),
    }
    Ok(errors)
}

/// Parse, check the id's kind, and validate one content file.
fn typed<T: DeserializeOwned + HasSchema + Content>(
    kind: &str,
    text: &str,
    path: &Path,
    errors: &mut Vec<DataError>,
) {
    if let Some(value) = loader::parse_checked::<T>(text, path, errors) {
        loader::check_id(value.id(), kind, path, errors);
        value.validate(path, errors);
    }
}
