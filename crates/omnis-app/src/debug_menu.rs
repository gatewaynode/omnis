//! What the debug panel reads (`debug_panel.rs`, `feathers_debug.rs`): the world's numbers,
//! the items, conditions, flags and maps of the packs, and the stacks of a fight; and the log's
//! line for a `Dev` command. Bevy-free.

use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Data;
use omnis_sim::{DevCommand, Mode, World};

/// One member's numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberDebug {
    /// The name.
    pub name: String,
    /// Hit points and maximum.
    pub hp: (i32, i32),
    /// Spell points and maximum.
    pub sp: (u32, u32),
    /// Experience.
    pub xp: u32,
    /// The level.
    pub level: u8,
    /// The six scores in SRD order.
    pub scores: [u8; 6],
    /// Condition ids in effect.
    pub conditions: Vec<String>,
}

/// One stack of a fight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDebug {
    /// Its index in the encounter.
    pub index: u8,
    /// The monster's name.
    pub name: String,
    /// The living and the initial count.
    pub count: (u8, u8),
    /// The lead individual's hit points; zero when none stands.
    pub lead_hp: i32,
}

/// What the menu reads, built from the world every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugView {
    /// Whether a fight is on: stack edits apply, teleport does not.
    pub fighting: bool,
    /// Whether the world accepts dev commands at all.
    pub devtools: bool,
    /// The members in marching order.
    pub members: Vec<MemberDebug>,
    /// The party's purse in copper.
    pub gold: u32,
    /// The party's food.
    pub food: u32,
    /// Every item id with its label, in id order.
    pub items: Vec<(String, String)>,
    /// Every condition id with its label, in id order.
    pub conditions: Vec<(String, String)>,
    /// Every flag id with its value (0 when unset), in id order.
    pub flags: Vec<(String, i64)>,
    /// Every map id with its size, in id order.
    pub maps: Vec<(String, u16, u16)>,
    /// Where the party stands: the map's index in `maps`, the tile, the facing.
    pub position: (usize, u16, u16, Facing),
    /// The stacks of the fight, when one is on.
    pub stacks: Vec<StackDebug>,
}

/// The view of a world.
#[must_use]
pub fn debug_view(world: &World, data: &Data) -> DebugView {
    let members = world
        .party
        .members
        .iter()
        .map(|m| MemberDebug {
            name: m.name.clone(),
            hp: (m.hp, m.hp_max),
            sp: (m.spell_points, m.spell_points_max),
            xp: m.xp,
            level: m.level,
            scores: m.scores,
            conditions: m
                .conditions
                .iter()
                .map(|c| data.registry.conditions.name(*c).unwrap_or("?").to_owned())
                .collect(),
        })
        .collect();
    let items = data
        .items
        .iter()
        .filter_map(|(id, item)| {
            let name = data.registry.items.name(*id)?;
            Some((name.to_owned(), data.label("en", &item.name).to_owned()))
        })
        .collect();
    let conditions = data
        .conditions
        .iter()
        .filter_map(|(id, c)| {
            let name = data.registry.conditions.name(*id)?;
            Some((name.to_owned(), data.label("en", &c.name).to_owned()))
        })
        .collect();
    let maps: Vec<(String, u16, u16)> = data
        .maps
        .iter()
        .filter_map(|(id, map)| {
            let name = data.registry.maps.name(*id)?;
            Some((name.to_owned(), map.def.width, map.def.height))
        })
        .collect();
    let p = world.position;
    let here = data.registry.maps.name(p.map).unwrap_or("?");
    let map_index = maps.iter().position(|(m, _, _)| m == here).unwrap_or(0);
    let stacks = match &world.mode {
        Mode::Combat(state) => state
            .encounter
            .stacks
            .iter()
            .enumerate()
            .map(|(i, s)| StackDebug {
                index: u8::try_from(i).unwrap_or(u8::MAX),
                name: data
                    .monsters
                    .get(&s.monster)
                    .map_or("?", |m| data.label("en", &m.name))
                    .to_owned(),
                count: (s.count(), s.initial),
                lead_hp: s.hp.first().copied().unwrap_or(0),
            })
            .collect(),
        _ => Vec::new(),
    };
    DebugView {
        fighting: matches!(world.mode, Mode::Combat(_)),
        devtools: world.settings.devtools,
        members,
        gold: world.party.gold,
        food: world.party.food,
        items,
        conditions,
        flags: data
            .registry
            .flags
            .names()
            .map(|name| {
                let value = data
                    .registry
                    .flags
                    .get(name)
                    .and_then(|id| world.flags.get(&id).copied())
                    .unwrap_or(0);
                (name.to_owned(), value)
            })
            .collect(),
        maps,
        position: (map_index, p.x, p.y, p.facing),
        stacks,
    }
}

/// One line for a dev command, for the log.
#[must_use]
pub fn describe(command: &DevCommand) -> String {
    match command {
        DevCommand::GiveItem {
            member,
            item,
            count,
        } => match member {
            Some(m) => format!("dev: {count} x {item} to member {m}"),
            None => format!("dev: {count} x {item} to the stores"),
        },
        DevCommand::SetHp { member, hp } => format!("dev: member {member} hp {hp}"),
        DevCommand::SetSpellPoints { member, points } => {
            format!("dev: member {member} sp {points}")
        }
        DevCommand::SetGold { gold } => format!("dev: gold {gold}"),
        DevCommand::SetFood { food } => format!("dev: food {food}"),
        DevCommand::SetXp { member, xp } => format!("dev: member {member} xp {xp}"),
        DevCommand::SetScore {
            member,
            ability,
            score,
        } => format!("dev: member {member} {} {score}", ability.short()),
        DevCommand::SetCondition {
            member,
            condition,
            applied,
        } => format!(
            "dev: member {member} {condition} {}",
            if *applied { "on" } else { "off" }
        ),
        DevCommand::SetFlag { flag, value } => format!("dev: {flag} = {value}"),
        DevCommand::Teleport { map, x, y, facing } => {
            format!("dev: teleport to {map} ({x}, {y}) facing {facing}")
        }
        DevCommand::SetMonsterHp { stack, index, hp } => {
            format!("dev: stack {stack} #{} hp {hp}", index + 1)
        }
        DevCommand::KillStack { stack } => format!("dev: stack {stack} slain"),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use omnis_sim::omnis_data::load_packs;
    use omnis_sim::{Command, PartyCommand, Settings, apply};
    use std::path::PathBuf;

    pub(crate) fn world_and_data() -> (World, Data) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")])
            .unwrap_or_else(|r| panic!("{r}"));
        let settings = Settings {
            devtools: true,
            ..Settings::default()
        };
        let mut world = World::new(&data, 1, settings).unwrap();
        let draft = omnis_sim::omnis_rules::Draft {
            name: "Brenna".to_owned(),
            race: "base:race:human".to_owned(),
            class: "base:class:fighter".to_owned(),
            background: "base:background:acolyte".to_owned(),
            alignment: omnis_sim::omnis_data::Alignment::LawfulGood,
            scores: [15, 14, 13, 12, 10, 8],
            skills: vec![
                omnis_sim::omnis_data::Skill::Athletics,
                omnis_sim::omnis_data::Skill::Perception,
            ],
        };
        apply(
            &mut world,
            &data,
            Command::Party(PartyCommand::Create(draft)),
        )
        .unwrap();
        (world, data)
    }

    #[test]
    fn the_view_reads_the_world_and_the_packs() {
        let (world, data) = world_and_data();
        let view = debug_view(&world, &data);
        assert!(view.devtools && !view.fighting);
        assert_eq!(view.members.len(), 1);
        assert_eq!(view.members[0].name, "Brenna");
        assert_eq!(view.items.len(), 24);
        assert_eq!(view.items[0].0, "base:item:arrows");
        assert_eq!(view.conditions.len(), 16);
        assert_eq!(view.maps.len(), 4, "town, meadow, dungeon, depths");
        assert_eq!(
            (view.position.1, view.position.2),
            (10, 2),
            "the town start"
        );
        assert!(view.stacks.is_empty());
    }
}
