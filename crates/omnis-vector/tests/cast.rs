//! Spells on the road (parity P5): the cast view the shell reads and the HUD lines a cast
//! makes. Real packs, the fixed party, the town's start.

use crate::common;

use omnis_sim::{Command, Target};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::shell::text;

#[test]
fn the_views_hold_each_member_s_road_spells_and_a_cast_reads_as_lines() {
    let data = common::data();
    let (mut world, _) = common::town_session(&data);
    let views = common::views(&world, &data);
    assert_eq!(
        views.casts,
        world.casts(&data),
        "the views carry the cast view"
    );
    let ilvara = common::id(&world, 2);
    let light = views
        .casts
        .iter()
        .find(|r| r.caster == ilvara && r.spell == "base:spell:light")
        .expect("Ilvara lights the way");
    assert_eq!((light.cost, light.refusal.as_ref()), (0, None));

    let mut names = Names::default();
    names.observe(&views.party);
    let events = world
        .apply(
            &data,
            Command::Cast {
                caster: ilvara,
                spell: "base:spell:light".into(),
                target: Target::Member(ilvara),
            },
        )
        .expect("cast");
    let got: Vec<String> = events
        .iter()
        .filter_map(|e| text::describe(e, &names, &data))
        .collect();
    let light = labels::spell(&data, "base:spell:light");
    assert_eq!(
        got,
        [
            format!("Ilvara casts {light} (free)"),
            format!("{light} lights the party's way"),
        ]
    );
}
