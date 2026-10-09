//! Subjective time (PRD §7.8, ARCHITECTURE.md §4.4): the party's age, its shared time and its
//! date, and reconciliation when holders meet. Time aligns where people gather (owner,
//! 2026-10-05): each minute the party lives counts toward its shared time at the company of the
//! region it is in; a settlement catches up by the party's shared time since their last contact
//! and sets the party's date; a wild region catches up by the party's own lived time and never
//! sets the date. Coupled regions reconcile once with the region the party entered.
//!
//! The interaction points, each tested: **region entry** (a portal to a map of another region,
//! a dev teleport, `DevCommand::Reconcile`). Not yet, because v1 content has no such holders:
//! meeting an actor, inspecting a project, two parties meeting, the start of an encounter
//! (monsters are not holders).

use crate::PARTY;
use crate::bus::{self, Bus, Host, Signal, Subscriber, Topic};
use crate::combat::Roller;
use crate::event::Event;
use crate::time_view::holder_name;
use crate::world::World;
use alloc::vec;
use alloc::vec::Vec;
use omnis_core::{Contact, EraId, HolderId, RegionId, StreamName};
use omnis_data::omnis_expr::Value;
use omnis_data::{Data, RegionKind};
use serde::{Deserialize, Serialize};

/// The party's shared time and the date it believes (§4.4). Its age is `clocks[PARTY]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyTime {
    /// Minutes lived × the company of where they were lived, per mille.
    pub shared_milli: i64,
    /// The date the party believes, in minutes on the calendar: its own reckoning, set to a
    /// settlement's date on entering it.
    pub date: i64,
    /// The era of that date.
    pub era: EraId,
}

impl Default for PartyTime {
    fn default() -> PartyTime {
        PartyTime {
            shared_milli: 0,
            date: 0,
            era: EraId(0),
        }
    }
}

/// The cap on one reconciliation when the rules do not give one: ten years of 360 days.
const DEFAULT_CAP: i64 = 10 * 360 * 1440;

/// The subscriptions a new world starts with: every region's entry to reconciliation.
#[must_use]
pub fn subscriptions(data: &Data) -> Bus {
    let mut bus = Bus::default();
    for region in data.regions.keys() {
        bus.subscribe(Topic::Region(*region), Subscriber::Reconcile);
    }
    bus
}

/// The clock of the region the party stands in: 0 before it was ever met.
#[must_use]
pub fn region_clock(world: &World, data: &Data) -> i64 {
    data.maps
        .get(&world.position.map)
        .and_then(|m| world.clocks.get(&HolderId::Region(m.region)))
        .map_or(0, |c| c.elapsed)
}

/// The minutes the party lived, at the company of the region of the map it is on.
pub(crate) fn live(world: &mut World, data: &Data, minutes: u32) {
    let company = data
        .maps
        .get(&world.position.map)
        .and_then(|m| data.regions.get(&m.region))
        .map_or(0, |r| i64::from(r.company));
    let t = &mut world.party_time;
    t.shared_milli = t
        .shared_milli
        .saturating_add(i64::from(minutes).saturating_mul(company));
    t.date = t.date.saturating_add(i64::from(minutes));
}

/// The party moved from the map `from` to the map it is on now: when the region changed, the
/// region it entered is raised on the bus.
pub(crate) fn moved(
    world: &mut World,
    data: &Data,
    from: omnis_core::MapId,
    events: &mut Vec<Event>,
) {
    let region = |map| data.maps.get(&map).map(|m| m.region);
    let (Some(was), Some(now)) = (region(from), region(world.position.map)) else {
        return;
    };
    if was != now {
        enter(world, data, now, Some(was), events);
    }
}

/// Raise the party's entry into `region` and deliver it.
pub(crate) fn enter(
    world: &mut World,
    data: &Data,
    region: RegionId,
    from: Option<RegionId>,
    events: &mut Vec<Event>,
) {
    let mut host = Sim {
        world,
        data,
        events,
    };
    let dropped = bus::drain(&mut host, vec![Signal::Entered { region, from }]);
    if dropped > 0 {
        events.push(Event::SignalsDropped { count: dropped });
    }
}

/// The simulation as the bus's host.
struct Sim<'a> {
    world: &'a mut World,
    data: &'a Data,
    events: &'a mut Vec<Event>,
}

impl Host for Sim<'_> {
    type Signal = Signal;
    type Subscriber = Subscriber;

    fn bus(&self) -> &Bus {
        &self.world.bus
    }

    fn deliver(&mut self, to: Subscriber, signal: &Signal, _raised: &mut Vec<Signal>) {
        match (to, signal) {
            (Subscriber::Reconcile, Signal::Entered { region, .. }) => {
                reconcile_party(self.world, self.data, *region, self.events);
            }
            // Reactions answer in a fight, through the fight's own host.
            (Subscriber::Reactions, _) | (Subscriber::Reconcile, Signal::Battle(_)) => {}
        }
    }
}

/// The party meets `region`: the region catches up, both remember, a settlement sets the
/// party's date, and its couplings reconcile with it.
fn reconcile_party(world: &mut World, data: &Data, region: RegionId, events: &mut Vec<Event>) {
    let Some(def) = data.regions.get(&region) else {
        return;
    };
    let (a, b) = (PARTY, HolderId::Region(region));
    let last = contact(world, a, b);
    let age = world.party_clock().elapsed;
    let lived = age.saturating_sub(last.self_elapsed).max(0);
    let shared = world
        .party_time
        .shared_milli
        .saturating_sub(last.self_shared)
        .max(0)
        / 1000;
    let delta = catch_up(world, data, a, b, &def.rule, lived, shared, def.stability);
    let clock = advance_holder(world, b, delta);
    let shared_now = world.party_time.shared_milli;
    remember(world, a, b, age, clock.elapsed, shared_now);
    events.push(Event::Reconciled {
        a: holder_name(a, data),
        b: holder_name(b, data),
        delta_a: lived,
        delta_b: delta,
        era_b: clock.era,
    });
    if def.kind == RegionKind::Settlement {
        world.party_time.date = clock.elapsed;
        world.party_time.era = clock.era;
    }
    for coupled in def.couplings.clone() {
        reconcile_coupled(world, data, region, coupled, events);
    }
}

/// `coupled` catches up with `region` by the rule over `region`'s change since they last met
/// (coupling depth one: it does not pass the change on).
fn reconcile_coupled(
    world: &mut World,
    data: &Data,
    region: RegionId,
    coupled: RegionId,
    events: &mut Vec<Event>,
) {
    let Some(def) = data.regions.get(&coupled) else {
        return;
    };
    let (a, b) = (HolderId::Region(region), HolderId::Region(coupled));
    let last = contact(world, a, b);
    let now = world.clocks.get(&a).map_or(0, |c| c.elapsed);
    let lived = now.saturating_sub(last.self_elapsed).max(0);
    let delta = catch_up(world, data, a, b, &def.rule, lived, lived, def.stability);
    let clock = advance_holder(world, b, delta);
    remember(world, a, b, now, clock.elapsed, 0);
    events.push(Event::Reconciled {
        a: holder_name(a, data),
        b: holder_name(b, data),
        delta_a: lived,
        delta_b: delta,
        era_b: clock.era,
    });
}

/// The last contact `a` keeps with `b`, or the origin when they never met.
fn contact(world: &World, a: HolderId, b: HolderId) -> Contact {
    world.contacts.get(&(a, b)).copied().unwrap_or(Contact {
        other: b,
        self_elapsed: 0,
        other_elapsed: 0,
        self_shared: 0,
    })
}

/// Both sides remember the meeting.
fn remember(world: &mut World, a: HolderId, b: HolderId, a_now: i64, b_now: i64, a_shared: i64) {
    world.contacts.insert(
        (a, b),
        Contact {
            other: b,
            self_elapsed: a_now,
            other_elapsed: b_now,
            self_shared: a_shared,
        },
    );
    world.contacts.insert(
        (b, a),
        Contact {
            other: a,
            self_elapsed: b_now,
            other_elapsed: a_now,
            self_shared: 0,
        },
    );
}

/// `holder`'s clock after it advances by `minutes`, created at the origin on first contact.
fn advance_holder(world: &mut World, holder: HolderId, minutes: i64) -> omnis_core::Clock {
    let clock = world
        .clocks
        .entry(holder)
        .or_insert(omnis_core::Clock::new(EraId(0)));
    clock.elapsed = clock.elapsed.saturating_add(minutes);
    *clock
}

/// How far `b` catches up: the region's rule on the stream of the pair, never negative and at
/// most the cap. A rule that fails to evaluate (only a slot replaced at run time can) moves
/// nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "the rule's three inputs and the pair"
)]
fn catch_up(
    world: &mut World,
    data: &Data,
    a: HolderId,
    b: HolderId,
    rule: &str,
    lived: i64,
    shared: i64,
    stability: u16,
) -> i64 {
    let cap = data.rules.value("time_cap_minutes").unwrap_or(DEFAULT_CAP);
    let mut roller = Roller::take_named(world, StreamName::time(a, b));
    let inputs = [
        ("lived", Value::Int(lived.min(cap))),
        ("shared_time", Value::Int(shared.min(cap))),
        ("stability", Value::Int(i64::from(stability))),
    ];
    let value = data
        .rules
        .eval(rule, &inputs, &mut roller.rng, &roller.stream)
        .ok()
        .map(|o| o.value);
    roller.store(world);
    match value {
        Some(Value::Int(n)) => n.clamp(0, cap),
        _ => 0,
    }
}
