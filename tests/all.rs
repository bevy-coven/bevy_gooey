//! Every integration test in one binary.
//!
//! Each file in `tests/` used to be its own test executable, and each linked its own copy of Bevy: 650
//! to 790 MB apiece, 24 GB for one relink. They are modules of this one now (`autotests = false` in
//! `Cargo.toml`), so a test is `all::<file>::<name>`. A new file in `tests/` is declared here, or
//! `every_test_file_is_a_module` fails: an undeclared file is never compiled and its tests never run.

mod access;
mod anchor;
mod badges;
mod binding;
mod cancel;
mod card;
mod choice;
mod color;
mod confirm;
mod context_menu;
mod controls;
mod cooldown;
mod density;
mod desk;
mod dialog;
mod dialog_look;
mod dock;
mod double_press;
mod drag;
mod elevation;
mod field_problem;
mod hold;
mod icon;
mod inspector;
mod large_text;
mod leaf;
mod lines;
mod list;
mod menu;
mod meter;
mod meter_motion;
mod motion;
mod motion_trace;
mod nav;
mod number;
mod options;
mod palette;
mod pointer;
mod popover;
mod presence;
mod primary_button;
mod progress;
mod prompts;
mod radial;
mod readme;
mod region;
mod scale;
mod scope;
mod section;
mod seed;
mod select;
mod serde;
mod shortcut;
mod side_tabs;
mod skin;
mod slots;
mod split;
mod state_roles;
mod status_bar;
mod table;
mod tabs;
mod testing;
mod text;
mod text_area;
mod toast;
mod toast_actions;
mod tokens;
mod tones;
mod toolbar;
mod tooltip;
mod tree;
mod type_select;

/// The files in `tests/` and the `mod` lines above are the same set.
#[test]
fn every_test_file_is_a_module() {
    const THIS_FILE: &str = include_str!("all.rs");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");
    let entries = std::fs::read_dir(dir).expect("tests/ is readable");
    let mut missing = Vec::new();
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a UTF-8 file name");
        if stem != "all" && !THIS_FILE.lines().any(|l| l == format!("mod {stem};")) {
            missing.push(stem.to_owned());
        }
    }
    assert!(
        missing.is_empty(),
        "tests/{{{}}}.rs is not declared as a `mod` in tests/all.rs, so it never compiles or runs",
        missing.join(",")
    );
}
