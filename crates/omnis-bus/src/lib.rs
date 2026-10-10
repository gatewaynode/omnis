//! The Omnis signal bus (ARCHITECTURE.md §4.8, A16): one system raises a signal on a topic and
//! every subscriber of that topic is called, in the order it subscribed. Subscriptions are data
//! (values the host matches to a handler), so a save holds them and a replay calls them in the
//! same order; nothing here is a closure. Delivery is synchronous: [`drain`] runs to the end
//! before the raising code goes on, and a signal raised during delivery is queued one level
//! deeper. Past [`MAX_DEPTH`] or [`MAX_SIGNALS`] the rest of the queue is dropped and counted,
//! never a panic.
//!
//! This crate is the mechanism only. The topics, subscribers and signals are the user's
//! vocabulary (`omnis-sim/src/bus.rs`), so a new topic never changes this crate.
//!
//! No external crate was taken (survey 2026-10-05): the candidates were thread or async
//! channels, kept closures that cannot be saved, or were abandoned.
//!
//! Simulation crate rules (CLAUDE.md, ARCHITECTURE.md §11): `no_std`, integers only, ordered
//! collections only, no Bevy, no wall clock, no threads, no I/O.
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// The deepest a raised signal may be: a signal raised by a delivery four deep is still
/// delivered, one raised five deep is dropped.
pub const MAX_DEPTH: u8 = 4;

/// The most signals one drain delivers.
pub const MAX_SIGNALS: u32 = 64;

/// A signal: what happened, and the topic it is raised on.
pub trait Signal {
    /// What a signal can be about.
    type Topic: Ord + Clone;
    /// The topic this signal is raised on.
    fn topic(&self) -> Self::Topic;
}

/// The subscriptions, by topic, in call order. Saved with the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize, S: Serialize",
    deserialize = "T: Ord + Deserialize<'de>, S: Deserialize<'de>"
))]
pub struct Bus<T, S> {
    subs: BTreeMap<T, Vec<S>>,
}

impl<T, S> Default for Bus<T, S> {
    fn default() -> Self {
        Bus {
            subs: BTreeMap::new(),
        }
    }
}

impl<T: Ord, S: Copy + Eq> Bus<T, S> {
    /// Add `subscriber` to `topic` after those already there; a second subscription is ignored.
    pub fn subscribe(&mut self, topic: T, subscriber: S) {
        let list = self.subs.entry(topic).or_default();
        if !list.contains(&subscriber) {
            list.push(subscriber);
        }
    }

    /// Remove `subscriber` from `topic`; the others keep their order.
    pub fn unsubscribe(&mut self, topic: T, subscriber: S) {
        if let Some(list) = self.subs.get_mut(&topic) {
            list.retain(|s| *s != subscriber);
            if list.is_empty() {
                self.subs.remove(&topic);
            }
        }
    }

    /// The subscribers of `topic`, in call order.
    #[must_use]
    pub fn subscribers(&self, topic: T) -> &[S] {
        self.subs.get(&topic).map_or(&[], Vec::as_slice)
    }
}

/// What a drain needs from its caller: the subscriptions, and a handler for each subscriber.
pub trait Host {
    /// The signals it carries.
    type Signal: Signal;
    /// Who listens.
    type Subscriber: Copy + Eq;
    /// The subscriptions.
    fn bus(&self) -> &Bus<<Self::Signal as Signal>::Topic, Self::Subscriber>;
    /// Call `to` with `signal`; anything it raises goes in `raised`, in order.
    fn deliver(
        &mut self,
        to: Self::Subscriber,
        signal: &Self::Signal,
        raised: &mut Vec<Self::Signal>,
    );
}

/// Deliver `signals` and everything they raise, first in first out. Returns how many signals
/// were dropped at a limit (0 when every one was delivered).
pub fn drain<H: Host>(host: &mut H, signals: Vec<H::Signal>) -> u32 {
    let mut queue: VecDeque<(H::Signal, u8)> = signals.into_iter().map(|s| (s, 0)).collect();
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
mod tests;
