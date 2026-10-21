//! Visible beings (M8.5 step 2c): a monster's figure, its own or one by its size, and a fixed
//! encounter that stands at an offset in its cell.

use crate::common;
use omnis_data::monster::figure_for;
use omnis_data::object::{Look, Shape};
use omnis_data::{Size, load_packs};

#[test]
fn a_figure_by_size_grows_with_the_size_and_keeps_to_a_cell_s_width() {
    let looks = [
        (Size::Tiny, (2, 2, 2)),
        (Size::Small, (3, 3, 3)),
        (Size::Medium, (4, 4, 5)),
        (Size::Large, (8, 8, 8)),
        (Size::Huge, (8, 8, 12)),
        (Size::Gargantuan, (8, 8, 16)),
    ];
    for (size, (width, depth, height)) in looks {
        assert_eq!(
            figure_for(size),
            Look {
                shape: Shape::Figure,
                width,
                depth,
                height
            },
            "{size:?}"
        );
    }
}

#[test]
fn a_monster_without_a_look_is_drawn_by_its_size_and_encounters_stand_only_when_told() {
    let [base, test] = common::test_packs();
    let data = load_packs(&[&base, &test]).expect("the test packs load");
    let rat = data
        .registry
        .monsters
        .get("test:monster:giant_rat")
        .expect("rat");
    let monster = &data.monsters[&rat];
    assert_eq!(monster.look, None);
    assert_eq!(monster.figure(), figure_for(monster.size));
    // No fixed encounter in the test pack stands: they stay unseen triggers.
    assert!(
        data.maps
            .values()
            .flat_map(|m| &m.encounters)
            .all(|e| e.stand.is_none())
    );
}
