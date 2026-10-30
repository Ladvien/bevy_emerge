//! **`repetition::vary` turns the clutter copies a player would see as stamped** (#143).
//!
//! `repeats` names the visible repeats; `vary` ends the clutter ones by turning copies (the cheapest
//! variation the repeat rule itself names), never touches structure, keeps the first copy of each
//! look as the author left it, and says how many copies four turns could not tell apart.

use bevy_emerge_core::descriptor::Descriptor;
use bevy_emerge_core::library::{Library, LIBRARY_VERSION};
use bevy_emerge_core::map::Placed;
use bevy_emerge_core::repetition::{repeats, vary, RepetitionRules, Role};

fn piece(id: &str, kind: &str) -> Descriptor {
    Descriptor {
        id: id.to_owned(),
        kind: vec![kind.to_owned()],
        ..Descriptor::default()
    }
}

fn library() -> Library {
    Library {
        version: LIBRARY_VERSION,
        note: None,
        descriptors: vec![piece("barrel", "container"), piece("fence", "fence")],
    }
}

fn at(id: &str, descriptor: &str, x: f32, yaw: f32) -> Placed {
    Placed {
        id: id.to_owned(),
        descriptor: descriptor.to_owned(),
        at: (x, 0.0),
        yaw,
        ..Placed::default()
    }
}

/// `n` barrels a metre apart and a run of ten fence pieces, all at one yaw.
fn yard(n: usize) -> Vec<Placed> {
    let mut rows: Vec<Placed> = (0..n)
        .map(|i| at(&format!("barrel_{i}"), "barrel", i as f32, 0.0))
        .collect();
    rows.extend((0..10).map(|i| at(&format!("fence_{i}"), "fence", 40.0 + i as f32 * 0.5, 0.0)));
    rows
}

fn applied(rows: &[Placed], turned: &[(usize, f32)]) -> Vec<Placed> {
    let mut rows = rows.to_vec();
    for &(i, yaw) in turned {
        rows[i].yaw = yaw;
    }
    rows
}

#[test]
fn three_barrels_stop_repeating_and_the_fence_is_left_alone() {
    let rules = RepetitionRules::default();
    let rows = yard(3);
    let v = vary(&rows, &library(), &rules);
    assert_eq!(v.left, 0, "{v:?}");
    assert!(v.turned.iter().all(|&(i, _)| rows[i].descriptor == "barrel"), "only clutter turns: {v:?}");
    assert!(!v.turned.iter().any(|&(i, _)| i == 0), "the first barrel keeps its yaw: {v:?}");
    let after = applied(&rows, &v.turned);
    let report = repeats(&after, &library(), &rules);
    assert!(report.iter().all(|r| r.role == Role::Structure), "no clutter repeat is left: {report:?}");
    assert_eq!(report.len(), 1, "the fence still repeats, by design: {report:?}");
    // Every turn is a right angle, so a footprint stays on the grid's cells.
    for &(i, yaw) in &v.turned {
        assert_eq!((yaw - rows[i].yaw).rem_euclid(90.0), 0.0, "{yaw}");
    }
}

#[test]
fn more_copies_than_four_turns_are_counted_as_left() {
    let v = vary(&yard(6), &library(), &RepetitionRules::default());
    // Four looks for six barrels within 6 m of each other: two still match one.
    assert_eq!(v.turned.len(), 3, "{v:?}");
    assert!(v.left > 0, "{v:?}");
}

#[test]
fn a_map_with_nothing_repeating_or_only_structure_turns_nothing() {
    let rules = RepetitionRules::default();
    let apart = vec![at("a", "barrel", 0.0, 0.0), at("b", "barrel", 20.0, 0.0)];
    assert!(vary(&apart, &library(), &rules).turned.is_empty());
    let fences = yard(0);
    assert!(vary(&fences, &library(), &rules).turned.is_empty());
    // An author who calls barrels structure keeps their row as built.
    let all_structure = RepetitionRules {
        structure: vec!["container".to_owned(), "fence".to_owned()],
        ..RepetitionRules::default()
    };
    let v = vary(&yard(3), &library(), &all_structure);
    assert!(v.turned.is_empty() && v.left == 0, "{v:?}");
}

#[test]
fn a_copy_already_turned_by_the_author_is_not_turned_again() {
    let rows = vec![at("a", "barrel", 0.0, 0.0), at("b", "barrel", 1.0, 90.0)];
    let v = vary(&rows, &library(), &RepetitionRules::default());
    assert!(v.turned.is_empty(), "{v:?}");
}
