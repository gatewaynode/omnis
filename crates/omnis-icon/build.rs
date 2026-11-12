//! Compiles the Lucide icon set into the crate: every `assets/svg-icons/icons/*.svg` as its name
//! and text, sorted by name, and the set's licence (ISC asks that its notice travel with every
//! copy). Nothing is read at run time.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("the manifest dir"))
        .join("../../assets/svg-icons");
    let icons = root.join("icons");
    let licence = root.join("LICENSE");
    println!("cargo:rerun-if-changed={}", icons.display());
    println!("cargo:rerun-if-changed={}", licence.display());
    let Ok(entries) = std::fs::read_dir(&icons) else {
        panic!(
            "omnis-icon: the Lucide icon set is expected at {} (assets/svg-icons/icons/*.svg)",
            icons.display()
        );
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "svg"))
        .filter_map(|p| {
            let name = p.file_stem()?.to_str()?.to_owned();
            Some((name, p))
        })
        .collect();
    files.sort();
    let mut out = String::new();
    writeln!(
        out,
        "/// Every icon of the set, by name, as its SVG text; sorted by name.\n\
         static ICONS: &[(&str, &str)] = &["
    )
    .unwrap();
    for (name, path) in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("omnis-icon: reading {}: {e}", path.display()));
        assert!(
            !text.contains("\"###"),
            "{}: an unexpected delimiter",
            path.display()
        );
        writeln!(out, "    ({name:?}, r###\"{text}\"###),").unwrap();
    }
    writeln!(out, "];").unwrap();
    let licence_text = std::fs::read_to_string(&licence)
        .unwrap_or_else(|e| panic!("omnis-icon: reading {}: {e}", licence.display()));
    writeln!(
        out,
        "/// The set's licence (ISC), carried with every copy as it asks.\n\
         pub const LICENSE: &str = r###\"{licence_text}\"###;"
    )
    .unwrap();
    let dest = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("catalogue.rs");
    std::fs::write(&dest, out).unwrap_or_else(|e| panic!("writing {}: {e}", dest.display()));
}
