//! Equipment slots against the base pack: what a new sheet wears, equipping and unequipping,
//! and the refusals.

use omnis_core::{CharacterId, Dice, ItemId, Pcg32, StreamName};
use omnis_data::{
    Alignment, DamageType, Data, EquipSlot, Item, ItemKind, Skill, WeaponKind, load_packs,
};
use omnis_rules::{
    Character, Draft, EquipRefusal, auto_equip, can_equip, create, equip, equipped_item, modifier,
    unequip,
};
use std::path::PathBuf;

fn data() -> Data {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/base");
    load_packs(&[&base]).unwrap_or_else(|r| panic!("{r}"))
}

fn member(data: &Data, race: &str, class: &str, scores: [u8; 6], skills: &[Skill]) -> Character {
    let draft = Draft {
        name: "Test".to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::NeutralGood,
        scores,
        skills: skills.to_vec(),
    };
    let mut rng = Pcg32::for_stream(1, &StreamName::new("party"));
    create(&draft, data, CharacterId(0), 0, &mut rng).unwrap_or_else(|e| panic!("{e}"))
}

fn item(data: &Data, name: &str) -> ItemId {
    data.registry
        .items
        .get(&format!("base:item:{name}"))
        .unwrap_or_else(|| panic!("{name}"))
}

fn dex(c: &Character) -> i64 {
    modifier(c.scores[1])
}

#[test]
fn a_new_sheet_wears_what_the_kit_rule_counted() {
    let data = data();
    let fighter = member(
        &data,
        "human",
        "fighter",
        [15, 14, 13, 12, 10, 8],
        &[Skill::Athletics, Skill::Perception],
    );
    let worn = auto_equip(&data, &fighter.equipment, dex(&fighter));
    assert_eq!(worn[&EquipSlot::Body], item(&data, "chain_mail"));
    assert_eq!(worn[&EquipSlot::MainHand], item(&data, "longsword"));
    assert_eq!(worn[&EquipSlot::OffHand], item(&data, "shield"));
    assert_eq!(
        worn[&EquipSlot::Ranged],
        item(&data, "light_crossbow"),
        "the crossbow is slung, so the shield stays"
    );
    let cleric = member(
        &data,
        "dwarf",
        "cleric",
        [10, 8, 14, 10, 15, 8],
        &[Skill::Medicine, Skill::History],
    );
    let worn = auto_equip(&data, &cleric.equipment, dex(&cleric));
    assert_eq!(worn[&EquipSlot::Body], item(&data, "scale_mail"));
    assert_eq!(worn[&EquipSlot::MainHand], item(&data, "mace"));
    assert_eq!(worn[&EquipSlot::OffHand], item(&data, "shield"));
    let wizard = member(
        &data,
        "elf",
        "wizard",
        [8, 14, 13, 15, 12, 10],
        &[Skill::Arcana, Skill::History],
    );
    let worn = auto_equip(&data, &wizard.equipment, dex(&wizard));
    assert_eq!(worn.len(), 1);
    assert_eq!(worn[&EquipSlot::MainHand], item(&data, "quarterstaff"));
    assert_eq!(
        equipped_item(&data, &worn, EquipSlot::MainHand).map(|i| i.id.as_str()),
        Some("base:item:quarterstaff")
    );
    assert!(auto_equip(&data, &[], 2).is_empty());
}

#[test]
fn equipping_swaps_slots_and_refuses_the_impossible() {
    let mut data = data();
    let fighter = member(
        &data,
        "human",
        "fighter",
        [15, 14, 13, 12, 10, 8],
        &[Skill::Athletics, Skill::Perception],
    );
    let mut carried = fighter.equipment.clone();
    let mut worn = auto_equip(&data, &carried, dex(&fighter));
    let dagger = item(&data, "dagger");
    assert_eq!(
        can_equip(&data, &carried, &worn, dagger),
        Err(EquipRefusal::NotCarried)
    );
    carried.push((dagger, 1));
    assert_eq!(
        equip(&data, &carried, &mut worn, dagger),
        Ok((EquipSlot::MainHand, Some(item(&data, "longsword")))),
        "the sword is displaced and stays carried"
    );
    assert_eq!(
        equip(&data, &carried, &mut worn, dagger),
        Ok((EquipSlot::MainHand, None))
    );
    assert_eq!(
        can_equip(&data, &carried, &worn, item(&data, "holy_symbol")),
        Err(EquipRefusal::NotEquippable)
    );
    assert_eq!(
        unequip(&mut worn, EquipSlot::OffHand),
        Some(item(&data, "shield"))
    );
    assert_eq!(unequip(&mut worn, EquipSlot::OffHand), None);

    let greatsword = data.registry.items.intern("test:item:greatsword");
    data.items.insert(
        greatsword,
        Item {
            schema: 1,
            id: "test:item:greatsword".into(),
            name: "test:text:item.greatsword.name".into(),
            kind: ItemKind::Weapon {
                kind: WeaponKind::Martial,
                damage: Dice::new(2, 6),
                damage_type: DamageType::Slashing,
                ranged: false,
                two_handed: true,
            },
            cost_cp: 5000,
            weight_tenths: 60,
            use_effect: None,
            consumable: false,
            description: None,
        },
    );
    carried.push((greatsword, 1));
    assert_eq!(
        equip(&data, &carried, &mut worn, greatsword),
        Ok((EquipSlot::MainHand, Some(dagger)))
    );
    assert_eq!(
        can_equip(&data, &carried, &worn, item(&data, "shield")),
        Err(EquipRefusal::HandsFull)
    );
    assert_eq!(unequip(&mut worn, EquipSlot::MainHand), Some(greatsword));
    assert_eq!(
        equip(&data, &carried, &mut worn, item(&data, "shield")),
        Ok((EquipSlot::OffHand, None))
    );
    assert_eq!(
        can_equip(&data, &carried, &worn, greatsword),
        Err(EquipRefusal::HandsFull)
    );
    assert_eq!(
        EquipRefusal::HandsFull.to_string(),
        "a two-handed weapon leaves no hand for a shield"
    );
    let worn = auto_equip(&data, &carried, dex(&fighter));
    assert_eq!(worn[&EquipSlot::MainHand], greatsword, "2d6 beats 1d8");
    assert!(
        !worn.contains_key(&EquipSlot::OffHand),
        "no hand left for the shield"
    );
}
