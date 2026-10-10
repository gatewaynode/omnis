//! `data/services`: the buildings a town is made of (PRD §7.4). A service is data placed on a map
//! tile by a site; nothing makes a map a town beyond its sites. What a service does is its kind;
//! what it offers is its stock; every price is a rule slot.

use crate::error::DataError;
use crate::limits::{MAX_COLLECTION, string_fits};
use crate::map::{Cell, MapDef};
use crate::registry::Interner;
use omnis_core::ServiceId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// What a service does for the party.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ServiceKind {
    /// A room for the night: a long rest; saves under inn-only rules.
    Inn,
    /// Healing, curing, and raising the dead, for a donation.
    Temple,
    /// Levels granted for a fee.
    Trainer,
    /// Buying and selling items.
    Smith,
    /// Food and rumors.
    Tavern,
    /// Gold kept safe.
    Bank,
    /// Spells for sale.
    Guild,
}

/// One service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDef {
    /// Schema version.
    pub schema: u32,
    /// `pack:service:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// What it does.
    pub kind: ServiceKind,
    /// Item ids for sale; a smith's only.
    #[serde(default)]
    pub items: Vec<String>,
    /// Spell ids for sale; a guild's or a temple's only (PRD §8.2).
    #[serde(default)]
    pub spells: Vec<String>,
    /// The rumors told; a tavern's only.
    #[serde(default)]
    pub rumors: Vec<RumorDef>,
}

/// One rumor: its text and the minute on the region's clock it happened (M8). A rumor is not
/// told before its region's clock reaches `at`, and is told with how long ago that was. A bare
/// text key is read as a rumor that happened at the origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RumorRow")]
pub struct RumorDef {
    /// Text key; the text may hold `{ago}`.
    pub text: String,
    /// The minute it happened, on the region's clock.
    pub at: i64,
}

/// A rumor as written: a bare key or the full form.
#[derive(Deserialize)]
#[serde(untagged)]
enum RumorRow {
    Key(String),
    Full { text: String, at: i64 },
}

impl From<RumorRow> for RumorDef {
    fn from(row: RumorRow) -> RumorDef {
        match row {
            RumorRow::Key(text) => RumorDef { text, at: 0 },
            RumorRow::Full { text, at } => RumorDef { text, at },
        }
    }
}

impl ServiceDef {
    /// Self-contained checks; every problem is pushed. References are checked at resolution.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        let rumors: Vec<String> = self.rumors.iter().map(|r| r.text.clone()).collect();
        if self.rumors.iter().any(|r| r.at < 0) {
            errors.push(DataError::new(file, "a rumor's 'at' is negative"));
        }
        let lists: [(&Vec<String>, &str, &str, &[ServiceKind]); 3] = [
            (&self.items, "item", "items", &[ServiceKind::Smith]),
            (
                &self.spells,
                "spell",
                "spells",
                &[ServiceKind::Guild, ServiceKind::Temple],
            ),
            (&rumors, "rumor", "rumors", &[ServiceKind::Tavern]),
        ];
        for (list, one, many, owners) in lists {
            if !list.is_empty() && !owners.contains(&self.kind) {
                errors.push(DataError::new(
                    file,
                    format!(
                        "{many} are stocked only by kind {}; this service is kind {:?}",
                        kinds(owners),
                        self.kind
                    ),
                ));
            }
            if list.len() > MAX_COLLECTION {
                errors.push(DataError::new(file, format!("too many {many}")));
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
    }
}

/// The kinds that may stock a list, as an error names them: `Guild or Temple`.
fn kinds(owners: &[ServiceKind]) -> String {
    owners
        .iter()
        .map(|k| format!("{k:?}"))
        .collect::<Vec<_>>()
        .join(" or ")
}

/// A service placed on a map tile: stepping onto the tile enters it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Service id.
    pub service: String,
}

/// A site with its service interned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSite {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// The service.
    pub service: ServiceId,
}

/// Checks that need no other file: every site inside the map, one per tile, and never on a
/// tile a portal or an encounter already claims (stepping there would have two meanings).
pub(crate) fn validate_sites(def: &MapDef, file: &Path, errors: &mut Vec<DataError>) {
    if def.sites.len() > MAX_COLLECTION {
        errors.push(DataError::new(file, "too many sites"));
        return;
    }
    let mut seen = BTreeSet::new();
    for site in &def.sites {
        let (x, y) = (site.x, site.y);
        if x >= def.width || y >= def.height {
            errors.push(DataError::new(
                file,
                format!("site at ({x}, {y}) is outside the map"),
            ));
        }
        if !seen.insert((x, y)) {
            errors.push(DataError::new(
                file,
                format!("site at ({x}, {y}) shares its tile with another site"),
            ));
        }
        if def.portals.iter().any(|p| (p.x, p.y) == (x, y)) {
            errors.push(DataError::new(
                file,
                format!("site at ({x}, {y}) shares its tile with a portal"),
            ));
        }
        if def.encounters.iter().any(|e| (e.x, e.y) == (x, y)) {
            errors.push(DataError::new(
                file,
                format!("site at ({x}, {y}) shares its tile with an encounter"),
            ));
        }
    }
}

/// Sites must name a loaded service and stand on a tile the party can enter. `cells` is empty
/// when the layout did not parse; the tile check is skipped then (the layout's errors stand).
pub(crate) fn resolve_sites(
    def: &MapDef,
    cells: &[Cell],
    services: &Interner<ServiceId>,
    file: &Path,
    errors: &mut Vec<DataError>,
) -> Vec<ResolvedSite> {
    let mut sites = Vec::with_capacity(def.sites.len());
    for site in &def.sites {
        let (x, y) = (site.x, site.y);
        let index = usize::from(y) * usize::from(def.width) + usize::from(x);
        let tile = cells.get(index).filter(|_| x < def.width);
        if tile.is_some_and(|c| !def.terrains[usize::from(c.terrain)].passable) {
            errors.push(DataError::new(
                file,
                format!("site at ({x}, {y}) is on a tile the party cannot enter"),
            ));
        }
        match services.get(&site.service) {
            Some(service) => sites.push(ResolvedSite { x, y, service }),
            None => errors.push(DataError::new(
                file,
                format!(
                    "site at ({x}, {y}) names service '{}', which is not defined by any loaded pack",
                    site.service
                ),
            )),
        }
    }
    sites
}
