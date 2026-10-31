//! **`repetition::vary` never moves a long piece onto cells it did not cover** (#186).
//!
//! `vary` ends a clutter repeat by turning a copy. A placement's `at` is its centre, so a half turn
//! leaves any footprint on the cells it covered, and a quarter turn does only for a square one: a
//! 1 m × 2 m bench turned 90° covers 2 m × 1 m, into the piece beside it. A measured piece whose
//! floor footprint (after its tip) is not square is given the half turn alone, and a copy the half
//! turn cannot end is counted as left, not turned off its cells.

use bevy_grimoire_core::descriptor::{Descriptor, Extent};
use bevy_grimoire_core::library::{Library, LIBRARY_VERSION};
use bevy_grimoire_core::map::Placed;
use bevy_grimoire_core::repetition::{vary, RepetitionRules};

fn piece(id: &str, footprint: (f32, f32), height: f32) -> Descriptor {
    Descriptor {
        id: id.to_owned(),
        kind: vec!["furniture".to_owned()],
        extent: Extent {
            footprint: Some(footprint),
            height: Some(height),
        },
        ..Descriptor::default()
    }
}

fn library() -> Library {
    Library {
        version: LIBRARY_VERSION,
        note: None,
        descriptors: vec![
            piece("bench", (1.0, 2.0), 0.5),
            piece("stool", (0.6, 0.6), 0.5),
            // Long as it stands, square once laid on its side about x: (1, 2) × 1 m tall tips to a
            // 1 × 1 m floor.
            piece("crate", (1.0, 2.0), 1.0),
        ],
    }
}

/// `n` copies of `descriptor` side by side, a metre apart along x, all at yaw 0 with `tip`.
fn row(descriptor: &str, n: usize, tip: (u8, u8)) -> Vec<Placed> {
    (0..n)
        .map(|i| Placed {
            id: format!("{descriptor}_{i}"),
            descriptor: descriptor.to_owned(),
            at: (i as f32, 0.0),
            yaw: 0.0,
            tip,
            ..Placed::default()
        })
        .collect()
}

#[test]
fn a_long_bench_is_only_ever_turned_a_half_turn() {
    let rows = row("bench", 3, (0, 0));
    let v = vary(&rows, &library(), &RepetitionRules::default());
    for &(i, yaw) in &v.turned {
        assert_eq!(
            (yaw - rows[i].yaw).rem_euclid(180.0),
            0.0,
            "{} is 1 m × 2 m: a turn to {yaw}° puts it on cells it never covered: {v:?}",
            rows[i].id
        );
    }
    // The second bench takes the half turn; the third has no turn left that keeps its cells, so it
    // and the first still look alike and both are counted.
    assert_eq!(v.turned, vec![(1, 180.0)], "{v:?}");
    assert_eq!(v.left, 2, "{v:?}");
}

#[test]
fn a_square_piece_still_takes_every_turn() {
    let rows = row("stool", 4, (0, 0));
    let v = vary(&rows, &library(), &RepetitionRules::default());
    assert_eq!(v.turned, vec![(1, 180.0), (2, 90.0), (3, 270.0)], "{v:?}");
    assert_eq!(v.left, 0, "{v:?}");
}

#[test]
fn the_floor_a_tipped_piece_covers_decides_whether_it_is_square() {
    // Standing, the crate is long and keeps to the half turn.
    let standing = vary(&row("crate", 3, (0, 0)), &library(), &RepetitionRules::default());
    assert_eq!(standing.turned, vec![(1, 180.0)], "{standing:?}");
    // On its side about x it covers 1 m × 1 m, so a quarter turn keeps it on its cells.
    let lying = vary(&row("crate", 4, (1, 0)), &library(), &RepetitionRules::default());
    assert_eq!(lying.turned, vec![(1, 180.0), (2, 90.0), (3, 270.0)], "{lying:?}");
    assert_eq!(lying.left, 0, "{lying:?}");
}
