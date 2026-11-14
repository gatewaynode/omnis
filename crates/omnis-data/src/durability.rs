//! The fight's scale and what a thing is made to withstand (owner, 2026-11-14;
//! `tasks/plans/combat-in-feet.md`). A fight is counted in feet on the SRD's five-foot square; an
//! object or a door an author makes durable has the SRD's armor class by material and hit points
//! by size.

use crate::object::Material;
use crate::terms::Size;
use serde::{Deserialize, Serialize};

/// The side of the SRD's square, in feet.
pub const SQUARE_FEET: u16 = 5;
/// The normal and long range of a ranged weapon or attack that writes none.
pub const DEFAULT_RANGE: (u16, u16) = (80, 320);
/// The longest reach a weapon or attack may declare, in feet.
pub const MAX_REACH_FEET: u8 = 30;
/// The longest range a weapon or attack may declare, in feet.
pub const MAX_RANGE_FEET: u16 = 2400;
/// The most of one type a five-foot square may be said to hold.
pub const MAX_PER_SQUARE: u8 = 16;
/// The largest tile a map may declare, in feet (ten miles).
pub const MAX_TILE_FEET: u32 = 52_800;
/// The most hit points a thing may be given.
pub const MAX_THING_HIT_POINTS: u16 = 1000;

impl Size {
    /// How many of this size fit a five-foot square when the type does not say: four Tiny
    /// ones (the SRD's space of two and a half feet), one of anything else.
    #[must_use]
    pub const fn per_square(self) -> u8 {
        match self {
            Size::Tiny => 4,
            _ => 1,
        }
    }

    /// How many squares one of this size takes (the SRD's space): one up to Medium, then a
    /// square of side two, three and four.
    #[must_use]
    pub const fn space(self) -> u8 {
        match self {
            Size::Tiny | Size::Small | Size::Medium => 1,
            Size::Large => 4,
            Size::Huge => 9,
            Size::Gargantuan => 16,
        }
    }

    /// The side of that space in squares: how much of a line one of this size fills.
    #[must_use]
    pub const fn front(self) -> u8 {
        match self {
            Size::Tiny | Size::Small | Size::Medium => 1,
            Size::Large => 2,
            Size::Huge => 3,
            Size::Gargantuan => 4,
        }
    }
}

impl Material {
    /// The SRD's armor class for a thing of this material; `None` for a liquid, which has to
    /// write its own.
    #[must_use]
    pub const fn armor_class(self) -> Option<u8> {
        match self {
            Material::Cloth | Material::Paper | Material::Plant => Some(11),
            Material::Glass => Some(13),
            Material::Wood | Material::Bone => Some(15),
            Material::Stone | Material::Earth => Some(17),
            Material::Metal => Some(19),
            Material::Liquid => None,
        }
    }
}

/// The SRD's hit points for a thing of a size, fragile or resilient; `None` above Large, where
/// the SRD counts sections and the author has to write the number.
#[must_use]
pub const fn hit_points_for(size: Size, fragile: bool) -> Option<u16> {
    let (weak, strong) = match size {
        Size::Tiny => (2, 5),
        Size::Small => (3, 10),
        Size::Medium => (4, 18),
        Size::Large => (5, 27),
        Size::Huge | Size::Gargantuan => return None,
    };
    Some(if fragile { weak } else { strong })
}

/// That a thing can be damaged, and how well it stands it. Left out of a kind, the thing
/// cannot be damaged at all.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Durability {
    /// The SRD's fragile column of hit points instead of the resilient one.
    pub fragile: bool,
    /// Hit points, where the size's are not wanted.
    pub hit_points: Option<u16>,
    /// Armor class, where the material's is not wanted.
    pub armor_class: Option<u8>,
    /// Damage from one blow under this does nothing (the SRD's damage threshold).
    pub threshold: u8,
    /// The state an object kind shows when broken; without one it lies as a heap.
    pub broken_state: Option<String>,
}

impl Durability {
    /// The armor class of a thing of `material`.
    #[must_use]
    pub fn armor_class(&self, material: Option<Material>) -> Option<u8> {
        self.armor_class
            .or_else(|| material.and_then(Material::armor_class))
    }

    /// The hit points of a thing of `size`.
    #[must_use]
    pub fn hit_points(&self, size: Option<Size>) -> Option<u16> {
        self.hit_points
            .or_else(|| size.and_then(|s| hit_points_for(s, self.fragile)))
    }

    /// What is wrong with it for a thing of this material and size, in the author's words.
    pub fn problems(&self, material: Option<Material>, size: Option<Size>) -> Vec<String> {
        let mut found = Vec::new();
        match self.armor_class(material) {
            None => found.push(
                "a durable thing needs a material that has an armor class, or its own armor_class"
                    .to_owned(),
            ),
            Some(ac) if ac == 0 || ac > 30 => found.push("armor_class must be 1..=30".to_owned()),
            Some(_) => {}
        }
        match self.hit_points(size) {
            None => found
                .push("a durable thing needs a size up to Large, or its own hit_points".to_owned()),
            Some(hp) if hp == 0 || hp > MAX_THING_HIT_POINTS => {
                found.push(format!("hit_points must be 1..={MAX_THING_HIT_POINTS}"));
            }
            Some(_) => {}
        }
        found
    }
}

/// What a map's doors are made of. Doors are wooden, Medium and resilient unless the map says
/// otherwise; `durability: None` makes them unbreakable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DoorMake {
    /// What the doors are made of.
    pub material: Material,
    /// How big they are.
    pub size: Size,
    /// How they stand damage; `None` and they cannot be damaged.
    pub durability: Option<Durability>,
}

impl Default for DoorMake {
    fn default() -> Self {
        DoorMake {
            material: Material::Wood,
            size: Size::Medium,
            durability: Some(Durability::default()),
        }
    }
}

impl DoorMake {
    /// Whether the map wrote nothing of its own.
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == DoorMake::default()
    }

    /// Armor class and hit points, when the doors can be damaged.
    #[must_use]
    pub fn numbers(&self) -> Option<(u8, u16)> {
        let d = self.durability.as_ref()?;
        Some((
            d.armor_class(Some(self.material))?,
            d.hit_points(Some(self.size))?,
        ))
    }
}

/// What an item is to someone repairing a thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Craft {
    /// A tool for things of these materials; it is kept.
    Tool(Vec<Material>),
    /// Supplies for things of these materials; they are used up.
    Supply(Vec<Material>),
}

impl Craft {
    /// The materials it serves.
    #[must_use]
    pub fn materials(&self) -> &[Material] {
        match self {
            Craft::Tool(m) | Craft::Supply(m) => m,
        }
    }
}

/// What is wrong with a reach and a range written on a weapon or an attack.
pub(crate) fn span_problems(reach: u8, range: Option<(u16, u16)>, ranged: bool) -> Vec<String> {
    let mut found = Vec::new();
    if reach == 0 || reach > MAX_REACH_FEET || u16::from(reach) % SQUARE_FEET != 0 {
        found.push(format!(
            "reach must be a multiple of {SQUARE_FEET} feet, {SQUARE_FEET}..={MAX_REACH_FEET}"
        ));
    }
    match range {
        Some(_) if !ranged => found.push("a range is for a ranged weapon or attack".to_owned()),
        Some((normal, long)) if normal == 0 || normal > long || long > MAX_RANGE_FEET => {
            found.push(format!(
                "range is (normal, long) in feet, 1..={MAX_RANGE_FEET}, normal no more than long"
            ));
        }
        _ => {}
    }
    found
}

/// The reach a weapon or attack has when its file writes none.
pub(crate) const fn melee_reach() -> u8 {
    5
}

/// Whether a reach is the unwritten one.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) const fn is_melee_reach(reach: &u8) -> bool {
    *reach == 5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_srd_tables_for_things() {
        assert_eq!(Material::Cloth.armor_class(), Some(11));
        assert_eq!(Material::Glass.armor_class(), Some(13));
        assert_eq!(Material::Wood.armor_class(), Some(15));
        assert_eq!(Material::Stone.armor_class(), Some(17));
        assert_eq!(Material::Metal.armor_class(), Some(19));
        assert_eq!(Material::Liquid.armor_class(), None);
        assert_eq!(hit_points_for(Size::Tiny, true), Some(2));
        assert_eq!(hit_points_for(Size::Tiny, false), Some(5));
        assert_eq!(hit_points_for(Size::Small, false), Some(10));
        assert_eq!(hit_points_for(Size::Medium, true), Some(4));
        assert_eq!(hit_points_for(Size::Medium, false), Some(18));
        assert_eq!(hit_points_for(Size::Large, false), Some(27));
        assert_eq!(hit_points_for(Size::Huge, false), None);
    }

    #[test]
    fn a_durability_needs_numbers_from_somewhere() {
        let plain = Durability::default();
        assert!(
            plain
                .problems(Some(Material::Wood), Some(Size::Small))
                .is_empty()
        );
        assert_eq!(plain.problems(None, Some(Size::Small)).len(), 1);
        assert_eq!(plain.problems(Some(Material::Liquid), None).len(), 2);
        assert_eq!(
            plain.problems(Some(Material::Wood), Some(Size::Huge)).len(),
            1
        );
        let written = Durability {
            hit_points: Some(60),
            armor_class: Some(17),
            ..Durability::default()
        };
        assert!(written.problems(None, None).is_empty());
        assert_eq!(
            written.hit_points(Some(Size::Tiny)),
            Some(60),
            "written wins"
        );
        let silly = Durability {
            hit_points: Some(0),
            armor_class: Some(31),
            ..Durability::default()
        };
        assert_eq!(silly.problems(None, None).len(), 2);
    }

    #[test]
    fn doors_are_wooden_and_breakable_unless_the_map_says() {
        assert_eq!(DoorMake::default().numbers(), Some((15, 18)));
        let iron = DoorMake {
            material: Material::Metal,
            ..DoorMake::default()
        };
        assert_eq!(iron.numbers(), Some((19, 18)));
        let fixed = DoorMake {
            durability: None,
            ..DoorMake::default()
        };
        assert_eq!(fixed.numbers(), None);
    }

    #[test]
    fn a_size_fills_squares_as_the_srd_has_it() {
        assert_eq!(Size::Tiny.per_square(), 4);
        assert_eq!(Size::Medium.per_square(), 1);
        assert_eq!(Size::Large.per_square(), 1);
        assert_eq!(
            [Size::Medium, Size::Large, Size::Huge, Size::Gargantuan].map(Size::space),
            [1, 4, 9, 16]
        );
        assert_eq!(Size::Large.front(), 2);
    }

    #[test]
    fn a_span_is_checked() {
        assert!(span_problems(5, None, false).is_empty());
        assert!(span_problems(10, None, false).is_empty());
        assert!(span_problems(5, Some((80, 320)), true).is_empty());
        assert_eq!(span_problems(0, None, false).len(), 1);
        assert_eq!(span_problems(7, None, false).len(), 1);
        assert_eq!(span_problems(35, None, false).len(), 1);
        assert_eq!(span_problems(5, Some((80, 320)), false).len(), 1, "melee");
        assert_eq!(span_problems(5, Some((320, 80)), true).len(), 1);
        assert_eq!(span_problems(5, Some((0, 80)), true).len(), 1);
    }
}
