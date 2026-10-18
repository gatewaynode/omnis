//! The wanderer (M8.5 7a): a seeded party played through the public API, recorded as a replay
//! that reproduces, so the journal's cost can be measured on it.

use omnis_cli::wander::{TELEPORT_EVERY, wander};
use omnis_data::{Data, load_packs};
use omnis_sim::replay::replay_world;
use omnis_sim::save::{join, split};
use std::path::PathBuf;
use std::time::Instant;

fn data() -> Data {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")])
        .unwrap_or_else(|r| panic!("{r}"))
}

#[test]
fn a_wander_is_seeded_and_replays_to_its_fingerprint() {
    let data = data();
    let one = wander(&data, 1, 300).unwrap_or_else(|e| panic!("{e}"));
    let again = wander(&data, 1, 300).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(one.replay, again.replay, "the same seed walks the same way");
    assert_eq!(one.replay.commands.len(), 300);
    one.replay.check(&data).unwrap_or_else(|e| panic!("{e}"));
    let other = wander(&data, 2, 300).unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(
        one.replay.commands, other.replay.commands,
        "another seed, another walk"
    );
}

#[test]
fn a_wander_walks_several_maps_fights_and_raises_the_fallen() {
    let data = data();
    let walk = wander(&data, 1, 3 * TELEPORT_EVERY + 1).unwrap_or_else(|e| panic!("{e}"));
    assert!(walk.maps >= 2, "{} maps", walk.maps);
    assert!(walk.fights >= 1, "{} fights", walk.fights);
    assert!(
        walk.revived >= 1,
        "a fallen member is raised to keep walking"
    );
    assert!(walk.minutes > 0);
    assert!(walk.event_bytes > 0);
}

/// A wander's world, split into its parts and joined back every 25 commands, is the world it was
/// (PRD D28, M8.5 7b): through deaths, revivals, rests, teleports and fights on every map.
#[test]
fn a_wanders_world_splits_and_joins_back_all_the_way() {
    let data = data();
    let walk = wander(&data, 4, 2000).unwrap_or_else(|e| panic!("{e}"));
    let r = &walk.replay;
    let mut world = omnis_sim::World::new(&data, r.seed, r.settings).unwrap();
    let mut checked = 0;
    for (at, command) in r.commands.iter().enumerate() {
        omnis_sim::apply(&mut world, &data, command.clone()).unwrap_or_else(|e| panic!("{e}"));
        if at % 25 == 0 {
            let back = join(split(&world, &data), &data, false)
                .unwrap_or_else(|e| panic!("command {at}: {e}"));
            let mut kept = world.clone();
            kept.log.clear();
            kept.journal = Default::default();
            assert_eq!(back, kept, "command {at}");
            checked += 1;
        }
    }
    assert_eq!(checked, 80);
    assert!(
        walk.maps >= 2 && walk.fights >= 1,
        "{} maps, {} fights",
        walk.maps,
        walk.fights
    );
}

/// The figures the provisional fold is sized from: `cargo test --release -p omnis-cli --test
/// integration journal_timing -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, run in release by hand"]
fn journal_timing() {
    let data = data();
    println!(
        "commands | apply µs/cmd | replay ms | replay µs/cmd | journal B/entry | events B/cmd | maps | fights | game hours | save KB (compact runs) | save KB (pretty runs) | runs | load ms"
    );
    for commands in [10_000, 50_000] {
        let start = Instant::now();
        let walk = wander(&data, 1, commands).unwrap_or_else(|e| panic!("{e}"));
        let played = start.elapsed();
        let r = &walk.replay;
        let start = Instant::now();
        let world =
            replay_world(&data, r.seed, r.settings, &r.commands).unwrap_or_else(|e| panic!("{e}"));
        let replayed = start.elapsed();
        assert_eq!(world.fingerprint().unwrap(), r.fingerprint);
        let journal = omnis_data::ron_io::to_string(&world.journal).unwrap().len();
        // The character save as written (compact, run-length), and as pretty RON for comparison.
        let files = omnis_sim::save::save_files(&world, &data, "x.ron").unwrap();
        let save = &files.last().unwrap().text;
        let parsed: omnis_sim::save::CharacterSave = omnis_data::ron_io::parse(save).unwrap();
        let pretty = omnis_data::ron_io::to_string(&parsed).unwrap().len();
        let start = Instant::now();
        let (loaded, _) = omnis_sim::save::load_save(save, &data, false, &mut |p| {
            files
                .iter()
                .find(|f| f.path == p)
                .map(|f| f.text.clone())
                .ok_or_else(|| p.to_owned())
        })
        .unwrap();
        let load_ms = start.elapsed().as_millis();
        assert_eq!(loaded.fingerprint().unwrap(), r.fingerprint);
        let per = |d: std::time::Duration| d.as_micros() / commands as u128;
        println!(
            "{commands} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {load_ms}",
            per(played),
            replayed.as_millis(),
            per(replayed),
            journal / commands,
            walk.event_bytes / commands,
            walk.maps,
            walk.fights,
            walk.minutes / 60,
            save.len() / 1024,
            pretty / 1024,
            parsed.journal.len(),
        );
    }
}
