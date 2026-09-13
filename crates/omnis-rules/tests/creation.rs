//! Character creation and the derived numbers, checked against the SRD figures in packs/base.

use omnis_core::{CharacterId, Pcg32, StreamName};
use omnis_data::ron_io::{parse, to_string};
use omnis_data::{Ability, Alignment, Data, Skill, load_packs};
use omnis_rules::{
    Character, CreationError, Draft, RollMode, armor_class, check, create, level_for_xp, modifier,
    point_cost, proficiency_bonus, save, spell_cost, spell_point_pool,
};
use std::path::PathBuf;

fn data() -> Data {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/base");
    load_packs(&[&base]).unwrap_or_else(|r| panic!("{r}"))
}

fn stream() -> StreamName {
    StreamName::new("party")
}

fn draft(name: &str, race: &str, class: &str, scores: [u8; 6], skills: &[Skill]) -> Draft {
    Draft {
        name: name.to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::NeutralGood,
        scores,
        skills: skills.to_vec(),
    }
}

fn make(data: &Data, draft: &Draft) -> Result<Character, CreationError> {
    let mut rng = Pcg32::for_stream(1, &stream());
    create(draft, data, CharacterId(0), 0, &mut rng)
}

#[test]
fn a_human_fighter_by_point_buy() {
    let data = data();
    let d = draft(
        " Brenna ",
        "human",
        "fighter",
        [15, 14, 13, 12, 10, 8],
        &[Skill::Athletics, Skill::Perception],
    );
    assert_eq!(point_cost(d.scores, &data).unwrap(), 27);
    let c = make(&data, &d).unwrap();
    assert_eq!(c.name, "Brenna", "trimmed");
    assert_eq!(
        c.scores,
        [16, 15, 14, 13, 11, 9],
        "human: +1 to every score"
    );
    assert_eq!(
        (c.level, c.xp, c.hp, c.hp_max),
        (1, 0, 12, 12),
        "10 + Con 14"
    );
    assert_eq!(
        c.skills,
        [
            Skill::Athletics,
            Skill::Insight,
            Skill::Perception,
            Skill::Religion
        ],
        "class picks plus the acolyte's two, sorted"
    );
    assert_eq!((c.spell_points, c.spell_points_max), (0, 0));
    assert!(c.known_spells.is_empty());
    assert_eq!(c.age_years, 18);
    let chain = data.registry.items.get("base:item:chain_mail").unwrap();
    let symbol = data.registry.items.get("base:item:holy_symbol").unwrap();
    assert!(c.equipment.contains(&(chain, 1)));
    assert!(c.equipment.contains(&(symbol, 1)), "background gear too");
    assert_eq!(
        armor_class(&c, &data),
        18,
        "chain mail 16, no Dex, shield 2"
    );
    let mut bare = c.clone();
    bare.equipment.clear();
    assert_eq!(armor_class(&bare, &data), 12, "10 + Dex 15");
    assert_eq!(proficiency_bonus(c.level, &data).unwrap(), 2);
}

#[test]
fn casters_get_their_spell_points_and_spells() {
    let data = data();
    let wizard = make(
        &data,
        &draft(
            "Ilvara",
            "elf",
            "wizard",
            [8, 14, 13, 15, 12, 10],
            &[Skill::Arcana, Skill::History],
        ),
    )
    .unwrap();
    assert_eq!(
        wizard.scores,
        [8, 16, 13, 16, 12, 10],
        "high elf: +2 Dex, +1 Int"
    );
    assert_eq!(wizard.hp_max, 7, "6 + Con 13");
    assert_eq!(
        wizard.spell_points_max, 4,
        "1 × Int +3, plus Wis +1 and Cha 0"
    );
    assert_eq!(
        wizard.known_spells.len(),
        6,
        "three cantrips and three first-level spells"
    );
    assert!(wizard.skills.contains(&Skill::Perception), "keen senses");
    assert_eq!(armor_class(&wizard, &data), 13, "unarmored, Dex 16");

    let cleric = make(
        &data,
        &draft(
            "Durin",
            "dwarf",
            "cleric",
            [10, 8, 14, 10, 15, 8],
            &[Skill::Medicine, Skill::History],
        ),
    )
    .unwrap();
    assert_eq!(
        cleric.scores,
        [10, 8, 16, 10, 16, 8],
        "hill dwarf: +2 Con, +1 Wis"
    );
    assert_eq!(cleric.hp_max, 12, "8 + Con 16 + dwarven toughness");
    assert_eq!(cleric.spell_points_max, 2, "Wis +3 with Int 0 and Cha -1");
    assert_eq!(
        armor_class(&cleric, &data),
        15,
        "scale mail 14 with Dex capped, shield 2, Dex -1"
    );
    let mut rng = Pcg32::for_stream(1, &stream());
    let missile = &data.spells[&data
        .registry
        .spells
        .get("base:spell:magic_missile")
        .unwrap()];
    let bolt = &data.spells[&data.registry.spells.get("base:spell:fire_bolt").unwrap()];
    assert_eq!(spell_cost(missile, &data, &mut rng).unwrap(), 1);
    assert_eq!(spell_cost(bolt, &data, &mut rng).unwrap(), 0);
}

#[test]
fn the_pool_formula_reproduces_the_prd_table_through_a_character() {
    let data = data();
    let mut wizard = make(
        &data,
        &draft(
            "Table",
            "human",
            "wizard",
            [8, 8, 8, 15, 8, 8],
            &[Skill::Arcana, Skill::History],
        ),
    )
    .unwrap();
    let mut rng = Pcg32::for_stream(1, &stream());
    let mut pool = |level: u8, int: u8, wis: u8, cha: u8| {
        wizard.level = level;
        wizard.scores[Ability::Intelligence.index()] = int;
        wizard.scores[Ability::Wisdom.index()] = wis;
        wizard.scores[Ability::Charisma.index()] = cha;
        spell_point_pool(&wizard, &data, &mut rng).unwrap()
    };
    // PRD §8.3 F3, focused caster: casting +3, +4 at 8, +5 at 16; others +1 and +0.
    assert_eq!(pool(1, 16, 12, 10), 4);
    assert_eq!(pool(5, 16, 12, 10), 16);
    assert_eq!(pool(10, 18, 12, 10), 41);
    assert_eq!(pool(20, 20, 12, 10), 101);
    // Dump-stat caster at level 1 hits the floor of the level.
    assert_eq!(pool(1, 16, 8, 8), 1);
    // Gifted caster.
    assert_eq!(pool(5, 16, 14, 14), 19);
}

#[test]
fn drafts_are_refused_for_the_right_reasons() {
    let data = data();
    let ok = [15, 14, 13, 12, 10, 8];
    let two = [Skill::Athletics, Skill::Perception];
    let refuse = |d: Draft| make(&data, &d).unwrap_err();
    assert_eq!(
        refuse(draft(
            "Over",
            "human",
            "fighter",
            [15, 15, 15, 9, 8, 8],
            &two
        )),
        CreationError::Points {
            spent: 28,
            budget: 27
        }
    );
    assert_eq!(
        refuse(draft("High", "human", "fighter", [16, 8, 8, 8, 8, 8], &two)),
        CreationError::ScoreRange {
            score: 16,
            min: 8,
            max: 15
        }
    );
    assert_eq!(
        refuse(draft("Low", "human", "fighter", [7, 8, 8, 8, 8, 8], &two)),
        CreationError::ScoreRange {
            score: 7,
            min: 8,
            max: 15
        }
    );
    assert_eq!(
        refuse(draft("Mage", "human", "mage", ok, &two)),
        CreationError::UnknownClass("base:class:mage".to_owned())
    );
    assert_eq!(
        refuse(draft("Orc", "orc", "fighter", ok, &two)),
        CreationError::UnknownRace("base:race:orc".to_owned())
    );
    assert_eq!(
        refuse(draft("   ", "human", "fighter", ok, &two)),
        CreationError::Name
    );
    let long = "x".repeat(33);
    assert_eq!(
        refuse(draft(&long, "human", "fighter", ok, &two)),
        CreationError::Name
    );
    assert!(matches!(
        refuse(draft("One", "human", "fighter", ok, &[Skill::Athletics])),
        CreationError::Skills(why) if why.contains("choose 2")
    ));
    assert!(matches!(
        refuse(draft("Off", "human", "fighter", ok, &[Skill::Arcana, Skill::Athletics])),
        CreationError::Skills(why) if why.contains("Arcana")
    ));
    assert!(matches!(
        refuse(draft("Twice", "human", "fighter", ok, &[Skill::Athletics, Skill::Athletics])),
        CreationError::Skills(why) if why.contains("already")
    ));
    assert!(matches!(
        refuse(draft("Held", "human", "cleric", ok, &[Skill::Insight, Skill::History])),
        CreationError::Skills(why) if why.contains("Insight")
    ));
    let error = refuse(draft(
        "Over",
        "human",
        "fighter",
        [15, 15, 15, 9, 8, 8],
        &two,
    ));
    assert_eq!(error.to_string(), "28 points spent of 27");
}

#[test]
fn checks_and_saves_roll_a_traced_d20() {
    let data = data();
    let fighter = make(
        &data,
        &draft(
            "Roll",
            "human",
            "fighter",
            [15, 14, 13, 12, 10, 8],
            &[Skill::Athletics, Skill::Perception],
        ),
    )
    .unwrap();
    let mut rng = Pcg32::for_stream(3, &stream());
    let athletics = check(
        &fighter,
        &data,
        Some(Skill::Athletics),
        Ability::Strength,
        RollMode::Normal,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!((athletics.modifier, athletics.proficiency), (3, 2));
    assert_eq!(athletics.total, i64::from(athletics.face) + 5);
    assert_eq!(i64::from(athletics.face), i64::from(athletics.trace.total));
    assert_eq!(athletics.trace.stream, stream());
    assert_eq!(rng.draws(), 1);
    let stealth = check(
        &fighter,
        &data,
        Some(Skill::Stealth),
        Ability::Dexterity,
        RollMode::Normal,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!((stealth.modifier, stealth.proficiency), (2, 0));
    let con = save(
        &fighter,
        &data,
        Ability::Constitution,
        RollMode::Normal,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(con.proficiency, 2, "a fighter's saving throw");
    let dex = save(
        &fighter,
        &data,
        Ability::Dexterity,
        RollMode::Normal,
        &mut rng,
        &stream(),
    )
    .unwrap();
    assert_eq!(dex.proficiency, 0);
    let mut again = Pcg32::for_stream(3, &stream());
    let replayed = check(
        &fighter,
        &data,
        Some(Skill::Athletics),
        Ability::Strength,
        RollMode::Normal,
        &mut again,
        &stream(),
    )
    .unwrap();
    assert_eq!(replayed, athletics, "same seed, same roll");
}

#[test]
fn levels_follow_the_xp_table_and_sheets_round_trip() {
    let data = data();
    for (xp, level) in [
        (0, 1),
        (299, 1),
        (300, 2),
        (6_500, 5),
        (355_000, 20),
        (4_000_000, 20),
    ] {
        assert_eq!(level_for_xp(xp, &data).unwrap(), level, "{xp} xp");
    }
    assert_eq!(proficiency_bonus(5, &data).unwrap(), 3);
    assert_eq!(proficiency_bonus(20, &data).unwrap(), 6);
    assert_eq!(modifier(15), 2);
    let d = draft(
        "Round",
        "halfling",
        "rogue",
        [8, 15, 12, 10, 13, 14],
        &[
            Skill::Stealth,
            Skill::Acrobatics,
            Skill::Deception,
            Skill::Perception,
        ],
    );
    assert_eq!(point_cost(d.scores, &data).unwrap(), 27);
    let c = make(&data, &d).unwrap();
    assert_eq!(
        c.scores[Ability::Dexterity.index()],
        17,
        "lightfoot: +2 Dex"
    );
    let text = to_string(&d).unwrap();
    assert_eq!(parse::<Draft>(&text).unwrap(), d);
    let text = to_string(&c).unwrap();
    assert_eq!(parse::<Character>(&text).unwrap(), c);
}
