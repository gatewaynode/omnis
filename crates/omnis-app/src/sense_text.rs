//! A look from afar as text: who looked through what, how far each knowledge layer reached,
//! and the checks behind it. Sits beside `spell_text.rs` and `item_text.rs`. Bevy-free.

use crate::text::{Line, Names};
use omnis_sim::{Event, LayerCheck};

/// The word for a `sense.dc` layer number.
fn layer_word(layer: u8) -> String {
    match layer {
        1 => "terrain".to_owned(),
        2 => "structure".to_owned(),
        3 => "objects".to_owned(),
        4 => "creatures".to_owned(),
        n => format!("layer {n}"),
    }
}

/// `14+3=17` for a check's roll: the face, the modifier with proficiency, the total.
fn roll_math(check: &LayerCheck) -> Option<String> {
    let roll = check.roll.as_ref()?;
    let bonus =
        roll.modifier + roll.proficiency + roll.bonus.as_ref().map_or(0, |b| i64::from(b.total));
    Some(format!("{}{bonus:+}={}", roll.face, roll.total))
}

/// One line for a look, or `None` for events this file does not render.
#[must_use]
pub fn sense_line(event: &Event, names: &Names) -> Option<Line> {
    let Event::Sensed {
        actor,
        item,
        checks,
        tiles,
    } = event
    else {
        return None;
    };
    let who = names.member(*actor);
    let what = names.item(*item);
    if checks.is_empty() {
        return Some(Line::new(
            format!("{who} looks through {what}: nothing ahead"),
            format!("{who} looks: nothing ahead"),
        ));
    }
    let reaches: Vec<String> = checks
        .iter()
        .map(|c| match c.reach {
            0 => format!("{} nothing", layer_word(c.layer)),
            1 => format!("{} to 1 tile", layer_word(c.layer)),
            n => format!("{} to {n} tiles", layer_word(c.layer)),
        })
        .collect();
    let brief: Vec<String> = checks
        .iter()
        .map(|c| format!("{} {}", layer_word(c.layer), c.reach))
        .collect();
    let outcome = format!("{who} looks through {what}: {}", reaches.join(", "));
    let math: Vec<String> = checks.iter().filter_map(roll_math).collect();
    let long = if math.is_empty() {
        outcome
    } else {
        format!("{outcome} ({}; {} tiles)", math.join(", "), tiles.len())
    };
    Some(Line::new(
        long,
        format!("{who} looks: {}", brief.join(", ")),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::tests::{data, facing};
    use omnis_sim::SensedTile;
    use omnis_sim::items::item_id;
    use omnis_sim::omnis_core::{Dice, RollTrace, StreamName};
    use omnis_sim::omnis_rules::{Roll, RollMode};

    fn roll(face: u32, modifier: i64, proficiency: i64) -> Roll {
        Roll {
            trace: RollTrace {
                stream: StreamName::new("sense"),
                dice: Dice::new(1, 20),
                rolls: Vec::new(),
                total: i32::try_from(face).unwrap(),
            },
            mode: RollMode::Normal,
            face,
            modifier,
            proficiency,
            bonus: None,
            total: i64::from(face) + modifier + proficiency,
        }
    }

    #[test]
    fn a_look_reads_as_its_reaches_with_the_rolls_behind_it() {
        let data = data();
        let world = facing(&data, &["fighter"], &[]);
        let names = Names::new(&world, &data);
        let brenna = world.party.members[0].id;
        let glass = item_id(&data, "spyglass").unwrap();
        let looked = Event::Sensed {
            actor: brenna,
            item: glass,
            checks: vec![
                LayerCheck {
                    layer: 1,
                    roll: Some(roll(14, 0, 2)),
                    reach: 7,
                },
                LayerCheck {
                    layer: 2,
                    roll: Some(roll(9, 0, 2)),
                    reach: 1,
                },
            ],
            tiles: (5..=11)
                .map(|y| SensedTile { x: 3, y, layers: 1 })
                .collect(),
        };
        let line = sense_line(&looked, &names).unwrap();
        assert_eq!(line.short, "Brenna looks: terrain 7, structure 1");
        assert_eq!(
            line.long,
            "Brenna looks through Spyglass: terrain to 7 tiles, structure to 1 tile (14+2=16, 9+2=11; 7 tiles)"
        );
        let blind = Event::Sensed {
            actor: brenna,
            item: glass,
            checks: vec![LayerCheck {
                layer: 1,
                roll: None,
                reach: 0,
            }],
            tiles: Vec::new(),
        };
        assert_eq!(
            sense_line(&blind, &names).unwrap().long,
            "Brenna looks through Spyglass: terrain nothing"
        );
        let wall = Event::Sensed {
            actor: brenna,
            item: glass,
            checks: Vec::new(),
            tiles: Vec::new(),
        };
        let wall = sense_line(&wall, &names).unwrap();
        assert_eq!(
            (wall.long.as_str(), wall.short.as_str()),
            (
                "Brenna looks through Spyglass: nothing ahead",
                "Brenna looks: nothing ahead"
            )
        );
        assert!(sense_line(&Event::PartyChanged, &names).is_none());
    }
}
