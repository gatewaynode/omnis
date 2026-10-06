//! `data/regions`: the temporal holders that own maps (ARCHITECTURE.md §4.4). A region's
//! company is how many people gather there; it weighs the party's shared time, which is what a
//! settlement catches up by when the party enters it (owner, 2026-10-05: "as more people gather
//! together in the same place time starts to align with them"). A wild region keeps its own
//! clock and never sets the party's date. Every map belongs to exactly one region.

use crate::error::DataError;
use crate::limits::{MAX_COLLECTION, string_fits};
use crate::registry::Interner;
use omnis_core::{MapId, RegionId, TextKey};
use omnis_expr::Rules;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The scale of company and stability: per mille.
pub const PER_MILLE: u16 = 1000;

/// The inputs a region's rule slot may declare.
pub const RULE_INPUTS: [&str; 3] = ["lived", "shared_time", "stability"];

/// Whether entering a region sets the party's date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RegionKind {
    /// A village, town or city: its date becomes the party's.
    Settlement,
    /// The wilds and dungeons: a clock of its own that never sets the party's date.
    Wild,
}

/// One region as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionDef {
    /// Schema version.
    pub schema: u32,
    /// `pack:region:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Settlement or wild.
    pub kind: RegionKind,
    /// How many people gather here, per mille: each minute the party lives here counts this
    /// much toward its shared time.
    pub company: u16,
    /// Temporal stability, per mille: the rule's drift (a stable town drifts little).
    pub stability: u16,
    /// The rules slot that reconciles this region (`time.settled`, `time.wild`).
    pub rule: String,
    /// Regions that reconcile alongside this one (a road, a river, trade).
    #[serde(default)]
    pub couplings: Vec<String>,
    /// The maps this region owns.
    #[serde(default)]
    pub maps: Vec<String>,
}

impl RegionDef {
    /// Self-contained checks; references are checked at resolution.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.company > PER_MILLE || self.stability > PER_MILLE {
            errors.push(DataError::new(
                file,
                "company and stability are per mille, 0..=1000",
            ));
        }
        if !string_fits(&self.rule) || self.rule.is_empty() {
            errors.push(DataError::new(file, "rule is empty or over the size limit"));
        }
        for (list, one) in [(&self.couplings, "coupling"), (&self.maps, "map")] {
            if list.len() > MAX_COLLECTION {
                errors.push(DataError::new(file, format!("too many {one}s")));
                continue;
            }
            let mut seen = BTreeSet::new();
            for id in list {
                if !string_fits(id) {
                    errors.push(DataError::new(
                        file,
                        format!("{one} id is over the size limit"),
                    ));
                } else if !seen.insert(id.as_str()) {
                    errors.push(DataError::new(
                        file,
                        format!("{one} '{id}' is listed twice"),
                    ));
                }
            }
        }
        if self.couplings.contains(&self.id) {
            errors.push(DataError::new(file, "a region cannot couple to itself"));
        }
    }
}

/// A region with its references interned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// Display name.
    pub name: TextKey,
    /// Settlement or wild.
    pub kind: RegionKind,
    /// Per mille.
    pub company: u16,
    /// Per mille.
    pub stability: u16,
    /// The rules slot that reconciles it.
    pub rule: String,
    /// Coupled regions, in file order.
    pub couplings: Vec<RegionId>,
    /// Owned maps, in file order.
    pub maps: Vec<MapId>,
}

/// Intern every region and check its references: maps and couplings exist, every map belongs to
/// exactly one region, the name is a text key, and the rule is a slot that declares only
/// [`RULE_INPUTS`]. Returns the regions and each map's region.
pub(crate) fn resolve(
    raw: BTreeMap<String, (PathBuf, RegionDef)>,
    maps: &BTreeMap<&str, MapId>,
    regions: &mut Interner<RegionId>,
    text: &Interner<TextKey>,
    rules: &Rules,
    errors: &mut Vec<DataError>,
) -> (BTreeMap<RegionId, Region>, BTreeMap<MapId, RegionId>) {
    for id in raw.keys() {
        regions.intern(id);
    }
    let mut owner: BTreeMap<MapId, RegionId> = BTreeMap::new();
    let mut out = BTreeMap::new();
    for (id, (file, def)) in raw {
        let region = regions.intern(&id);
        let name = text.get(&def.name).unwrap_or_else(|| {
            errors.push(DataError::new(
                &file,
                format!(
                    "name text key '{}' is not defined in any language",
                    def.name
                ),
            ));
            TextKey(0)
        });
        match rules.slot(&def.rule) {
            None => errors.push(DataError::new(
                &file,
                format!("rule '{}' is not a rules slot", def.rule),
            )),
            Some(slot) => {
                for input in slot.inputs() {
                    if !RULE_INPUTS.contains(&input.as_str()) {
                        errors.push(DataError::new(
                            &file,
                            format!(
                                "rule '{}' declares input '{input}'; a region's rule takes only {}",
                                def.rule,
                                RULE_INPUTS.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
        let couplings = def
            .couplings
            .iter()
            .filter_map(|c| {
                let found = regions.get(c);
                if found.is_none() {
                    errors.push(DataError::new(
                        &file,
                        format!("coupling '{c}' is not defined by any loaded pack"),
                    ));
                }
                found
            })
            .collect();
        let mut owned = Vec::with_capacity(def.maps.len());
        for m in &def.maps {
            let Some(&map) = maps.get(m.as_str()) else {
                errors.push(DataError::new(
                    &file,
                    format!("map '{m}' is not defined by any loaded pack"),
                ));
                continue;
            };
            if let Some(other) = owner.insert(map, region) {
                errors.push(DataError::new(
                    &file,
                    format!(
                        "map '{m}' already belongs to region '{}'",
                        regions.name(other).unwrap_or_default()
                    ),
                ));
            }
            owned.push(map);
        }
        out.insert(
            region,
            Region {
                name,
                kind: def.kind,
                company: def.company,
                stability: def.stability,
                rule: def.rule,
                couplings,
                maps: owned,
            },
        );
    }
    (out, owner)
}
