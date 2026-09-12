//! Deterministic randomness (ARCHITECTURE.md §11, A14): our own PCG32, FNV-1a 64, and
//! splitmix64, plus dice that leave a `RollTrace` for every draw.
//!
//! One world seed. Every consumer draws from a named stream whose initial state is a pure
//! function of the seed and the name, so adding a stream never perturbs another.

use crate::{Error, StreamName};
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

/// FNV-1a, 64-bit. Used for stream derivation and text seeds; never for security.
#[must_use]
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }
    hash
}

/// The splitmix64 output function: one well-mixed 64-bit value from another.
#[must_use]
pub const fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A PCG32 generator (XSH RR output, 64-bit LCG state), with a draw counter so a trace can
/// name the exact draw and a loaded save continues where it stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
    draws: u64,
}

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

impl Pcg32 {
    /// The stream for `name` under `world_seed`:
    /// `state = splitmix64(seed ^ fnv1a64(name))`, `increment = splitmix64(fnv1a64(name)) | 1`.
    #[must_use]
    pub const fn for_stream(world_seed: u64, name: &StreamName) -> Pcg32 {
        let name_hash = fnv1a64(name.0.as_bytes());
        Pcg32 {
            state: splitmix64(world_seed ^ name_hash),
            inc: splitmix64(name_hash) | 1,
            draws: 0,
        }
    }

    /// The reference `pcg32_srandom_r` seeding, kept so the generator can be checked against
    /// the published PCG test vectors.
    #[must_use]
    pub const fn seeded(initstate: u64, initseq: u64) -> Pcg32 {
        let mut rng = Pcg32 {
            state: 0,
            inc: (initseq << 1) | 1,
            draws: 0,
        };
        rng.step();
        rng.state = rng.state.wrapping_add(initstate);
        rng.step();
        rng
    }

    /// Rebuild a generator from persisted parts.
    #[must_use]
    pub const fn from_parts(state: u64, inc: u64, draws: u64) -> Pcg32 {
        Pcg32 { state, inc, draws }
    }

    /// The LCG state.
    #[must_use]
    pub const fn state(&self) -> u64 {
        self.state
    }

    /// The stream increment (always odd).
    #[must_use]
    pub const fn increment(&self) -> u64 {
        self.inc
    }

    /// How many values have been drawn.
    #[must_use]
    pub const fn draws(&self) -> u64 {
        self.draws
    }

    const fn step(&mut self) {
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(self.inc);
    }

    /// The next 32-bit value.
    pub const fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.step();
        self.draws = self.draws.wrapping_add(1);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// A value in `0..bound`, unbiased by rejection (the reference `pcg32_boundedrand_r`).
    /// A zero bound draws nothing and returns zero.
    pub const fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % bound;
            }
        }
    }
}

/// One die's contribution to a roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DieRoll {
    /// The stream's draw count when this die was rolled (zero-based index of the first draw).
    pub index: u64,
    /// The generator's raw 32-bit output that produced the face.
    pub raw: u32,
    /// The face, `1..=sides`.
    pub value: u32,
}

/// A dice expression such as `2d6+3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Dice {
    /// How many dice.
    pub count: u16,
    /// Faces per die.
    pub sides: u16,
    /// Added to the sum.
    pub modifier: i32,
}

impl Dice {
    /// `count` dice of `sides` faces, no modifier.
    #[must_use]
    pub const fn new(count: u16, sides: u16) -> Dice {
        Dice {
            count,
            sides,
            modifier: 0,
        }
    }

    /// The same dice with a modifier.
    #[must_use]
    pub const fn plus(self, modifier: i32) -> Dice {
        Dice { modifier, ..self }
    }

    /// The smallest possible total.
    #[must_use]
    pub const fn min(self) -> i32 {
        (self.count as i32).saturating_add(self.modifier)
    }

    /// The largest possible total.
    #[must_use]
    pub const fn max(self) -> i32 {
        (self.count as i32 * self.sides as i32).saturating_add(self.modifier)
    }

    /// Roll on `rng`, recording every die. Fails for zero dice or zero sides without drawing.
    pub fn roll(self, rng: &mut Pcg32, stream: &StreamName) -> Result<RollTrace, Error> {
        if self.count == 0 || self.sides == 0 {
            return Err(Error::InvalidDice);
        }
        let mut rolls = Vec::with_capacity(usize::from(self.count));
        let mut total: i32 = self.modifier;
        for _ in 0..self.count {
            let index = rng.draws();
            let (raw, value) = roll_die(rng, u32::from(self.sides));
            rolls.push(DieRoll { index, raw, value });
            total = total.saturating_add(value as i32);
        }
        Ok(RollTrace {
            stream: stream.clone(),
            dice: self,
            rolls,
            total,
        })
    }
}

/// One unbiased die, returning the raw output that was accepted alongside the face.
const fn roll_die(rng: &mut Pcg32, sides: u32) -> (u32, u32) {
    let threshold = sides.wrapping_neg() % sides;
    loop {
        let r = rng.next_u32();
        if r >= threshold {
            return (r, r % sides + 1);
        }
    }
}

impl fmt::Display for Dice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}d{}", self.count, self.sides)?;
        match self.modifier {
            0 => Ok(()),
            m if m > 0 => write!(f, "+{m}"),
            m => write!(f, "{m}"),
        }
    }
}

/// The record of a roll: which stream, which draws, what came out. Every dice roll in the
/// simulation carries one so clients can show the math and a replay divergence names the
/// first stream and draw index that differed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RollTrace {
    /// The stream drawn from.
    pub stream: StreamName,
    /// The expression rolled.
    pub dice: Dice,
    /// One entry per die, in order.
    pub rolls: Vec<DieRoll>,
    /// The sum of faces plus the modifier.
    pub total: i32,
}

impl fmt::Display for RollTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} on {}: [", self.dice, self.stream)?;
        for (i, roll) in self.rolls.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{}", roll.value)?;
        }
        write!(f, "] = {}", self.total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn stream_names_are_the_only_input() {
        let a = Pcg32::for_stream(7, &StreamName::from("party"));
        let b = Pcg32::for_stream(7, &StreamName::from("party"));
        let c = Pcg32::for_stream(7, &StreamName::from("combat"));
        let d = Pcg32::for_stream(8, &StreamName::from("party"));
        assert_eq!(a, b);
        assert_ne!(a.state(), c.state());
        assert_ne!(a.state(), d.state());
        assert_eq!(
            a.increment(),
            d.increment(),
            "increment depends on the name only"
        );
        assert_eq!(a.increment() & 1, 1);
    }

    #[test]
    fn draw_counter_and_parts_round_trip() {
        let mut rng = Pcg32::for_stream(1, &StreamName::from("party"));
        let first = rng.next_u32();
        let copy = Pcg32::from_parts(rng.state(), rng.increment(), rng.draws());
        let mut resumed = copy;
        assert_eq!(rng.draws(), 1);
        assert_eq!(resumed.next_u32(), rng.next_u32());
        assert_ne!(first, rng.next_u32());
    }

    #[test]
    fn below_stays_in_range_and_zero_bound_draws_nothing() {
        let mut rng = Pcg32::seeded(1, 1);
        for bound in [1, 2, 3, 6, 20, 1000, u32::MAX] {
            for _ in 0..200 {
                assert!(rng.below(bound) < bound);
            }
        }
        let draws = rng.draws();
        assert_eq!(rng.below(0), 0);
        assert_eq!(rng.draws(), draws);
    }

    #[test]
    fn dice_trace_records_each_die() {
        let stream = StreamName::from("combat");
        let mut rng = Pcg32::for_stream(3, &stream);
        let trace = Dice::new(3, 6).plus(2).roll(&mut rng, &stream).unwrap();
        assert_eq!(trace.rolls.len(), 3);
        assert_eq!(trace.rolls[0].index, 0);
        let faces: i32 = trace.rolls.iter().map(|r| r.value as i32).sum();
        assert_eq!(trace.total, faces + 2);
        assert!(trace.rolls.iter().all(|r| (1..=6).contains(&r.value)));
        assert!(trace.rolls.iter().all(|r| r.raw % 6 + 1 == r.value));
        assert_eq!(rng.draws(), u64::try_from(trace.rolls.len()).unwrap());
        assert!(format!("{trace}").starts_with("3d6+2 on combat: ["));
        assert_eq!(format!("{}", Dice::new(1, 20).plus(-1)), "1d20-1");
    }

    #[test]
    fn invalid_dice_are_refused_without_drawing() {
        let stream = StreamName::from("combat");
        let mut rng = Pcg32::for_stream(3, &stream);
        assert_eq!(
            Dice::new(0, 6).roll(&mut rng, &stream),
            Err(Error::InvalidDice)
        );
        assert_eq!(
            Dice::new(1, 0).roll(&mut rng, &stream),
            Err(Error::InvalidDice)
        );
        assert_eq!(rng.draws(), 0);
        assert_eq!(Dice::new(2, 6).plus(1).min(), 3);
        assert_eq!(Dice::new(2, 6).plus(1).max(), 13);
    }
}
