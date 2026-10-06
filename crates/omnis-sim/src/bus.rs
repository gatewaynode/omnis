//! The signal bus (ARCHITECTURE.md §4.8): one system raises a signal on a topic and every
//! subscriber of that topic is called, in the order it subscribed. Subscriptions are data (an
//! enum each, matched to a handler), so a save holds them and a replay calls them in the same
//! order; nothing here is a closure. Delivery is synchronous: `drain` runs to the end before the
//! raising code goes on, and a signal raised during delivery is queued one level deeper. Past
//! [`MAX_DEPTH`] or [`MAX_SIGNALS`] the rest of the queue is dropped and counted, never a panic.
//!
//! No external crate was taken (survey 2026-10-05): the candidates were thread or async
//! channels, kept closures that cannot be saved, or were abandoned.

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use omnis_core::RegionId;
use serde::{Deserialize, Serialize};

/// The deepest a raised signal may be: a signal raised by a delivery four deep is still
/// delivered, one raised five deep is dropped.
pub const MAX_DEPTH: u8 = 4;

/// The most signals one drain delivers.
pub const MAX_SIGNALS: u32 = 64;

/// What a signal is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Topic {
    /// A region: the party entered it.
    Region(RegionId),
    /// The fight under way.
    Battle,
}

/// Who listens. Each is a handler the simulation matches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Subscriber {
    /// Subjective time: reconcile the region with the party (§4.4).
    Reconcile,
    /// Declared reactions answer a combat trigger (§4.7).
    Reactions,
}

/// What happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// The party entered `region`, from `from` (none at the start of a game).
    Entered {
        /// The region entered.
        region: RegionId,
        /// The region left.
        from: Option<RegionId>,
    },
}

impl Signal {
    /// The topic it is raised on.
    #[must_use]
    pub const fn topic(&self) -> Topic {
        match self {
            Signal::Entered { region, .. } => Topic::Region(*region),
        }
    }
}

/// The subscriptions, by topic, in call order. Saved with the world.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bus {
    subs: BTreeMap<Topic, Vec<Subscriber>>,
}

impl Bus {
    /// Add `subscriber` to `topic` after those already there; a second subscription is ignored.
    pub fn subscribe(&mut self, topic: Topic, subscriber: Subscriber) {
        let list = self.subs.entry(topic).or_default();
        if !list.contains(&subscriber) {
            list.push(subscriber);
        }
    }

    /// Remove `subscriber` from `topic`; the others keep their order.
    pub fn unsubscribe(&mut self, topic: Topic, subscriber: Subscriber) {
        if let Some(list) = self.subs.get_mut(&topic) {
            list.retain(|s| *s != subscriber);
            if list.is_empty() {
                self.subs.remove(&topic);
            }
        }
    }

    /// The subscribers of `topic`, in call order.
    #[must_use]
    pub fn subscribers(&self, topic: Topic) -> &[Subscriber] {
        self.subs.get(&topic).map_or(&[], Vec::as_slice)
    }
}

/// What a drain needs from its caller: the subscriptions, and a handler for each subscriber.
pub trait Host {
    /// The subscriptions.
    fn bus(&self) -> &Bus;
    /// Call `to` with `signal`; anything it raises goes in `raised`, in order.
    fn deliver(&mut self, to: Subscriber, signal: &Signal, raised: &mut Vec<Signal>);
}

/// Deliver `signals` and everything they raise, first in first out. Returns how many signals
/// were dropped at a limit (0 when every one was delivered).
pub fn drain<H: Host>(host: &mut H, signals: Vec<Signal>) -> u32 {
    let mut queue: VecDeque<(Signal, u8)> = signals.into_iter().map(|s| (s, 0)).collect();
    let mut delivered = 0_u32;
    while let Some((signal, depth)) = queue.pop_front() {
        if depth > MAX_DEPTH || delivered >= MAX_SIGNALS {
            let rest = u32::try_from(queue.len()).unwrap_or(u32::MAX);
            return rest.saturating_add(1);
        }
        delivered += 1;
        let subscribers = host.bus().subscribers(signal.topic()).to_vec();
        for to in subscribers {
            let mut raised = Vec::new();
            host.deliver(to, &signal, &mut raised);
            queue.extend(raised.into_iter().map(|s| (s, depth + 1)));
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// A host that records each call and raises `fan` signals per delivery on the next region,
    /// up to region `stop`.
    struct Recorder {
        bus: Bus,
        calls: Vec<(Subscriber, u32)>,
        fan: u32,
        stop: u32,
    }

    impl Host for Recorder {
        fn bus(&self) -> &Bus {
            &self.bus
        }
        fn deliver(&mut self, to: Subscriber, signal: &Signal, raised: &mut Vec<Signal>) {
            let Signal::Entered { region, .. } = signal;
            self.calls.push((to, region.0));
            if region.0 < self.stop && to == Subscriber::Reconcile {
                for _ in 0..self.fan {
                    raised.push(entered(region.0 + 1));
                }
            }
        }
    }

    fn entered(region: u32) -> Signal {
        Signal::Entered {
            region: RegionId(region),
            from: None,
        }
    }

    fn recorder(fan: u32, stop: u32) -> Recorder {
        let mut bus = Bus::default();
        for r in 0..=8 {
            bus.subscribe(Topic::Region(RegionId(r)), Subscriber::Reconcile);
            bus.subscribe(Topic::Region(RegionId(r)), Subscriber::Reactions);
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
        host.bus
            .subscribe(Topic::Region(RegionId(0)), Subscriber::Reconcile);
        assert_eq!(drain(&mut host, vec![entered(0), entered(5)]), 0);
        let (c, r) = (Subscriber::Reconcile, Subscriber::Reactions);
        assert_eq!(
            host.calls,
            [
                (c, 0),
                (r, 0),
                (c, 5),
                (r, 5),
                (c, 1),
                (r, 1),
                (c, 2),
                (r, 2)
            ],
            "first in first out; each topic's subscribers in subscription order; a repeat is ignored"
        );
        host.bus.unsubscribe(Topic::Region(RegionId(5)), c);
        host.calls.clear();
        drain(&mut host, vec![entered(5), entered(7)]);
        assert_eq!(host.calls, [(r, 5), (c, 7), (r, 7)]);
        assert!(host.bus.subscribers(Topic::Battle).is_empty());
    }

    #[test]
    fn the_order_survives_a_save() {
        let mut bus = Bus::default();
        let topic = Topic::Region(RegionId(3));
        bus.subscribe(topic, Subscriber::Reactions);
        bus.subscribe(topic, Subscriber::Reconcile);
        bus.subscribe(Topic::Battle, Subscriber::Reactions);
        let text = omnis_data::ron_io::to_string(&bus).unwrap();
        let back: Bus = omnis_data::ron_io::parse(&text).unwrap();
        assert_eq!(back, bus);
        assert_eq!(
            back.subscribers(topic),
            [Subscriber::Reactions, Subscriber::Reconcile]
        );
    }

    #[test]
    fn a_chain_deeper_than_the_cap_is_dropped_and_counted() {
        let mut host = recorder(1, 8);
        assert_eq!(
            drain(&mut host, vec![entered(0)]),
            1,
            "region 5 is five deep"
        );
        let deepest = host.calls.iter().map(|(_, r)| *r).max();
        assert_eq!(deepest, Some(u32::from(MAX_DEPTH)));
    }

    #[test]
    fn a_fan_past_the_budget_is_dropped_and_counted() {
        let mut host = recorder(3, 8);
        let dropped = drain(&mut host, vec![entered(0)]);
        let delivered = host.calls.len() / 2;
        assert_eq!(delivered, MAX_SIGNALS as usize);
        // The first signal and three from each of the 64 delivered; the rest are dropped.
        assert_eq!(dropped, 1 + 3 * MAX_SIGNALS - MAX_SIGNALS);
    }
}
