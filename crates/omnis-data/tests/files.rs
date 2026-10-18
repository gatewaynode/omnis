//! Writing and reading a save's files under a root (M8.5 7d): each file written whole or not at
//! all, a file named by its content left as it is, and every path held under the root.

use crate::common::scratch;
use omnis_data::ron_io::{read_under, write_files};

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
