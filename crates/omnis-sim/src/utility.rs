//! Spells on the world rather than a creature. Mage hand is a hand (owner, 2026-11-14): it
//! works the door chosen, or the object chosen by the object's own interaction, so what the
//! object asks of a hand of flesh it asks of this one.

use crate::apply::toggle_door;
use crate::combat::cast::Target;
use crate::command::Rejection;
use crate::event::Event;
use crate::objects;
use crate::targets::{in_range, map};
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::Position;
use omnis_data::placement::ResolvedPlacement;
use omnis_data::{Data, Interaction, Targeting, Verb};

/// What a hand at a distance can do to an object.
pub(crate) const HAND: [Verb; 5] = [Verb::Open, Verb::Close, Verb::Push, Verb::Pull, Verb::Use];

/// What a hand that passed every check will do.
pub(crate) enum Hand<'d> {
    /// Open or close the door on the edge this position faces.
    Door(Position),
    /// Work an object by interaction `index` of its kind.
    Object {
        /// The object.
        placement: &'d ResolvedPlacement,
        /// The interaction's index in the kind.
        index: u8,
        /// The interaction.
        interaction: &'d Interaction,
    },
}

/// Every check of a hand sent at `target` by the member in slot `own`: a door or an object
/// the party knows of, within `targeting`, and for an object an interaction it allows now.
pub(crate) fn check<'d>(
    world: &World,
    data: &'d Data,
    own: usize,
    targeting: Targeting,
    target: &Target,
) -> Result<Hand<'d>, Rejection> {
    if !matches!(target, Target::Door { .. } | Target::Object { .. }) {
        return Err(Rejection::WrongTarget);
    }
    in_range(world, data, None, own, targeting, target)?;
    match target {
        Target::Door { x, y, facing } => Ok(Hand::Door(Position {
            facing: *facing,
            ..world.position.at(*x, *y)
        })),
        Target::Object { object } => {
            let gone = || Rejection::NoSuchObject {
                object: object.clone(),
            };
            let placement = map::placed(world, data, object).ok_or_else(gone)?;
            let (index, interaction) = objects::pick(world, data, placement, &HAND)?;
            Ok(Hand::Object {
                placement,
                index,
                interaction,
            })
        }
        _ => Err(Rejection::WrongTarget),
    }
}

/// Do what a checked hand was sent to do.
pub(crate) fn carry_out(
    world: &mut World,
    data: &Data,
    hand: &Hand<'_>,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    match hand {
        Hand::Door(at) => {
            toggle_door(world, data, *at, events);
            Ok(())
        }
        Hand::Object {
            placement,
            index,
            interaction,
        } => objects::perform(world, data, placement, *index, interaction, events),
    }
}
