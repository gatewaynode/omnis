//! A rest as a client sees it before asking (the camp panel, M7 step 8b): each member's hit
//! dice and how many a short rest may spend, and whether a long rest would be refused, and why.
//! Every answer comes from the checks `rest::apply` makes, so the view and the command cannot
//! disagree; nothing is rolled and nothing is stored.

use crate::combat::state::is_dead;
use crate::command::Rejection;
use crate::rest;
use crate::world::{Mode, World};
use alloc::vec::Vec;
use omnis_data::Data;
use serde::{Deserialize, Serialize};

/// The rests on offer where the party stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestView {
    /// Why no rest may begin here (inside a service, in a fight), if so.
    pub refusal: Option<Rejection>,
    /// Every member, in party order.
    pub members: Vec<CampMember>,
    /// Food the long rest would eat.
    pub long_food: u32,
    /// Food in the stores.
    pub food: u32,
    /// Why the long rest would be refused, if it would.
    pub long: Option<Rejection>,
}

/// One member at camp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampMember {
    /// Hit points now.
    pub hp: i32,
    /// Hit point maximum.
    pub hp_max: i32,
    /// Hit dice not yet spent.
    pub dice_left: u8,
    /// Hit dice in all (one per level).
    pub dice: u8,
    /// The hit die's sides.
    pub die: u8,
    /// How many a short rest may spend now: none when dead or at full hit points.
    pub spendable: u8,
}

/// The rests where the party stands.
#[must_use]
pub fn rest_view(world: &World, data: &Data) -> RestView {
    let refusal = (!matches!(world.mode, Mode::Explore)).then_some(Rejection::WrongMode);
    let members = world
        .party
        .members
        .iter()
        .map(|member| {
            let dice_left = member.level.saturating_sub(member.hit_dice_spent);
            let hurt = member.hp < member.hp_max && !is_dead(member, data);
            CampMember {
                hp: member.hp,
                hp_max: member.hp_max,
                dice_left,
                dice: member.level,
                die: rest::hit_die(data, member),
                spendable: if hurt { dice_left } else { 0 },
            }
        })
        .collect();
    let long = refusal.clone().or_else(|| {
        rest::too_soon(world, data, rest::long_rest_minutes(data))
            .and_then(|()| rest::food_needed(world, data).map(|_| ()))
            .err()
    });
    RestView {
        refusal,
        members,
        long_food: rest::food_need(world, data),
        food: world.party.food,
        long,
    }
}
