//! `SenseSource`: what an item, skill, or spell reveals into the automap (PRD §7.2, D18).
//! Every source has a geometry, a fidelity ladder, a check, and a persistence; the sim rolls
//! one check per layer and records what it reaches as remotely seen.

use crate::error::DataError;
use crate::limits::MAX_SENSE_RANGE;
use crate::terms::Skill;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The shape a source reveals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Geometry {
    /// Along the facing line of sight, stopping at the first opaque tile or closed edge.
    Ray {
        /// Tiles ahead.
        range: u8,
    },
}

/// The highest knowledge layer a source can reach; layers below it come first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Fidelity {
    /// Terrain only.
    Terrain,
    /// Walls and doors.
    Structure,
    /// Objects and triggers (recorded as structure until objects exist).
    Objects,
    /// Monsters (recorded as structure until monster knowledge exists).
    Creatures,
}

impl Fidelity {
    /// 1..=4, the `layer` input of `sense.dc`.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Fidelity::Terrain => 1,
            Fidelity::Structure => 2,
            Fidelity::Objects => 3,
            Fidelity::Creatures => 4,
        }
    }
}

/// How long revealed knowledge lasts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Persistence {
    /// Stays on the automap until the party sees the tile itself.
    #[default]
    Permanent,
}

/// A remote-sensing source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SenseSource {
    /// What it reveals.
    pub geometry: Geometry,
    /// The highest layer it can reach.
    pub fidelity: Fidelity,
    /// The skill rolled once per layer against `sense.dc`; `None` reveals without a die.
    #[serde(default)]
    pub check: Option<Skill>,
    /// How long the knowledge lasts.
    #[serde(default)]
    pub persistence: Persistence,
    /// Party-clock minutes one use costs.
    #[serde(default = "one")]
    pub minutes: u32,
}

const fn one() -> u32 {
    1
}

impl SenseSource {
    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        let Geometry::Ray { range } = self.geometry;
        if range == 0 || range > MAX_SENSE_RANGE {
            errors.push(DataError::new(
                file,
                format!("sense ray range must be 1..={MAX_SENSE_RANGE}"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ron_io::{from_str, to_string};

    #[test]
    fn ranks_are_the_ladder_and_minutes_default_to_one() {
        assert_eq!(Fidelity::Terrain.rank(), 1);
        assert_eq!(Fidelity::Creatures.rank(), 4);
        assert!(Fidelity::Structure < Fidelity::Objects);
        let source: SenseSource = from_str(
            "(geometry: Ray(range: 16), fidelity: Structure, check: Some(Perception))",
            Path::new("memory"),
        )
        .unwrap();
        assert_eq!(source.minutes, 1);
        assert_eq!(source.persistence, Persistence::Permanent);
        let back: SenseSource =
            from_str(&to_string(&source).unwrap(), Path::new("memory")).unwrap();
        assert_eq!(back, source);
    }

    #[test]
    fn a_ray_is_bounded() {
        for (range, bad) in [(0, true), (1, false), (64, false), (65, true)] {
            let source = SenseSource {
                geometry: Geometry::Ray { range },
                fidelity: Fidelity::Terrain,
                check: None,
                persistence: Persistence::Permanent,
                minutes: 1,
            };
            let mut errors = Vec::new();
            source.validate(Path::new("x.ron"), &mut errors);
            assert_eq!(errors.len(), usize::from(bad), "range {range}");
        }
    }
}
