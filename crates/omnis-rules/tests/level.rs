//! Gaining levels (M7b): hit points, the pool, picks, proficiency and features against SRD
//! figures worked out by hand from packs/base, and which spells a member may add.

use omnis_core::{CharacterId, Pcg32, StreamName};
use omnis_data::{Alignment, Data, Skill, load_packs};
use omnis_rules::{
    Character, Draft, MAX_LEVEL, SpellRefusal, create, eligible, level_up, max_spell_level,
    may_learn, next_threshold, ready,
};
use std::path::PathBuf;

fn data() -> Data {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/base");
    load_packs(&[&base]).unwrap_or_else(|r| panic!("{r}"))
}

fn rng() -> Pcg32 {
    Pcg32::for_stream(1, &StreamName::new("town"))
}

fn make(data: &Data, race: &str, class: &str, scores: [u8; 6], skills: [Skill; 2]) -> Character {
    let draft = Draft {
        name: "Test".to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::NeutralGood,
        scores,
        skills: skills.to_vec(),
    };
    create(&draft, data, CharacterId(0), 0, &mut rng()).unwrap_or_else(|e| panic!("{e:?}"))
}

/// Human fighter: Con 13 + 1 = 14 (+2), d10: 12 at level 1, 10/2 + 1 + 2 = 8 a level after.
fn fighter(data: &Data) -> Character {
    make(
        data,
        "human",
        "fighter",
        [15, 14, 13, 12, 10, 8],
        [Skill::Athletics, Skill::Perception],
    )
}

/// Dwarf cleric: Con 14 + 2 = 16 (+3), d8, toughness +1: 12, then 4 + 1 + 3 + 1 = 9 a level.
/// Wis 15 + 1 = 16 (+3), Int 10 (0), Cha 8 (-1): pool max(level, 3 × level - 1).
fn cleric(data: &Data) -> Character {
    make(
        data,
        "dwarf",
        "cleric",
        [10, 8, 14, 10, 15, 8],
        [Skill::Medicine, Skill::History],
    )
}

/// Elf wizard: Con 13 (+1), d6: 7, then 3 + 1 + 1 = 5 a level. Int 15 + 1 = 16 (+3), Wis 12
/// (+1), Cha 10 (0): pool 3 × level + 1.
fn wizard(data: &Data) -> Character {
    make(
        data,
        "elf",
        "wizard",
        [8, 14, 13, 15, 12, 10],
        [Skill::Arcana, Skill::History],
    )
}

fn spell(data: &Data, name: &str) -> omnis_core::SpellId {
    data.registry
        .spells
        .get(&format!("base:spell:{name}"))
        .unwrap_or_else(|| panic!("no {name}"))
}

#[test]
fn a_fighter_gains_the_average_hit_points_and_the_level_s_features() {
    let data = data();
    let mut brenna = fighter(&data);
    assert_eq!((brenna.hp_max, brenna.spell_points_max), (12, 0));
    brenna.hp = 3;
    let gains = level_up(&mut brenna, &data, &mut rng()).unwrap();
    assert_eq!(gains.hp, 8);
    assert_eq!(gains.spell_points, 0);
    assert_eq!(gains.picks, 0);
    assert_eq!(gains.proficiency, 2);
    assert_eq!(gains.features, ["base:text:class.fighter.action_surge"]);
    assert_eq!((brenna.level, brenna.hp_max, brenna.hp), (2, 20, 11));
    let gains = level_up(&mut brenna, &data, &mut rng()).unwrap();
    assert_eq!(
        gains.features,
        ["base:text:class.fighter.martial_archetype"]
    );
    assert_eq!((brenna.level, brenna.hp_max), (3, 28));
    assert_eq!(brenna.spell_picks, 0);
}

#[test]
fn casters_gain_points_and_picks_and_a_dwarf_a_hit_point_more() {
    let data = data();
    let mut durin = cleric(&data);
    assert_eq!((durin.hp_max, durin.spell_points_max), (12, 2));
    durin.spell_points = 1;
    let gains = level_up(&mut durin, &data, &mut rng()).unwrap();
    assert_eq!((gains.hp, gains.spell_points, gains.picks), (9, 3, 1));
    assert_eq!(
        (durin.hp_max, durin.spell_points_max, durin.spell_points),
        (21, 5, 4),
        "the current points rise by the gain"
    );
    level_up(&mut durin, &data, &mut rng()).unwrap();
    assert_eq!(
        (durin.hp_max, durin.spell_points_max, durin.spell_picks),
        (30, 8, 2)
    );

    let mut ilvara = wizard(&data);
    assert_eq!((ilvara.hp_max, ilvara.spell_points_max), (7, 4));
    let gains = level_up(&mut ilvara, &data, &mut rng()).unwrap();
    assert_eq!((gains.hp, gains.spell_points, gains.picks), (5, 3, 2));
    assert_eq!(gains.features, ["base:text:class.wizard.arcane_tradition"]);
    level_up(&mut ilvara, &data, &mut rng()).unwrap();
    assert_eq!(
        (ilvara.hp_max, ilvara.spell_points_max, ilvara.spell_picks),
        (17, 10, 4)
    );
}

#[test]
fn proficiency_rises_at_five_and_nothing_passes_twenty() {
    let data = data();
    let mut brenna = fighter(&data);
    let mut proficiency = Vec::new();
    for _ in 2..=MAX_LEVEL {
        proficiency.push(
            level_up(&mut brenna, &data, &mut rng())
                .unwrap()
                .proficiency,
        );
    }
    assert_eq!(proficiency[..4], [2, 2, 2, 3], "levels 2 to 5");
    assert_eq!(proficiency.last(), Some(&6));
    assert_eq!(brenna.level, 20);
    assert!(level_up(&mut brenna, &data, &mut rng()).is_err());
    assert_eq!(brenna.level, 20, "a refused level changes nothing");
}

#[test]
fn ready_means_the_experience_reached_an_ungranted_level() {
    let data = data();
    let mut brenna = fighter(&data);
    assert_eq!(next_threshold(&brenna, &data).unwrap(), Some(300));
    assert!(!ready(&brenna, &data).unwrap());
    brenna.xp = 299;
    assert!(!ready(&brenna, &data).unwrap());
    brenna.xp = 300;
    assert!(ready(&brenna, &data).unwrap());
    brenna.xp = 2700;
    assert!(ready(&brenna, &data).unwrap(), "one level at a time");
    level_up(&mut brenna, &data, &mut rng()).unwrap();
    assert_eq!(next_threshold(&brenna, &data).unwrap(), Some(900));
    assert!(ready(&brenna, &data).unwrap());
    brenna.level = 20;
    brenna.xp = u32::MAX;
    assert!(!ready(&brenna, &data).unwrap());
    assert_eq!(next_threshold(&brenna, &data).unwrap(), None);
}

#[test]
fn the_highest_spell_level_follows_the_srd_full_caster() {
    let data = data();
    let max: Vec<u8> = [1, 2, 3, 4, 5, 9, 17, 20]
        .iter()
        .map(|l| max_spell_level(*l, &data).unwrap())
        .collect();
    assert_eq!(max, [1, 1, 2, 2, 3, 5, 9, 9]);
}

#[test]
fn a_spell_may_be_learned_from_the_class_list_up_to_the_level_s_maximum() {
    let mut data = data();
    let mut durin = cleric(&data);
    let (guidance, bless, healing_word, magic_missile) = (
        spell(&data, "guidance"),
        spell(&data, "bless"),
        spell(&data, "healing_word"),
        spell(&data, "magic_missile"),
    );
    let check = |c: &Character, d: &Data, s| may_learn(c, d, s).unwrap();
    assert_eq!(check(&durin, &data, guidance), Some(SpellRefusal::Cantrip));
    assert_eq!(check(&durin, &data, bless), Some(SpellRefusal::Known));
    assert_eq!(
        check(&durin, &data, magic_missile),
        Some(SpellRefusal::NotOnList)
    );
    assert_eq!(check(&durin, &data, healing_word), None);
    let (guiding_bolt, inflict_wounds, spiritual_weapon) = (
        spell(&data, "guiding_bolt"),
        spell(&data, "inflict_wounds"),
        spell(&data, "spiritual_weapon"),
    );
    assert_eq!(
        check(&durin, &data, spiritual_weapon),
        Some(SpellRefusal::TooHigh { level: 2, max: 1 })
    );
    assert_eq!(
        eligible(&durin, &data).unwrap(),
        [healing_word, guiding_bolt, inflict_wounds],
        "the list's first-level spells not yet known, in list order"
    );

    // A second-level healing word: too high at level 2, allowed from level 3.
    data.spells.get_mut(&healing_word).unwrap().level = 2;
    durin.level = 2;
    assert_eq!(
        check(&durin, &data, healing_word),
        Some(SpellRefusal::TooHigh { level: 2, max: 1 })
    );
    assert_eq!(
        eligible(&durin, &data).unwrap(),
        [guiding_bolt, inflict_wounds]
    );
    durin.level = 3;
    assert_eq!(check(&durin, &data, healing_word), None);
    assert_eq!(check(&durin, &data, spiritual_weapon), None);

    let brenna = fighter(&data);
    assert_eq!(check(&brenna, &data, bless), Some(SpellRefusal::NotOnList));
    assert!(eligible(&brenna, &data).unwrap().is_empty());
}
