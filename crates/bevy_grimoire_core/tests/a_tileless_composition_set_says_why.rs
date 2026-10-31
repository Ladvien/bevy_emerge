//! A composition set with no tile in it is refused by the composition grammar, by name.
//!
//! E-083 (#132): a game's compositions were all `Anchored` (whole houses, placed as a group), so the
//! composition grammar held `Empty` alone and the solver's refusal was the one written for placements:
//! "nothing has been placed yet". The author could not act on it. The refusal must come from the
//! grammar, name each composition it could not use and why, and say what a tile is.

use bevy_grimoire_core::composition::{Composition, Envelope};
use bevy_grimoire_core::grammar::from_compositions;
use bevy_grimoire_core::library::Library;

fn group(id: &str, envelope: Envelope) -> Composition {
    Composition {
        id: id.to_owned(),
        envelope,
        members: Vec::new(),
        locations: Vec::new(),
        note: None,
    }
}

#[test]
fn a_set_of_anchored_groups_is_refused_with_each_reason() {
    let comps = vec![
        group("town/house_cottage", Envelope::Anchored),
        group("town/house_hall", Envelope::Anchored),
        // Bounded, but two cells across on a one-metre grid: also not a tile.
        group("town/yard", Envelope::Bounded { size: (2.0, 1.0, 2.0) }),
    ];
    let err = from_compositions(&comps, &Library::default(), 1, 1.0, |_, _, _| true)
        .expect_err("a set with no tile has nothing to solve with, so it is refused");

    for id in ["town/house_cottage", "town/house_hall", "town/yard"] {
        assert!(err.contains(id), "the refusal names `{id}`: {err}");
    }
    assert!(err.contains("anchored"), "it says why the anchored groups were left out: {err}");
    assert!(err.contains("Bounded"), "it says what a tile is: {err}");
    assert!(
        !err.contains("nothing has been placed"),
        "the cause is the composition set, not the map's placements: {err}"
    );
}

#[test]
fn many_reasons_are_counted_rather_than_listed_without_end() {
    let comps: Vec<Composition> =
        (0..16).map(|i| group(&format!("house_{i:02}"), Envelope::Anchored)).collect();
    let err = from_compositions(&comps, &Library::default(), 1, 1.0, |_, _, _| true)
        .expect_err("sixteen anchored groups are no tile");
    assert!(err.contains("none of the 16"), "the total is said: {err}");
    assert!(err.contains("house_00") && err.contains("house_04"), "the first five are named: {err}");
    assert!(!err.contains("house_05"), "the sixth is counted, not named: {err}");
    assert!(err.contains("and 11 more"), "the rest are counted: {err}");
}

#[test]
fn a_project_with_no_compositions_is_refused_too() {
    let err = from_compositions(&[], &Library::default(), 1, 1.0, |_, _, _| true)
        .expect_err("no compositions, no tile");
    assert!(err.contains("no compositions"), "{err}");
}
