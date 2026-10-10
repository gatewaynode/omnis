//! The fixed party the viewer starts with (alt-ARCHITECTURE.md §9): four base-pack drafts,
//! created through commands so the session log replays from a fresh world.

use omnis_sim::omnis_data::{Alignment, Skill};
use omnis_sim::omnis_rules::Draft;

fn draft(name: &str, race: &str, class: &str, scores: [u8; 6], skills: &[Skill]) -> Draft {
    Draft {
        name: name.to_owned(),
        race: format!("base:race:{race}"),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::LawfulGood,
        scores,
        skills: skills.to_vec(),
    }
}

/// A fighter, a cleric, a wizard and a rogue, in marching order.
#[must_use]
pub fn fixed() -> Vec<Draft> {
    vec![
        draft(
            "Brenna",
            "human",
            "fighter",
            [15, 14, 13, 12, 10, 8],
            &[Skill::Athletics, Skill::Perception],
        ),
        draft(
            "Durin",
            "dwarf",
            "cleric",
            [10, 8, 14, 10, 15, 8],
            &[Skill::Medicine, Skill::History],
        ),
        draft(
            "Ilvara",
            "elf",
            "wizard",
            [8, 14, 13, 15, 12, 10],
            &[Skill::Arcana, Skill::History],
        ),
        draft(
            "Pip",
            "halfling",
            "rogue",
            [8, 15, 12, 10, 13, 14],
            &[
                Skill::Stealth,
                Skill::Acrobatics,
                Skill::Deception,
                Skill::Perception,
            ],
        ),
    ]
}
