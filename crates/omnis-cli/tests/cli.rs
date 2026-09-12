//! The binary's subcommands on the real test pack, the bad-pack corpus, and the golden replay.

use omnis_data::load_packs;
use omnis_sim::Settings;
use omnis_sim::command::parse_script;
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
    let (ok, out, _) = cli(&["validate", "packs/test"]);
    assert!(ok);
    assert!(out.starts_with("ok: 1 packs, 2 maps, 2 tilesets"), "{out}");
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
    let (ok, out, _) = cli(&["map", "text", "test:map:meadow", "--pack", "packs/test"]);
    assert!(ok);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 65);
    assert_eq!(lines[2 * 16 + 1].chars().nth(2 * 16 + 1), Some('^'));
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
    let commands = parse_script(text).unwrap();
    let expected = omnis_sim::replay::run(&data, 9, Settings::default(), &commands).unwrap();
    assert_eq!(
        lines.last().copied(),
        Some(format!("fingerprint: {expected:016x}").as_str())
    );

    std::fs::write(&script, "forward\nfly\n").unwrap();
    let (ok, _, err) = cli(&["play", "--script", script.to_str().unwrap()]);
    assert!(!ok);
    assert!(err.contains("line 2: unknown command 'fly'"), "{err}");
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
fn nonsense_prints_usage_and_schema_dump_prints_sections() {
    let (ok, _, err) = cli(&["dance"]);
    assert!(!ok && err.contains("usage:"), "{err}");
    let (ok, out, _) = cli(&["schema", "dump"]);
    assert!(ok);
    for header in [
        "# pack.ron (schema 1)",
        "# data/tiles/<name>.ron (schema 1)",
        "# data/maps/<name>.ron (schema 1)",
        "# save (schema 2)",
        "# replay",
        "# protocol ops",
    ] {
        assert!(out.contains(header), "{header} missing from:\n{out}");
    }
}
