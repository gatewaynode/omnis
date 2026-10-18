//! Writing and reading a save's files under a root (M8.5 7d): each file written whole or not at
//! all, a file named by its content left as it is, and every path held under the root; listing
//! and deleting for the tidy (7e), never through a link or out of the root.

use crate::common::scratch;
use omnis_data::ron_io::{list_under, read_under, remove_under, size_under, write_files};

#[test]
fn files_are_written_whole_under_the_root_and_kept_when_named_by_content() {
    let root = scratch("files-write");
    write_files(
        &root,
        &[
            (".omnis/worlds/a/base.ron", "(one)", true),
            (".omnis/quick.ron", "(save)", false),
        ],
    )
    .unwrap();
    assert_eq!(read_under(&root, ".omnis/quick.ron").unwrap(), "(save)");
    write_files(
        &root,
        &[
            (".omnis/worlds/a/base.ron", "(two)", true),
            (".omnis/quick.ron", "(again)", false),
        ],
    )
    .unwrap();
    assert_eq!(
        read_under(&root, ".omnis/worlds/a/base.ron").unwrap(),
        "(one)",
        "a file named by its content stays"
    );
    assert_eq!(
        read_under(&root, ".omnis/quick.ron").unwrap(),
        "(again)",
        "replaced"
    );
    let left: Vec<_> = std::fs::read_dir(root.join(".omnis"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(
        left.iter().all(|n| !n.contains("tmp")),
        "no temp file left: {left:?}"
    );
}

#[test]
fn a_path_outside_the_root_is_refused_both_ways() {
    let root = scratch("files-refuse");
    for bad in ["../x.ron", "/tmp/x.ron", "a/../../x.ron", "x.txt", ""] {
        assert!(write_files(&root, &[(bad, "()", false)]).is_err(), "{bad}");
        assert!(read_under(&root, bad).is_err(), "{bad}");
    }
}

#[test]
fn listing_finds_ron_files_and_never_follows_a_link_or_leaves_the_root() {
    let root = scratch("files-list");
    write_files(
        &root,
        &[
            (".omnis/quick.ron", "(save)", false),
            (".omnis/worlds/a/manifest.ron", "(m)", true),
            (
                ".omnis/worlds/a/regions/p/r/0123456789abcdef.ron",
                "(r)",
                true,
            ),
        ],
    )
    .unwrap();
    std::fs::write(root.join(".omnis/notes.txt"), "not ron").unwrap();
    #[cfg(unix)]
    {
        let outside = scratch("files-list-outside");
        std::fs::write(outside.join("secret.ron"), "(secret)").unwrap();
        std::os::unix::fs::symlink(&outside, root.join(".omnis/away")).unwrap();
        std::os::unix::fs::symlink(outside.join("secret.ron"), root.join(".omnis/link.ron"))
            .unwrap();
    }
    assert_eq!(
        list_under(&root, ".omnis").unwrap(),
        [
            ".omnis/quick.ron",
            ".omnis/worlds/a/manifest.ron",
            ".omnis/worlds/a/regions/p/r/0123456789abcdef.ron",
        ]
    );
    assert!(list_under(&root, "nothing-here").unwrap().is_empty());
    for bad in ["../x", "/etc", ""] {
        assert!(list_under(&root, bad).is_err(), "{bad}");
    }
    assert_eq!(size_under(&root, ".omnis/quick.ron").unwrap(), 6);
}

#[test]
fn deleting_takes_a_file_only_never_a_folder_a_link_or_a_climb() {
    let root = scratch("files-remove");
    write_files(&root, &[(".omnis/worlds/a/manifest.ron", "(m)", true)]).unwrap();
    std::fs::create_dir_all(root.join(".omnis/dir.ron")).unwrap();
    assert!(remove_under(&root, ".omnis/dir.ron").is_err(), "a folder");
    assert!(remove_under(&root, "../outside.ron").is_err(), "a climb");
    #[cfg(unix)]
    {
        let outside = scratch("files-remove-outside");
        std::fs::write(outside.join("keep.ron"), "(keep)").unwrap();
        std::os::unix::fs::symlink(outside.join("keep.ron"), root.join(".omnis/link.ron")).unwrap();
        assert!(remove_under(&root, ".omnis/link.ron").is_err(), "a link");
        assert!(outside.join("keep.ron").is_file());
    }
    remove_under(&root, ".omnis/worlds/a/manifest.ron").unwrap();
    assert!(!root.join(".omnis/worlds/a/manifest.ron").exists());
}
