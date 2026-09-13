//! The fight: its state between commands. The turn loop arrives with M4's combat step.

pub mod state;

pub use state::{CombatState, Initiative};
