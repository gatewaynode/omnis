//! The vocabulary every event-text file shares: the names events refer to by id, the two
//! lengths of a line, and the roll math as text. `combat_text.rs`, `spell_text.rs` and
//! `item_text.rs` build on it. Bevy-free.

use crate::font::fit;
use crate::sim::Views;
use omnis_sim::ActorRef;
use omnis_sim::omnis_core::{Calendar, CharacterId, Coins, RollTrace};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::registry::Interner;
use std::collections::BTreeMap;

/// How long ago `minutes` was, in the calendar's days, months of 30 days, and years:
/// "today", "yesterday", "3 days ago", "a month ago", "2 years ago".
#[must_use]
pub fn ago_text(minutes: i64, calendar: Calendar) -> String {
    let days = calendar.day(minutes.max(0));
    let per_year = i64::from(calendar.days_per_year.max(1));
    let count = |n: i64, one: &str, many: &str| {
        if n == 1 {
            format!("a {one} ago")
        } else {
            format!("{n} {many} ago")
        }
    };
    match days {
        0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        d if d < 30 => format!("{d} days ago"),
        d if d < per_year => count(d / 30, "month", "months"),
        d => count(d / per_year, "year", "years"),
    }
}

/// Cells a long line may take: the band's message line.
pub const LONG_CELLS: usize = 100;
/// Cells a short line may take: a roll-log row under the viewport.
pub const SHORT_CELLS: usize = 39;
// A long line fits the band's log and its message line.
const _: () = assert!(LONG_CELLS <= crate::band::LOG_CELLS);
const _: () = assert!(LONG_CELLS <= crate::band::BAND_COLUMNS);

/// The names events refer to by id: members by `CharacterId`, definitions by string id. Members are remembered by id after they leave the party
/// and stacks after a fight ends, so the batch that ends a fight still reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    pub(crate) members: BTreeMap<CharacterId, String>,
    /// Label and initial count per stack index.
    pub(crate) stacks: Vec<(String, u8)>,
    conditions: BTreeMap<String, String>,
    spells: BTreeMap<String, String>,
    items: BTreeMap<String, String>,
    /// A service's name and its rumors, in its pack's order.
    services: BTreeMap<String, (String, Vec<String>)>,
    /// A map's rest-event lines, in its file's order.
    rest_events: BTreeMap<String, Vec<String>>,
    /// Class features by text key (a level-up names the ones it brings).
    features: BTreeMap<String, String>,
    /// The calendar, for how long ago something happened.
    calendar: Calendar,
}

impl Names {
    /// Names for the world as its views show it.
    #[must_use]
    pub fn new(views: &Views, data: &Data) -> Names {
        let mut names = Names::default();
        names.refresh(views, data);
        names
    }

    /// Names for a world, as the app's views would show it (tests).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn of_world(world: &omnis_sim::World, data: &Data) -> Names {
        Names::new(&Views::of(world, data), data)
    }

    /// Learn the current members and, while monsters stand there, the current stacks.
    pub fn refresh(&mut self, views: &Views, data: &Data) {
        for member in &views.party.members {
            self.members.insert(member.member, member.name.clone());
        }
        if let Some(combat) = &views.combat {
            self.stacks = combat
                .stacks
                .iter()
                .map(|s| (data.label("en", &s.name).to_owned(), s.initial))
                .collect();
        }
        if self.conditions.is_empty() {
            for (id, condition) in &data.conditions {
                self.conditions.insert(
                    key(&data.registry.conditions, *id),
                    data.label("en", &condition.name).to_owned(),
                );
            }
        }
        if self.spells.is_empty() {
            for (id, spell) in &data.spells {
                self.spells.insert(
                    key(&data.registry.spells, *id),
                    data.label("en", &spell.name).to_owned(),
                );
            }
        }
        if self.items.is_empty() {
            for (id, item) in &data.items {
                self.items.insert(
                    key(&data.registry.items, *id),
                    data.label("en", &item.name).to_owned(),
                );
            }
        }
        self.calendar = data.calendar();
        if self.services.is_empty() {
            for (id, service) in &data.services {
                let rumors = service
                    .rumors
                    .iter()
                    .map(|rumor| data.label("en", &rumor.text).to_owned())
                    .collect();
                let name = data.label("en", &service.name).to_owned();
                self.services
                    .insert(key(&data.registry.services, *id), (name, rumors));
            }
        }
        if self.features.is_empty() {
            for class in data.classes.values() {
                for feature in &class.features {
                    let label = data.label("en", &feature.name).to_owned();
                    self.features.insert(feature.name.clone(), label);
                }
            }
        }
        if self.rest_events.is_empty() {
            for (id, map) in &data.maps {
                let lines = map
                    .def
                    .rest_events
                    .iter()
                    .map(|e| data.label("en", &e.text).to_owned())
                    .collect();
                self.rest_events
                    .insert(key(&data.registry.maps, *id), lines);
            }
        }
    }

    /// A member's name.
    #[must_use]
    pub fn member(&self, id: CharacterId) -> &str {
        self.members.get(&id).map_or("?", String::as_str)
    }

    /// Who an actor is: `Brenna`, `Goblins` for a stack, `Goblin 2` for one of several.
    #[must_use]
    pub fn actor(&self, actor: &ActorRef) -> String {
        match actor {
            ActorRef::Member(id) => self.member(*id).to_owned(),
            ActorRef::Stack(stack) => match self.stacks.get(usize::from(*stack)) {
                Some((label, 1)) => label.clone(),
                Some((label, _)) => format!("{label}s"),
                None => format!("Stack {stack}"),
            },
            ActorRef::Monster { stack, index } => match self.stacks.get(usize::from(*stack)) {
                Some((label, 1)) => label.clone(),
                Some((label, _)) => format!("{label} {}", index + 1),
                None => format!("Stack {stack} #{}", index + 1),
            },
        }
    }

    /// A condition's name.
    #[must_use]
    pub fn condition(&self, id: &str) -> &str {
        self.conditions.get(id).map_or("?", String::as_str)
    }

    /// A spell's name.
    #[must_use]
    pub fn spell(&self, id: &str) -> &str {
        self.spells.get(id).map_or("?", String::as_str)
    }

    /// A class feature's name, by its text key.
    #[must_use]
    pub fn feature<'a>(&'a self, key: &'a str) -> &'a str {
        self.features.get(key).map_or(key, String::as_str)
    }

    /// An item's name.
    #[must_use]
    pub fn item(&self, id: &str) -> &str {
        self.items.get(id).map_or("?", String::as_str)
    }

    /// A service's name.
    #[must_use]
    pub fn service(&self, id: &str) -> &str {
        self.services.get(id).map_or("?", |(name, _)| name.as_str())
    }

    /// One of a service's rumors, by its row, told `ago` minutes after it happened.
    #[must_use]
    pub fn rumor(&self, id: &str, rumor: u16, ago: i64) -> String {
        let text = self
            .services
            .get(id)
            .and_then(|(_, rumors)| rumors.get(usize::from(rumor)))
            .map_or("?", String::as_str);
        let ago = ago_text(ago, self.calendar);
        omnis_sim::omnis_data::text::fill(text, &[("ago", &ago)])
    }

    /// The line of a map's rest event.
    #[must_use]
    pub fn rest_event(&self, map: &str, entry: u16) -> &str {
        self.rest_events
            .get(map)
            .and_then(|lines| lines.get(usize::from(entry)))
            .map_or("?", String::as_str)
    }
}

/// The string id events carry for a registry number: `#n` for one no pack names, as the
/// engine writes it.
fn key<I: Copy + From<u32> + Into<u32>>(registry: &Interner<I>, id: I) -> String {
    registry
        .name(id)
        .map_or_else(|| format!("#{}", id.into()), str::to_owned)
}

/// One event as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// With the math, at most `LONG_CELLS`.
    pub long: String,
    /// The outcome alone, at most `SHORT_CELLS`.
    pub short: String,
}

impl Line {
    pub(crate) fn new(long: String, short: String) -> Line {
        Line {
            long: fit(&long, LONG_CELLS),
            short: fit(&short, SHORT_CELLS),
        }
    }

    pub(crate) fn same(text: String) -> Line {
        Line::new(text.clone(), text)
    }
}

/// A purse or price in copper broken out by coin, largest first: `15 gp 3 sp 7 cp`. The
/// inventory, the debug menu and every town price show every coin (owner, 2026-09-27; a town
/// price can end in silver); elsewhere money shows as whole gold rounded down
/// (`money::gp_floor`).
#[must_use]
pub fn coins(cp: u32) -> String {
    Coins::of(cp).to_string()
}

/// `1d8+2 [5]=7`: the trace without its stream name.
#[must_use]
pub fn trace_math(trace: &RollTrace) -> String {
    format!("{} {}={}", trace.dice, faces(trace), trace.total)
}

pub(crate) fn faces(trace: &RollTrace) -> String {
    let faces = trace
        .rolls
        .iter()
        .map(|r| r.value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{faces}]")
}

#[cfg(test)]
mod ago_tests {
    use super::ago_text;
    use omnis_sim::omnis_core::Calendar;

    #[test]
    fn how_long_ago_reads_in_days_months_and_years() {
        let c = Calendar::default();
        let day = 1440;
        for (minutes, text) in [
            (0, "today"),
            (day - 1, "today"),
            (day, "yesterday"),
            (3 * day, "3 days ago"),
            (29 * day, "29 days ago"),
            (30 * day, "a month ago"),
            (75 * day, "2 months ago"),
            (360 * day, "a year ago"),
            (3 * 360 * day + 5, "3 years ago"),
            (-5, "today"),
        ] {
            assert_eq!(ago_text(minutes, c), text, "{minutes}");
        }
    }
}
