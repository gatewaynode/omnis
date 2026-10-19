//! `omnis-cli migration draft` (M8.5 7h6): the next migration of a pack, drafted from what its
//! old version had that its new one lacks. Every name gone becomes a `Remove` step, with the
//! `Rename` to write instead when it lives on under another name; a race, class, background or
//! region cannot be removed, so it is left as a commented `Rename` the author must finish. A
//! fixed encounter or an object gone from a map is listed as a comment: it needs no step unless
//! it was renamed. The draft never guesses a rename.

use omnis_data::Data;
use omnis_data::migration::{Kind, compare};
use std::cmp::Ordering;
use std::fmt::Write;

/// A drafted migration: its text and how many steps it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The file's text.
    pub text: String,
    /// Live steps in it.
    pub steps: usize,
    /// Commented lines the author must decide.
    pub open: usize,
}

/// Draft the migration of `pack` from `old` (the packs with its old version) to `new` (with its
/// new one).
///
/// # Errors
/// The pack is missing from either side, or its new version is not after its old one, or after
/// the last migration it keeps.
pub fn draft(pack: &str, old: &Data, new: &Data) -> Result<Draft, String> {
    let version = |data: &Data, side: &str| {
        data.packs
            .iter()
            .find(|p| p.id == pack)
            .map(|p| p.version.clone())
            .ok_or_else(|| format!("pack '{pack}' is not in the {side} packs"))
    };
    let (from, to) = (version(old, "old")?, version(new, "new")?);
    if compare(&to, &from) != Some(Ordering::Greater) {
        return Err(format!(
            "the new version '{to}' must be an x.y.z after the old '{from}': raise it with \
             `omnis-cli pack seal --version`"
        ));
    }
    if let Some(last) = new.migrations.get(pack).and_then(|m| m.last())
        && compare(&to, &last.to) != Some(Ordering::Greater)
    {
        return Err(format!(
            "the new version '{to}' must be after '{}', where {} leads",
            last.to, last.file
        ));
    }
    let own = format!("{pack}:");
    let mut text = format!(
        "// {from} to {to}, drafted by `omnis-cli migration draft`. Each name below is gone from \
         {pack};\n// a `Remove` drops it from every save, so where one lives on under another name, \
         write the\n// `Rename` (or a `Replace` with what stands in for it) instead.\n(\n    \
         schema: 1,\n    to: \"{to}\",\n    steps: [\n"
    );
    let (mut steps, mut open) = (0, 0);
    for kind in Kind::ALL {
        for name in kind.names(&old.registry) {
            if !name.starts_with(&own) || kind.defined(&new.registry, name) {
                continue;
            }
            let k = kind.as_str();
            let rename = format!("Rename(kind: {k}, from: \"{name}\", to: \"<its new id>\"),");
            if kind.removable() {
                let _ = writeln!(text, "        // or: {rename}");
                let _ = writeln!(text, "        Remove(kind: {k}, id: \"{name}\"),");
                steps += 1;
            } else {
                let _ = writeln!(
                    text,
                    "        // A {k} cannot be removed: rename or replace it.\n        // {rename}"
                );
                open += 1;
            }
        }
    }
    for (id, data) in &old.maps {
        let Some(map) = old.registry.maps.name(*id).filter(|m| m.starts_with(&own)) else {
            continue;
        };
        let Some(now) = new.registry.maps.get(map).and_then(|m| new.maps.get(&m)) else {
            continue;
        };
        let gone = |was: Vec<&str>, is: Vec<&str>| -> Vec<String> {
            was.into_iter()
                .filter(|w| !is.contains(w))
                .map(String::from)
                .collect()
        };
        let encounters = gone(
            data.def.encounters.iter().map(|e| e.id.as_str()).collect(),
            now.def.encounters.iter().map(|e| e.id.as_str()).collect(),
        );
        let objects = gone(
            data.def.objects.iter().map(|o| o.id.as_str()).collect(),
            now.def.objects.iter().map(|o| o.id.as_str()).collect(),
        );
        for (what, step, ids) in [
            ("encounter", "RenameEncounter", encounters),
            ("object", "RenameObject", objects),
        ] {
            for from in ids {
                let _ = writeln!(
                    text,
                    "        // {what} '{from}' is gone from {map}; if it was renamed:\n        \
                     // {step}(map: \"{map}\", from: \"{from}\", to: \"<its new id>\"),"
                );
                open += 1;
            }
        }
    }
    text.push_str("    ],\n)\n");
    Ok(Draft { text, steps, open })
}
