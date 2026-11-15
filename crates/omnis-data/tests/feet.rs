//! Combat in feet and things that break (owner, 2026-11-14; `tasks/plans/combat-in-feet.md`):
//! what a file may write, what it gets when it writes nothing, and what the loader refuses.

use omnis_data::ron_io::{from_str, to_string};
use omnis_data::{
    DataError, Durability, Item, Material, Monster, ObjectKind, Reach, Size, Spell, SpellEffect,
    Utility,
};
use std::path::Path;

fn read<T: serde::de::DeserializeOwned>(text: &str) -> T {
    from_str(text, Path::new("x.ron")).unwrap_or_else(|e| panic!("{e:?}"))
}

fn messages(check: impl FnOnce(&Path, &mut Vec<DataError>)) -> Vec<String> {
    let mut errors = Vec::new();
    check(Path::new("x.ron"), &mut errors);
    errors.into_iter().map(|e| e.message).collect()
}

fn weapon(extra: &str) -> Item {
    read(&format!(
        "(schema: 1, id: \"t:item:w\", name: \"t:text:w\", kind: Weapon(kind: Martial, damage: \
         (count: 1, sides: 10, modifier: 0), damage_type: Slashing{extra}))"
    ))
}

fn monster(extra: &str) -> Monster {
    read(&format!(
        "(schema: 1, id: \"t:monster:m\", name: \"t:text:m\", ac: 12, hit_points: (count: 1, \
         sides: 6, modifier: 0), speed: 30, abilities: (10, 10, 10, 10, 10, 10), challenge: \
         (1, 4), xp: 50{extra})"
    ))
}

fn object(attributes: &str) -> ObjectKind {
    read(&format!(
        "(schema: 1, id: \"t:object:o\", name: \"t:text:o\", look: (shape: Box, width: 4, depth: \
         4, height: 4), initial_state: \"whole\", states: {{\"whole\": (), \"wreck\": ()}}, \
         attributes: ({attributes}))"
    ))
}

fn spell(effect: &str, extra: &str) -> Spell {
    read(&format!(
        "(schema: 1, id: \"t:spell:s\", name: \"t:text:s\", level: 1, school: Evocation, classes: \
         [\"t:class:c\"], cost: Action, bonus_action_available: false, preparation_available: \
         false, preparation_required_for_bonus_action: false, description: \"t:text:d\", effect: \
         Some({effect}){extra})"
    ))
}

#[test]
fn a_weapon_that_writes_nothing_reaches_five_feet_and_shoots_eighty() {
    let sword = weapon("");
    assert_eq!(sword.span(), Some((5, None)));
    assert!(messages(|f, e| sword.validate(f, e)).is_empty());
    let bow = weapon(", ranged: true");
    assert_eq!(bow.span(), Some((5, Some((80, 320)))));
    let glaive = weapon(", reach: 10");
    assert_eq!(glaive.span(), Some((10, None)));
    let longbow = weapon(", ranged: true, range: Some((150, 600))");
    assert_eq!(longbow.span(), Some((5, Some((150, 600)))));
    assert!(messages(|f, e| longbow.validate(f, e)).is_empty());
    // What is written is kept; what is the default is not written back.
    let text = to_string(&glaive).unwrap().replace(' ', "");
    assert!(
        text.contains("reach:10") && !text.contains("range:"),
        "{text}"
    );
    assert!(!to_string(&sword).unwrap().contains("reach"));
}

#[test]
fn a_bad_reach_or_range_is_refused() {
    for (extra, problems) in [
        (", reach: 7", 1),
        (", reach: 0", 1),
        (", reach: 35", 1),
        (", range: Some((80, 320))", 1),
        (", ranged: true, range: Some((320, 80))", 1),
        (", ranged: true, range: Some((0, 80))", 1),
        (", reach: 12, range: Some((80, 320))", 2),
    ] {
        let found = messages(|f, e| weapon(extra).validate(f, e));
        assert_eq!(found.len(), problems, "{extra}: {found:?}");
    }
    let bad = monster(
        ", size: Medium, attacks: [(name: \"t:text:a\", to_hit: 2, damage: (count: 1, sides: 6, \
         modifier: 0), damage_type: Piercing, reach: 3)]",
    );
    assert_eq!(messages(|f, e| bad.validate(f, e)).len(), 1);
}

#[test]
fn how_many_fit_a_square_is_the_size_unless_the_type_says() {
    let rat = monster(", size: Tiny");
    assert_eq!(
        (rat.per_square(), rat.squares(9), rat.fitting(2)),
        (4, 3, 8)
    );
    let kobold = monster(", size: Small, per_square: Some(2)");
    assert_eq!(
        (kobold.per_square(), kobold.squares(5), kobold.fitting(3)),
        (2, 3, 6)
    );
    assert!(messages(|f, e| kobold.validate(f, e)).is_empty());
    let human = monster(", size: Medium");
    assert_eq!(
        (human.per_square(), human.squares(5), human.fitting(3)),
        (1, 5, 3)
    );
    let ogre = monster(", size: Large");
    assert_eq!(
        (
            ogre.squares(2),
            ogre.fitting(9),
            ogre.fitting(3),
            ogre.squares(0)
        ),
        (8, 2, 1, 0),
        "four squares each, and one always fits"
    );
    // Across a line: the kobolds two a square, the ogre two squares wide.
    assert_eq!(
        (kobold.line(5), kobold.abreast(3), kobold.abreast(0)),
        (3, 6, 0)
    );
    assert_eq!((human.line(5), human.abreast(3)), (5, 3));
    assert_eq!((ogre.line(2), ogre.abreast(5), ogre.abreast(1)), (4, 2, 1));
    for (extra, why) in [
        (", size: Small, per_square: Some(0)", "none fit"),
        (", size: Small, per_square: Some(17)", "too many"),
        (
            ", size: Large, per_square: Some(2)",
            "a big one takes squares",
        ),
    ] {
        let found = messages(|f, e| monster(extra).validate(f, e));
        assert_eq!(found.len(), 1, "{why}: {found:?}");
    }
}

#[test]
fn an_object_is_durable_only_when_its_kind_says() {
    let plain = object("material: Some(Wood), size: Some(Small)");
    assert_eq!(plain.attributes.durability, None, "size and material alone");
    let chest = object("material: Some(Wood), size: Some(Small), durability: Some(())");
    let made = chest.attributes.durability.clone().unwrap();
    assert_eq!(made, Durability::default());
    assert_eq!(
        (
            made.armor_class(chest.attributes.material),
            made.hit_points(chest.attributes.size)
        ),
        (Some(15), Some(10))
    );
    assert!(messages(|f, e| chest.validate(f, e)).is_empty());
    let vase = object(
        "material: Some(Glass), size: Some(Tiny), durability: Some((fragile: true, \
         broken_state: Some(\"wreck\")))",
    );
    let made = vase.attributes.durability.clone().unwrap();
    assert_eq!(
        (
            made.armor_class(Some(Material::Glass)),
            made.hit_points(Some(Size::Tiny))
        ),
        (Some(13), Some(2))
    );
    assert!(messages(|f, e| vase.validate(f, e)).is_empty());
    for (attributes, problems) in [
        ("durability: Some(())", 2),
        (
            "material: Some(Liquid), size: Some(Small), durability: Some(())",
            1,
        ),
        (
            "material: Some(Stone), size: Some(Huge), durability: Some(())",
            1,
        ),
        (
            "material: Some(Stone), size: Some(Huge), durability: Some((hit_points: Some(200), \
             threshold: 10))",
            0,
        ),
        (
            "material: Some(Wood), size: Some(Small), durability: Some((broken_state: \
             Some(\"gone\")))",
            1,
        ),
        (
            "durability: Some((hit_points: Some(0), armor_class: Some(40)))",
            2,
        ),
    ] {
        let found = messages(|f, e| object(attributes).validate(f, e));
        assert_eq!(found.len(), problems, "{attributes}: {found:?}");
    }
}

#[test]
fn a_spell_states_its_area_and_whether_it_strikes_things() {
    let save = "Save(ability: Dexterity, dice: (count: 3, sides: 6, modifier: 0), damage_type: \
                Fire, half_on_save: true)";
    let cone = spell(save, ", reach: Area(squares: 6)");
    assert_eq!(cone.reach, Reach::Area { squares: 6 });
    assert!(!cone.objects);
    assert!(messages(|f, e| cone.validate(f, e)).is_empty());
    let none = spell(save, ", reach: Area(squares: 0)");
    assert_eq!(messages(|f, e| none.validate(f, e)).len(), 1);
    let bolt = "Attack(dice: (count: 1, sides: 10, modifier: 0), damage_type: Fire)";
    assert_eq!(
        messages(|f, e| spell(bolt, ", reach: Area(squares: 6)").validate(f, e)).len(),
        1,
        "an attack is at one target"
    );
    assert!(messages(|f, e| spell(bolt, ", objects: true").validate(f, e)).is_empty());
    let heal = "Heal(dice: (count: 1, sides: 8, modifier: 0), add_mod: true)";
    assert_eq!(
        messages(|f, e| spell(heal, ", objects: true").validate(f, e)).len(),
        1,
        "only a spell that strikes strikes things"
    );
    let mend = spell("Utility(Mend(up_to: Small))", "");
    assert_eq!(
        mend.effect,
        Some(SpellEffect::Utility(Utility::Mend { up_to: Size::Small }))
    );
    assert_eq!(mend.targeting(), omnis_data::Targeting::TOUCH);
    assert!(messages(|f, e| mend.validate(f, e)).is_empty());
}

#[test]
fn an_item_is_a_tool_or_supplies_for_named_materials() {
    let gear = |extra: &str| -> Item {
        read(&format!(
            "(schema: 1, id: \"t:item:g\", name: \"t:text:g\", kind: Gear{extra})"
        ))
    };
    let plain = gear("");
    assert!(!plain.fragile && plain.craft.is_none());
    let text = to_string(&plain).unwrap();
    assert!(
        !text.contains("fragile") && !text.contains("craft"),
        "{text}"
    );
    let saw = gear(", craft: Some(Tool([Wood, Bone]))");
    assert_eq!(
        saw.craft.as_ref().unwrap().materials(),
        [Material::Wood, Material::Bone]
    );
    assert!(messages(|f, e| saw.validate(f, e)).is_empty());
    let nothing = gear(", craft: Some(Supply([]))");
    assert_eq!(messages(|f, e| nothing.validate(f, e)).len(), 1);
    assert!(gear(", fragile: true").fragile);
}

/// The tile cap: on a map that states its scale a group must fit one tile's five-foot squares,
/// each monster taking what its type says; a map that states none is not held to it.
#[test]
fn a_group_must_fit_the_squares_of_a_scaled_tile() {
    use crate::biospheres::{refused, rewrite};
    use crate::common::{base_pack, copy_pack, fixture_pack, test_pack};
    let scaled = |name: &str, feet: u32, per_square: Option<u8>| {
        let pack = copy_pack(&fixture_pack("objects"), name);
        let map = "data/maps/yard.ron";
        let scale = format!("id: \"objects:map:yard\",\n    tile_feet: Some({feet}),");
        rewrite(&pack, map, "id: \"objects:map:yard\",", &scale);
        // The shamblers down the hall are another matter: four of them share a square here.
        let shambler = "data/monsters/shambler.ron";
        rewrite(
            &pack,
            shambler,
            "size: Medium,",
            "size: Medium, per_square: Some(4),",
        );
        if let Some(n) = per_square {
            let rat = "data/monsters/pack_rat.ron";
            rewrite(
                &pack,
                rat,
                "size: Small,",
                &format!("size: Small, per_square: Some({n}),"),
            );
        }
        omnis_data::seal::seal(&pack, None).unwrap();
        pack
    };
    // Two pack rats, one a square: a tile of five feet holds one of them.
    let tight = scaled("tile-cap-tight", 5, None);
    let text = refused(&[&base_pack(), &test_pack(), &tight]);
    assert!(
        text.contains("encounter 'rat' needs 2 five-foot squares; a tile of 5 ft holds 1"),
        "{text}"
    );
    // Two to a square, they fit it; and a tile of ten feet holds four squares.
    for (name, feet, per_square) in [("tile-cap-pairs", 5, Some(2)), ("tile-cap-wide", 10, None)] {
        let pack = scaled(name, feet, per_square);
        omnis_data::load_packs(&[&base_pack(), &test_pack(), &pack])
            .unwrap_or_else(|report| panic!("{name}: {report}"));
    }
    // A random table is held to it at its fullest roll.
    let pack = copy_pack(&test_pack(), "tile-cap-random");
    let id = "id: \"test:map:dungeon\",";
    let scale = format!("{id}\n    tile_feet: Some(5),");
    rewrite(&pack, "data/maps/dungeon.ron", id, &scale);
    omnis_data::seal::seal(&pack, None).unwrap();
    let text = refused(&[&base_pack(), &pack]);
    assert!(
        text.contains("random encounter 1 at its fullest needs"),
        "{text}"
    );
    // The yard as shipped states no scale and loads, whatever stands on its tiles.
    omnis_data::load_packs(&[&base_pack(), &test_pack(), &fixture_pack("objects")])
        .unwrap_or_else(|report| panic!("{report}"));
}
