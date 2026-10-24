//! The icons the client draws by name (owner, 2026-10-24: the Lucide set through `omnis-icon`),
//! read once and kept. Bevy-free.

use omnis_icon::Icon;
use omnis_sim::omnis_data::ServiceKind;
use std::sync::OnceLock;

/// The activities' placeholder: a grinning face for an activity with no icon of its own (a
/// service the loaded packs lack); the set has no `smile`.
pub const ACTIVITY: &str = "face-grinning";

/// The icon `name` of the set, read once; an icon the set lacks is empty, drawing nothing.
#[must_use]
pub fn named(name: &str, cell: &'static OnceLock<Icon>) -> &'static Icon {
    cell.get_or_init(|| {
        omnis_icon::get(name).unwrap_or_else(|| Icon {
            name: name.to_owned(),
            strokes: Vec::new(),
        })
    })
}

/// The activities' placeholder icon.
#[must_use]
pub fn activity() -> &'static Icon {
    static CELL: OnceLock<Icon> = OnceLock::new();
    named(ACTIVITY, &CELL)
}

/// Every kind of service, in the icons' order.
const KINDS: [ServiceKind; 7] = [
    ServiceKind::Inn,
    ServiceKind::Temple,
    ServiceKind::Trainer,
    ServiceKind::Smith,
    ServiceKind::Tavern,
    ServiceKind::Bank,
    ServiceKind::Guild,
];

/// The icon of a kind of service (owner, 2026-10-24: the first activities): a bed for an inn,
/// a church for a temple, a dumbbell for a trainer, an anvil for a smith, a beer for a tavern,
/// coins for a bank, a shield for a guild.
#[must_use]
pub const fn service_icon(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Inn => "bed",
        ServiceKind::Temple => "church",
        ServiceKind::Trainer => "dumbbell",
        ServiceKind::Smith => "anvil",
        ServiceKind::Tavern => "beer",
        ServiceKind::Bank => "coins",
        ServiceKind::Guild => "shield",
    }
}

/// The icon of a kind of service, read once.
#[must_use]
pub fn service(kind: ServiceKind) -> &'static Icon {
    static CELLS: [OnceLock<Icon>; 7] = [const { OnceLock::new() }; 7];
    let i = KINDS.iter().position(|k| *k == kind).unwrap_or(0);
    named(service_icon(kind), &CELLS[i])
}

/// An activity's icon: its service's when it is a service the packs know, else the placeholder.
#[must_use]
pub fn activity_of(kind: Option<ServiceKind>) -> &'static Icon {
    kind.map_or_else(activity, service)
}
