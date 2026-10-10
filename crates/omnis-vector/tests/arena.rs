//! The fight screen's layout on a real fight (presentation-ARCHITECTURE.md §9): the dungeon's placed
//! group of two giant rats, met by walking into it. The figures stand where the plan puts them
//! at the owner's two window sizes, a click lands on the figure under it and nowhere else, and
//! the marks frame whoever acts and whatever can be clicked.

use crate::common;

use common::fight::{met, turn_of};
use common::id;
use omnis_sim::{Mode, World};
use omnis_vector::arena::{Arena, Group, Rect, State, Tone, arena, pick, segments};
use omnis_vector::combat_menu::{CombatMenu, Pick};
use omnis_vector::shell::session::Session;

/// The owner's main display and ultrawide, in logical pixels.
const SIZES: [(f32, f32); 2] = [(1600.0, 900.0), (5120.0, 1440.0)];

fn laid_out(session: &Session, size: (f32, f32)) -> Arena {
    arena(&session.world, &session.data, size, &[], None).expect("a fight to lay out")
}

fn centre(r: &Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn group(arena: &Arena, pick: Pick) -> &Group {
    arena
        .groups()
        .find(|g| g.pick == pick)
        .unwrap_or_else(|| panic!("a group for {pick:?}"))
}

#[test]
fn nothing_to_lay_out_outside_a_fight() {
    let session = met();
    let mut world = session.world.clone();
    world.mode = Mode::Explore;
    assert!(arena(&world, &session.data, SIZES[0], &[], None).is_none());
}

#[test]
fn the_rats_stand_above_the_party_in_its_two_rows() {
    let session = met();
    for size in SIZES {
        let arena = laid_out(&session, size);
        assert_eq!(arena.stacks.len(), 1, "one stack");
        let rats = &arena.stacks[0];
        assert_eq!(rats.pick, Pick::Stack(0));
        assert_eq!(rats.figures.len(), 2, "a figure per rat");
        assert_eq!(rats.label.text, "Giant Rat");
        let picks: Vec<Pick> = arena.members.iter().map(|g| g.pick).collect();
        let marching = session
            .world
            .party
            .members
            .iter()
            .map(|m| Pick::Member(m.id));
        assert_eq!(picks, marching.collect::<Vec<_>>());
        let names: Vec<&str> = arena
            .members
            .iter()
            .map(|g| g.label.text.as_str())
            .collect();
        assert_eq!(names, ["Brenna", "Durin", "Ilvara", "Pip"]);
        // The front row (the pack's first three in marching order) stands nearer: lower on
        // the screen. Pip, behind, stands higher.
        let foot = |slot: u8| {
            group(&arena, Pick::Member(id(&session.world, slot))).figures[0]
                .rect
                .bottom()
        };
        let party_top = arena
            .members
            .iter()
            .map(|g| g.slot.y)
            .fold(f32::MAX, f32::min);
        assert!((0..3).all(|m| foot(m) > foot(3)), "front row lower");
        assert!((foot(0) - foot(1)).abs() < 0.01 && (foot(1) - foot(2)).abs() < 0.01);
        assert!(
            rats.slot.bottom() <= party_top,
            "the monsters above the party"
        );
        for g in arena.groups() {
            for f in &g.figures {
                assert_eq!(f.state, State::Standing);
                assert!(f.health > 0.0 && f.health <= 1.0, "{f:?}");
            }
        }
    }
}

/// The world with a stack of rats added for each count. The pack keeps two stacks in front,
/// so a third stands behind.
fn with_stacks(world: &World, counts: &[u8]) -> World {
    let mut world = world.clone();
    let Mode::Encounter(encounter) = &mut world.mode else {
        panic!("met in an encounter");
    };
    for &count in counts {
        let mut more = encounter.stacks[0].clone();
        more.initial = count;
        more.hp = vec![3; usize::from(count)];
        encounter.stacks.push(more);
    }
    world
}

#[test]
fn a_stack_behind_stands_higher_and_smaller_and_a_crowd_shows_its_count() {
    let session = met();
    let world = with_stacks(&session.world, &[1, 12]);
    for size in SIZES {
        let arena = arena(&world, &session.data, size, &[], None).expect("a fight");
        let front = group(&arena, Pick::Stack(0));
        let beside = group(&arena, Pick::Stack(1));
        let back = group(&arena, Pick::Stack(2));
        assert_eq!(beside.figures.len(), 1);
        assert!(
            (beside.figures[0].rect.bottom() - front.figures[0].rect.bottom()).abs() < 0.01,
            "the two in front share a ground line"
        );
        assert_eq!(back.figures.len(), 8, "no more than eight drawn");
        assert_eq!(back.label.text, "Giant Rat x12");
        let (f, b) = (&front.figures[0].rect, &back.figures[0].rect);
        assert!(b.bottom() < f.y, "behind stands higher: {b:?} over {f:?}");
        assert!(b.h < f.h, "and smaller");
        // Health 3 against the dice's most.
        assert!(back.figures[0].health < 1.0);
    }
}

#[test]
fn everything_stays_in_the_field_clear_of_the_picture_and_log() {
    let session = met();
    let world = with_stacks(&session.world, &[1, 12]);
    let mut tallest = Vec::new();
    for size in SIZES {
        let arena = arena(&world, &session.data, size, &[], None).expect("a fight");
        let l = arena.layout;
        let field = l.field.grown(0.01);
        for s in segments(&arena) {
            assert!(
                field.contains(s.a) && field.contains(s.b),
                "{s:?} in {size:?}"
            );
        }
        for g in arena.groups() {
            assert!(field.encloses(&g.slot) && g.slot.encloses(&g.label.rect));
            for r in [&g.slot, &g.label.rect] {
                for part in [l.picture, l.actions, l.status, l.log, l.buttons, l.title] {
                    assert!(!r.overlaps(&part), "{r:?} clear of {part:?}");
                }
            }
        }
        tallest.push(
            group(&arena, Pick::Member(id(&session.world, 0))).figures[0]
                .rect
                .h,
        );
    }
    assert!(
        tallest[1] > tallest[0],
        "figures grow with the window: {tallest:?}"
    );
}

#[test]
fn a_click_lands_on_the_figure_under_it_and_nowhere_else() {
    let session = met();
    for size in SIZES {
        let arena = laid_out(&session, size);
        for g in arena.groups() {
            for f in &g.figures {
                assert_eq!(pick(&arena, centre(&f.rect)), Some(g.pick));
                assert_eq!(pick(&arena, centre(&f.bar)), Some(g.pick), "the bar too");
            }
        }
        // Between the two rats, between two members of a row, and on the other parts.
        let rats = &arena.stacks[0].figures;
        let between = (
            (rats[0].rect.right() + rats[1].rect.x) / 2.0,
            centre(&rats[0].rect).1,
        );
        assert_eq!(pick(&arena, between), None);
        let (a, b) = (
            &group(&arena, Pick::Member(id(&session.world, 0))).figures[0].rect,
            &group(&arena, Pick::Member(id(&session.world, 1))).figures[0].rect,
        );
        assert_eq!(pick(&arena, ((a.right() + b.x) / 2.0, centre(a).1)), None);
        for part in [arena.layout.picture, arena.layout.log, arena.layout.title] {
            assert_eq!(pick(&arena, centre(&part)), None);
        }
    }
}

fn tones(arena: &Arena, tone: Tone) -> Vec<omnis_vector::arena::Seg2> {
    segments(arena)
        .into_iter()
        .filter(|s| s.tone == tone)
        .collect()
}

#[test]
fn the_marks_frame_whoever_acts_and_the_targets() {
    let session = turn_of("Durin");
    let (world, data) = (&session.world, &session.data);
    let targets = CombatMenu::default().clickable(world, data);
    assert_eq!(targets, [Pick::Stack(0)], "the rats in reach");
    for size in SIZES {
        let plain = arena(world, data, size, &targets, None).expect("a fight");
        assert_eq!(
            plain.marks.acting,
            Some(Pick::Member(id(world, 1))),
            "Durin acts"
        );
        let durin = group(&plain, Pick::Member(id(world, 1))).slot;
        let acting = tones(&plain, Tone::Acting);
        assert_eq!(acting.len(), 8, "four corner brackets");
        assert!(
            acting
                .iter()
                .all(|s| durin.contains(s.a) && durin.contains(s.b))
        );
        let rats = group(&plain, Pick::Stack(0)).slot;
        let framed = tones(&plain, Tone::Target);
        assert_eq!(framed.len(), 4, "one frame");
        assert!(
            framed
                .iter()
                .all(|s| rats.contains(s.a) && rats.contains(s.b))
        );
        assert!(tones(&plain, Tone::Hover).is_empty());
        // The pointer on the rats turns their frame to the hover's.
        let hovered = arena(world, data, size, &targets, Some(Pick::Stack(0))).expect("a fight");
        assert!(tones(&hovered, Tone::Target).is_empty());
        assert_eq!(tones(&hovered, Tone::Hover).len(), 4);
    }
}

#[test]
fn a_member_at_zero_is_drawn_down_with_an_empty_bar() {
    let session = met();
    let mut world = session.world.clone();
    world.party.members[0].hp = 0;
    let arena = arena(&world, &session.data, SIZES[0], &[], None).expect("a fight");
    let brenna = &group(&arena, Pick::Member(id(&world, 0))).figures[0];
    assert_eq!(brenna.state, State::Down);
    assert!(brenna.health == 0.0);
    let down = tones(&arena, Tone::Down);
    assert!(!down.is_empty());
    assert!(
        down.iter()
            .all(|s| brenna.rect.contains(s.a) && brenna.rect.contains(s.b))
    );
}
