//! The committed test pack is exactly what the bake specs produce, and the baked sprites are
//! well-formed PNGs inside the viewport.

use omnis_cli::bake::{BakeSpec, bake_to, load_png};
use omnis_data::ron_io::read_ron;
use omnis_data::{SlotKind, Tileset, load_packs};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn committed_tilesets_match_a_fresh_bake() {
    for name in ["dungeon", "outdoor"] {
        let out = scratch(&format!("bake-{name}"));
        let spec = repo().join(format!("packs/test/bake/{name}.ron"));
        let report = bake_to(&spec, &out).unwrap_or_else(|e| panic!("{e}"));
        assert!(report.sprites > 0);
        let fresh: Tileset = read_ron(&report.tileset_file, Path::new("fresh")).unwrap();
        let committed: Tileset = read_ron(
            &repo().join(format!("packs/test/data/tiles/{name}.ron")),
            Path::new("committed"),
        )
        .unwrap();
        assert_eq!(
            fresh, committed,
            "{name}: re-run `omnis-cli tileset bake packs/test/bake/{name}.ron` and commit the result"
        );
        let mut files = 0;
        for surface in fresh.surfaces.values() {
            for slot in &surface.slots {
                let image = load_png(&out.join(&slot.path)).unwrap_or_else(|e| panic!("{e}"));
                files += 1;
                assert!(
                    slot.x >= 0 && slot.y >= 0,
                    "{}: placed inside the canvas",
                    slot.path
                );
                let right = u32::try_from(slot.x).unwrap() + image.width;
                let bottom = u32::try_from(slot.y).unwrap() + image.height;
                assert!(
                    right <= u32::from(fresh.viewport.0) && bottom <= u32::from(fresh.viewport.1),
                    "{}: {right}x{bottom} exceeds the viewport",
                    slot.path
                );
                let opaque = image
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[3] != 0)
                    .count();
                assert!(
                    opaque > 0,
                    "{}: an empty sprite would not have been written",
                    slot.path
                );
                let committed_png = repo().join("packs/test").join(&slot.path);
                assert!(
                    committed_png.exists(),
                    "{} is missing from the pack",
                    slot.path
                );
            }
        }
        assert_eq!(files, report.sprites);
        assert!(
            fresh
                .surfaces
                .values()
                .any(|s| s.kind == SlotKind::WallLeft)
        );
    }
}

#[test]
fn the_baked_pack_loads() {
    let data = load_packs(&[&repo().join("packs/test")]).unwrap_or_else(|r| panic!("{r}"));
    let dungeon = &data.tilesets[&data.registry.tilesets.get("test:tileset:dungeon").unwrap()];
    let spec: BakeSpec = read_ron(
        &repo().join("packs/test/bake/dungeon.ron"),
        Path::new("spec"),
    )
    .unwrap();
    assert_eq!(
        dungeon.viewport, spec.viewport,
        "baked for the spec's viewport"
    );
    assert!(dungeon.slot("wall", 0, 0).is_some());
    assert!(
        dungeon.slot("floor", 0, 0).is_some(),
        "the party's own floor strip exists"
    );
}
