//! `cast.get` (M8 step 8): the spells each member could cast outside a fight, with what each
//! costs and, when the rules would refuse it, why. Each row is checked on its own copy of the
//! `cast` stream, never stored, so looking draws nothing and each row says what that cast would
//! meet if it were sent next (a pack's cost formula may roll).

use crate::combat::{Roller, cast};
use crate::command::Rejection;
use crate::world::World;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::Data;
use serde::{Deserialize, Serialize};

/// One spell a member could cast outside a fight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastView {
    /// The caster's slot in marching order, the `caster` that `Command::Cast` takes.
    pub caster: u8,
    /// The caster's identity.
    pub caster_id: CharacterId,
    /// The spell's row in the caster's list, the `spell` that `Command::Cast` takes.
    pub spell: u8,
    /// Spell id.
    pub id: String,
    /// Text key of the spell's name.
    pub name: String,
    /// Points it costs; 0 for a cantrip.
    pub cost: u32,
    /// Aimed at a member rather than at the caster.
    pub targets_members: bool,
    /// Why the rules would refuse it now, if they would.
    pub refusal: Option<Rejection>,
}

/// Every member's spells castable outside a fight, in marching order, then list order.
#[must_use]
pub fn cast_view(world: &World, data: &Data) -> Vec<CastView> {
    let mut rows = Vec::new();
    for (own, member) in world.party.members.iter().enumerate() {
        let Ok(caster) = u8::try_from(own) else {
            break;
        };
        for (row, id) in member.known_spells.iter().enumerate() {
            let Ok(spell) = u8::try_from(row) else {
                break;
            };
            let Some(def) = data.spells.get(id) else {
                continue;
            };
            if !def.effect.as_ref().is_some_and(|e| e.explore_castable()) {
                continue;
            }
            rows.push(CastView {
                caster,
                caster_id: member.id,
                spell,
                id: data.registry.spells.name(*id).unwrap_or("?").to_owned(),
                name: def.name.clone(),
                cost: def.point_cost(),
                targets_members: def.effect.as_ref().is_some_and(|e| e.targets_members()),
                refusal: cast::check(world, data, own, spell, false, &mut fresh(world)).err(),
            });
        }
    }
    rows
}

fn fresh(world: &World) -> omnis_core::Pcg32 {
    Roller::take_stream(world, "cast").rng
}
