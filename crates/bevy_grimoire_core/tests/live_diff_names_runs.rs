//! **The live diff names a run once** (#115 acceptance 5): editing a run changes its rows
//! `<id>/0..n`, and a reader told "ring/0 ring/1 ... ring/9 changed" cannot see it was one edit.

use bevy_grimoire_core::descriptor::{Descriptor, Extent};
use bevy_grimoire_core::library::{LIBRARY_VERSION, Library};
use bevy_grimoire_core::live::diff_maps;
use bevy_grimoire_core::map::{Map, Placed};
use bevy_grimoire_core::run::{Run, RunPlaced};

fn library() -> Library {
    Library {
        version: LIBRARY_VERSION,
        note: None,
        descriptors: vec![Descriptor {
            id: "kit/fence".to_owned(),
            extent: Extent { footprint: Some((2.064, 0.12)), height: Some(0.838) },
            ..Default::default()
        }],
    }
}

fn run(to: f32) -> RunPlaced {
    RunPlaced {
        id: "ring".to_owned(),
        run: Run {
            descriptor: "kit/fence".to_owned(),
            pitch: None,
            from: (0.0, 0.0),
            to: (to, 0.0),
            corners: Vec::new(),
            gaps: Vec::new(),
        },
        owned: false,
        owned_because: None,
    }
}

/// What the game's world holds: the map with its runs expanded into rows and still listed.
fn world(runs: Vec<RunPlaced>, hand: &[&str]) -> Map {
    let map = Map {
        name: "yard".to_owned(),
        bounds: (100.0, 10.0, 100.0),
        placements: hand
            .iter()
            .map(|id| Placed { id: (*id).to_owned(), descriptor: "kit/fence".to_owned(), ..Placed::default() })
            .collect(),
        runs,
        ..Map::default()
    };
    map.expanded(&[], &library()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn moving_a_run_end_names_the_run_once() {
    let old = world(vec![run(19.0)], &["gate"]);
    let new = world(vec![run(30.0)], &["gate"]);
    assert_ne!(old.placements.len(), new.placements.len());
    let diff = diff_maps(&old, &new);
    assert_eq!(diff.changed, vec!["ring".to_owned()], "{diff:?}");
    assert!(diff.added.is_empty() && diff.removed.is_empty(), "{diff:?}");
}

#[test]
fn adding_or_removing_a_run_names_it_once() {
    let none = world(Vec::new(), &["gate"]);
    let some = world(vec![run(19.0)], &["gate"]);
    let added = diff_maps(&none, &some);
    assert_eq!(added.added, vec!["ring".to_owned()], "{added:?}");
    assert!(added.changed.is_empty() && added.removed.is_empty());
    let removed = diff_maps(&some, &none);
    assert_eq!(removed.removed, vec!["ring".to_owned()], "{removed:?}");
    assert!(removed.changed.is_empty() && removed.added.is_empty());
}

#[test]
fn a_hand_row_beside_a_run_is_still_named_by_its_own_id() {
    let old = world(vec![run(19.0)], &["gate"]);
    let mut new = world(vec![run(19.0)], &["gate", "post"]);
    new.placements.iter_mut().find(|p| p.id == "gate").unwrap().yaw = 90.0;
    let diff = diff_maps(&old, &new);
    assert_eq!(diff.added, vec!["post".to_owned()]);
    assert_eq!(diff.changed, vec!["gate".to_owned()]);
    assert!(diff.removed.is_empty());
}
