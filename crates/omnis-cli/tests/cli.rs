//! The binary's subcommands on the real test pack, the bad-pack corpus, and the golden replay.

use omnis_data::load_packs;
use omnis_sim::Settings;
use omnis_sim::parse_script;
use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the binary from the repository root: `(succeeded, stdout, stderr)`.
fn cli(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_omnis-cli"))
        .current_dir(repo())
        .args(args)
        .output()
        .expect("the binary runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn validate_accepts_the_test_pack_and_reports_every_error_of_a_broken_one() {
    let (ok, out, _) = cli(&["validate", "packs/base", "packs/test"]);
    assert!(ok);
    assert!(out.starts_with("ok: 2 packs, 4 maps, 2 tilesets"), "{out}");
    assert!(
        out.ends_with("36 rule slots, 7 services, 0 objects\n"),
        "{out}"
    );
    let (ok, _, err) = cli(&["validate", "packs/test"]);
    assert!(!ok);
    assert!(
        err.contains("depends on 'base', which is not loaded before this pack"),
        "{err}"
    );
    let broken = "crates/omnis-data/tests/packs-bad/broken";
    let (ok, _, err) = cli(&["validate", broken]);
    assert!(!ok);
    assert!(err.contains("pack id 'Broken' is not [a-z0-9_-]+"), "{err}");
    assert!(
        err.contains("portal at (0, 0) leads to unknown map"),
        "{err}"
    );
}

#[test]
fn map_text_marks_the_start_and_names_unknown_maps() {
    let (ok, out, _) = cli(&["map", "text", "test:map:town"]);
    assert!(ok);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 11);
    assert_eq!(
        lines[2 * 2 + 1].chars().nth(2 * 10 + 1),
        Some('<'),
        "a new game faces west on the town's street"
    );
    let (ok, _, err) = cli(&["map", "text", "nope:map:x"]);
    assert!(!ok);
    assert!(err.contains("no map 'nope:map:x' is loaded"), "{err}");
}

#[test]
fn play_prints_events_and_the_fingerprint_the_library_computes() {
    let dir = scratch("cli-play");
    let script = dir.join("walk.txt");
    let text = "forward, forward\nturn-left # face west\nuse\n";
    std::fs::write(&script, text).unwrap();
    let (ok, out, err) = cli(&["play", "--script", script.to_str().unwrap(), "--seed", "9"]);
    assert!(ok, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].starts_with("0 forward: Moved"), "{}", lines[0]);
    assert!(
        lines.iter().any(|l| l.starts_with("3 use: Message")),
        "{out}"
    );
    let data = load_packs(&[&repo().join("packs/base"), &repo().join("packs/test")]).unwrap();
    // The headless driver is a dev world (`Settings.devtools`), and a save says so.
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    // The walk names no member or row, so the words resolve against the new world alike.
    let fresh = omnis_sim::World::new(&data, 9, settings).unwrap();
    let commands: Vec<_> = parse_script(text)
        .unwrap()
        .iter()
        .map(|w| w.command(&fresh, &data).unwrap())
        .collect();
    let expected = omnis_sim::replay::run(&data, 9, settings, &commands).unwrap();
    assert_eq!(
        lines.last().copied(),
        Some(format!("fingerprint: {expected:016x}").as_str())
    );

    std::fs::write(&script, "forward\nfly\n").unwrap();
    let (ok, _, err) = cli(&["play", "--script", script.to_str().unwrap()]);
    assert!(!ok);
    assert!(err.contains("line 2: unknown command 'fly'"), "{err}");
    // A word names a slot, resolved when it is applied: the headless party is empty, so the
    // fourth slot names nobody.
    std::fs::write(&script, "forward\ncast-0-m3\n").unwrap();
    let (ok, _, err) = cli(&["play", "--script", script.to_str().unwrap()]);
    assert!(!ok);
    assert!(
        err.contains("turn 1: 'cast-0-m3' names an empty slot"),
        "{err}"
    );
    let (ok, _, err) = cli(&["play"]);
    assert!(!ok && err.contains("--script"), "{err}");
}

#[test]
fn replay_checks_the_golden_file_and_catches_tampering() {
    let golden = "crates/omnis-sim/tests/replays/walk.ron";
    let (ok, out, err) = cli(&[
        "replay",
        golden,
        "--pack",
        "packs/base",
        "--pack",
        "packs/test",
    ]);
    assert!(ok, "{err}");
    assert!(
        out.starts_with("ok: ") && out.contains("commands reproduce fingerprint"),
        "{out}"
    );
    let text = std::fs::read_to_string(repo().join(golden)).unwrap();
    let start = text.find("fingerprint: ").unwrap() + "fingerprint: ".len();
    let end = start + text[start..].find(',').unwrap();
    let recorded: u64 = text[start..end].trim().parse().unwrap();
    let tampered = format!("{}{}{}", &text[..start], recorded ^ 1, &text[end..]);
    let file = scratch("cli-replay").join("tampered.ron");
    std::fs::write(&file, tampered).unwrap();
    let (ok, _, err) = cli(&["replay", file.to_str().unwrap()]);
    assert!(!ok);
    assert!(err.contains("differs from recorded"), "{err}");
}

#[test]
fn wander_writes_a_replay_that_replay_times() {
    let file = scratch("cli-wander").join("w.ron");
    let path = file.to_str().unwrap();
    let (ok, out, err) = cli(&["wander", "--seed", "3", "--commands", "120", "--out", path]);
    assert!(ok, "{err}");
    assert!(out.starts_with("wrote 120 commands to "), "{out}");
    assert!(out.contains("bytes of events a command"), "{out}");
    let (ok, out, err) = cli(&["replay", path, "--time"]);
    assert!(ok, "{err}");
    assert!(out.starts_with("ok: 120 commands reproduce"), "{out}");
    assert!(out.contains("µs a command"), "{out}");
    assert!(out.contains(" interactions, "), "{out}");
    let (ok, _, err) = cli(&["wander", "--commands", "5"]);
    assert!(!ok && err.contains("--out"), "{err}");
}

#[test]
fn nonsense_prints_usage_and_schema_dump_prints_sections() {
    let (ok, _, err) = cli(&["dance"]);
    assert!(!ok && err.contains("usage:"), "{err}");
    let (ok, out, _) = cli(&["schema", "dump"]);
    assert!(ok);
    for header in [
        "# pack.ron (schema 1)",
        "# data/tiles/<name>.ron (schema 1)",
        "# data/maps/<name>.ron (schema 1)",
        "# data/regions/<name>.ron",
        "# data/flags/<name>.ron",
        "# save (schema 9)",
        "# replay",
        "# protocol ops",
    ] {
        assert!(out.contains(header), "{header} missing from:\n{out}");
    }
}

/// `world tidy` (M8.5 7e) lists the world-folder files no save under `.omnis/` names, and
/// deletes them only with `--yes`.
#[test]
fn world_tidy_lists_then_deletes_with_yes() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-world-tidy");
    let _ = std::fs::remove_dir_all(&root);
    let stray = root.join(".omnis/worlds/0123456789abcdef/manifest.ron");
    std::fs::create_dir_all(stray.parent().unwrap()).unwrap();
    std::fs::write(&stray, "(seed: 1, packs: [])").unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_omnis-cli"))
            .current_dir(&root)
            .args(args)
            .output()
            .expect("the binary runs");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let (ok, out) = run(&["world", "tidy"]);
    assert!(ok, "{out}");
    assert!(
        out.contains(".omnis/worlds/0123456789abcdef/manifest.ron"),
        "{out}"
    );
    assert!(out.contains("--yes"), "{out}");
    assert!(stray.is_file(), "listing deletes nothing");
    let (ok, out) = run(&["world", "tidy", "--yes"]);
    assert!(ok, "{out}");
    assert!(!stray.exists(), "{out}");
}

fn fixture(name: &str) -> PathBuf {
    repo().join("crates/omnis-sim/tests/packs").join(name)
}

fn copy(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// `migration draft` (M8.5 7h6): the `chain` fixture's 0.3.0 without its migrations, drafted
/// from 0.1.0: each item and spell gone as a `Remove` with the `Rename` to write instead, the
/// chest and the rats as comments; the draft loads once resealed.
#[test]
fn migration_draft_writes_each_name_gone_as_a_remove() {
    let new = scratch("cli-draft").join("chain");
    copy(&fixture("chain/v3"), &new);
    std::fs::remove_dir_all(new.join("migrations")).unwrap();
    let new_dir = new.to_str().unwrap();
    assert!(cli(&["pack", "seal", new_dir, "--version", "0.3.0"]).0);
    let deps = [
        "--pack",
        "packs/base",
        "--pack",
        "packs/test",
        "--pack",
        "crates/omnis-sim/tests/packs/objects",
    ];
    let from = "crates/omnis-sim/tests/packs/chain/v1";
    let mut args = vec!["migration", "draft", "--from", from, "--to", new_dir];
    args.extend(deps);
    let (ok, out, err) = cli(&args);
    assert!(ok, "{out}{err}");
    assert!(out.contains("0001.ron: 4 steps, 2 to decide"), "{out}");
    let text = std::fs::read_to_string(new.join("migrations/0001.ron")).unwrap();
    for line in [
        "to: \"0.3.0\"",
        "        // or: Rename(kind: item, from: \"chain:item:old_sword\", to: \"<its new id>\"),",
        "        Remove(kind: item, id: \"chain:item:old_sword\"),",
        "        Remove(kind: item, id: \"chain:item:junk\"),",
        "        Remove(kind: spell, id: \"chain:spell:fading\"),",
        "        Remove(kind: spell, id: \"chain:spell:old_bolt\"),",
        "        // encounter 'rats' is gone from chain:map:hall; if it was renamed:",
        "        // RenameObject(map: \"chain:map:hall\", from: \"chest_1\", to: \"<its new id>\"),",
    ] {
        assert!(text.contains(line), "{line}\n{text}");
    }
    assert!(
        !text.contains("chain:item:sword\""),
        "never a guessed rename: {text}"
    );
    let (ok, _, err) = cli(&args);
    assert!(!ok, "the pack changed: it must be resealed first");
    assert!(err.contains("reseal"), "{err}");
    assert!(cli(&["pack", "seal", new_dir]).0);
    let mut validate = vec!["validate", "packs/base", "packs/test"];
    validate.push("crates/omnis-sim/tests/packs/objects");
    validate.push(new_dir);
    let (ok, out, err) = cli(&validate);
    assert!(ok, "the draft loads: {out}{err}");
}

/// `save check` (M8.5 7h6): a save made on `chain` 0.1.0 holding the old sword, examined on
/// 0.3.0, asks for its migrations and loads with them; on packs without `chain` it does not
/// load, and the command fails.
#[test]
fn save_check_prints_the_report_and_fails_when_the_save_does_not_load() {
    let root = scratch("cli-save-check");
    let old = load_packs(&[
        &repo().join("packs/base"),
        &repo().join("packs/test"),
        &fixture("objects"),
        &fixture("chain/v1"),
    ])
    .unwrap();
    let mut world = omnis_sim::World::new(&old, 3, Settings::default()).unwrap();
    let sword = old.registry.items.get("chain:item:old_sword").unwrap();
    world.party.inventory.push((sword, 1));
    for file in omnis_sim::save::save_files(&world, &old, "saves/a.ron").unwrap() {
        let path = root.join(&file.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, file.text).unwrap();
    }
    let run = |chain: bool| {
        let mut args = vec![
            "save".to_owned(),
            "check".to_owned(),
            "saves/a.ron".to_owned(),
        ];
        let mut packs = vec![
            repo().join("packs/base"),
            repo().join("packs/test"),
            fixture("objects"),
        ];
        if chain {
            packs.push(fixture("chain/v3"));
        }
        for p in packs {
            args.push("--pack".into());
            args.push(p.to_string_lossy().into_owned());
        }
        let out = Command::new(env!("CARGO_BIN_EXE_omnis-cli"))
            .current_dir(&root)
            .args(&args)
            .output()
            .expect("the binary runs");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let (ok, out) = run(true);
    assert!(ok, "{out}");
    for line in [
        "verdict: NeedsMigration",
        "pack chain: saved 0.1.0, loaded 0.3.0",
        "migration chain 0001.ron (to 0.2.0): item 'chain:item:old_sword' is now 'chain:item:mid_sword'",
    ] {
        assert!(out.contains(line), "{line}\n{out}");
    }
    let (ok, out) = run(false);
    assert!(!ok, "{out}");
    assert!(out.contains("verdict: Unresolved"), "{out}");
    assert!(
        out.contains("missing: item 'chain:item:old_sword' in party.inventory"),
        "{out}"
    );
}

/// A race, class, background or region gone cannot be removed: the draft leaves it open, a
/// commented `Rename` with why, and counts it.
#[test]
fn migration_draft_leaves_what_cannot_be_removed_open() {
    let load = |chain: &str| {
        load_packs(&[
            &repo().join("packs/base"),
            &repo().join("packs/test"),
            &fixture("objects"),
            &fixture(chain),
        ])
        .unwrap()
    };
    let (mut old, mut new) = (load("chain/v1"), load("chain/v3"));
    old.registry.races.intern("chain:race:gone");
    new.migrations.clear();
    let draft = omnis_cli::draft::draft("chain", &old, &new).unwrap();
    assert!(
        draft.text.contains(
            "        // A race cannot be removed: rename or replace it.\n        // Rename(kind: race, from: \"chain:race:gone\""
        ),
        "{}",
        draft.text
    );
    assert!(!draft.text.contains("Remove(kind: race"), "{}", draft.text);
    assert_eq!((draft.steps, draft.open), (4, 3));
}
