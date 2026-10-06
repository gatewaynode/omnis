//! The mechanism on a vocabulary of its own: places numbered from 0, a fight, two subscribers.

use super::*;
use alloc::vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum Topic {
    Place(u32),
    Fight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Sub {
    First,
    Second,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sig {
    Entered(u32),
    Cue,
}

impl Signal for Sig {
    type Topic = Topic;
    fn topic(&self) -> Topic {
        match self {
            Sig::Entered(place) => Topic::Place(*place),
            Sig::Cue => Topic::Fight,
        }
    }
}

/// A host that records each call and raises `fan` signals per delivery to `First` on the next
/// place, up to place `stop`.
struct Recorder {
    bus: Bus<Topic, Sub>,
    calls: Vec<(Sub, u32)>,
    fan: u32,
    stop: u32,
}

impl Host for Recorder {
    type Signal = Sig;
    type Subscriber = Sub;
    fn bus(&self) -> &Bus<Topic, Sub> {
        &self.bus
    }
    fn deliver(&mut self, to: Sub, signal: &Sig, raised: &mut Vec<Sig>) {
        let Sig::Entered(place) = *signal else {
            return;
        };
        self.calls.push((to, place));
        if place < self.stop && to == Sub::First {
            for _ in 0..self.fan {
                raised.push(Sig::Entered(place + 1));
            }
        }
    }
}

fn recorder(fan: u32, stop: u32) -> Recorder {
    let mut bus = Bus::default();
    for place in 0..=8 {
        bus.subscribe(Topic::Place(place), Sub::First);
        bus.subscribe(Topic::Place(place), Sub::Second);
    }
    Recorder {
        bus,
        calls: Vec::new(),
        fan,
        stop,
    }
}

#[test]
fn subscribers_are_called_in_order_and_raises_after_the_signal_that_raised_them() {
    let mut host = recorder(1, 2);
    host.bus.subscribe(Topic::Place(0), Sub::First);
    assert_eq!(drain(&mut host, vec![Sig::Entered(0), Sig::Entered(5)]), 0);
    let (f, s) = (Sub::First, Sub::Second);
    assert_eq!(
        host.calls,
        [
            (f, 0),
            (s, 0),
            (f, 5),
            (s, 5),
            (f, 1),
            (s, 1),
            (f, 2),
            (s, 2)
        ],
        "first in first out; each topic's subscribers in subscription order; a repeat is ignored"
    );
    host.bus.unsubscribe(Topic::Place(5), f);
    host.calls.clear();
    drain(&mut host, vec![Sig::Entered(5), Sig::Entered(7)]);
    assert_eq!(host.calls, [(s, 5), (f, 7), (s, 7)]);
    assert!(host.bus.subscribers(Topic::Fight).is_empty());
}

#[test]
fn the_order_survives_a_save() {
    let mut bus = Bus::default();
    let topic = Topic::Place(3);
    bus.subscribe(topic, Sub::Second);
    bus.subscribe(topic, Sub::First);
    bus.subscribe(Topic::Fight, Sub::Second);
    let text = ron::to_string(&bus).unwrap();
    let back: Bus<Topic, Sub> = ron::from_str(&text).unwrap();
    assert_eq!(back, bus);
    assert_eq!(back.subscribers(topic), [Sub::Second, Sub::First]);
}

#[test]
fn a_chain_deeper_than_the_cap_is_dropped_and_counted() {
    let mut host = recorder(1, 8);
    assert_eq!(
        drain(&mut host, vec![Sig::Entered(0)]),
        1,
        "place 5 is five deep"
    );
    let deepest = host.calls.iter().map(|(_, place)| *place).max();
    assert_eq!(deepest, Some(u32::from(MAX_DEPTH)));
}

#[test]
fn a_fan_past_the_budget_is_dropped_and_counted() {
    let mut host = recorder(3, 8);
    let dropped = drain(&mut host, vec![Sig::Entered(0)]);
    let delivered = host.calls.len() / 2;
    assert_eq!(delivered, MAX_SIGNALS as usize);
    // The first signal and three from each of the 64 delivered; the rest are dropped.
    assert_eq!(dropped, 1 + 3 * MAX_SIGNALS - MAX_SIGNALS);
}

/// A host whose subscriber answers every cue with another: a runaway the depth cap stops.
struct Echo {
    bus: Bus<Topic, Sub>,
    calls: Vec<Sub>,
}

impl Host for Echo {
    type Signal = Sig;
    type Subscriber = Sub;
    fn bus(&self) -> &Bus<Topic, Sub> {
        &self.bus
    }
    fn deliver(&mut self, to: Sub, signal: &Sig, raised: &mut Vec<Sig>) {
        self.calls.push(to);
        if *signal == Sig::Cue {
            raised.push(Sig::Cue);
        }
    }
}

#[test]
fn a_cue_raised_by_every_answer_stops_at_the_depth_cap() {
    let mut bus = Bus::default();
    bus.subscribe(Topic::Fight, Sub::Second);
    bus.subscribe(Topic::Place(0), Sub::First);
    let mut host = Echo {
        bus,
        calls: Vec::new(),
    };
    assert_eq!(
        drain(&mut host, vec![Sig::Cue]),
        1,
        "the sixth cue is dropped"
    );
    assert_eq!(
        host.calls,
        [Sub::Second; MAX_DEPTH as usize + 1],
        "the cue and four nested answers, and only the fight's subscribers"
    );
}
