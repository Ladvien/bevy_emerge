//! **Kit stress test: can the kit loop a hallway back** (#160, lens #4).
//!
//! `adjacency::dead_ends` proves every opening can be capped; it cannot see a kit that never turns.
//! `adjacency::ring` builds the smallest closed loop, four pieces around a point with every inner
//! seam declared and passing `faults`, or says there is none.

use bevy_emerge_core::adjacency::{faults, ring};
use bevy_emerge_core::descriptor::{Descriptor, SubCell, Subgrid};
use bevy_emerge_core::grid::SNAP;
use bevy_emerge_core::library::Library;

/// A piece three cells by three on the floor and one high, at one lattice cell per authoring cell,
/// with `hall` on the middle cell of each named face (x grows east, z grows south).
fn piece(id: &str, faces: &[char]) -> Descriptor {
    let cells = faces
        .iter()
        .map(|f| match f {
            'N' => (1, 0, 0),
            'E' => (2, 0, 1),
            'S' => (1, 0, 2),
            _ => (0, 0, 1),
        })
        .map(|at| SubCell {
            at,
            edge: Some("hall".to_owned()),
            ..SubCell::default()
        })
        .collect();
    let mut d = Descriptor {
        id: id.to_owned(),
        subgrid: Some(Subgrid { cells }),
        ..Descriptor::default()
    };
    d.extent.footprint = Some((3.0 * SNAP, 3.0 * SNAP));
    d.extent.height = Some(SNAP);
    d
}

fn kit(pieces: Vec<Descriptor>) -> Library {
    Library {
        descriptors: pieces,
        ..Library::default()
    }
}

#[test]
fn a_kit_of_straights_cannot_close_a_loop() {
    // Every face of a straight has a partner (another straight, or itself turned), so dead_ends
    // passes it; a straight never turns, so no ring.
    assert_eq!(ring(&kit(vec![piece("straight", &['E', 'W'])]), 1), None);
}

#[test]
fn a_corner_closes_the_loop_and_the_ring_passes_the_map_check() {
    let lib = kit(vec![
        piece("straight", &['E', 'W']),
        piece("corner", &['E', 'S']),
    ]);
    let found = ring(&lib, 1).expect("four corners turned close a loop");
    assert!(
        found.pieces.iter().all(|(p, _)| p == "corner"),
        "a straight cannot sit in the smallest loop: {:?}",
        found.pieces
    );
    // Four different turns: each corner opens toward the middle.
    let mut turns: Vec<u8> = found.pieces.iter().map(|(_, q)| *q).collect();
    turns.sort_unstable();
    assert_eq!(turns, vec![0, 1, 2, 3], "{:?}", found.pieces);
    assert_eq!(found.map.placements.len(), 4);
    assert_eq!(faults(&found.map, &lib, 1), Vec::new());
}

#[test]
fn a_corner_whose_sockets_disagree_once_turned_cannot_close_a_loop() {
    // One socket is `hall`, the other `door`: turned, each meets the other kind, so no ring.
    let mut odd = piece("odd_corner", &['E', 'S']);
    if let Some(g) = odd.subgrid.as_mut() {
        g.cells[1].edge = Some("door".to_owned());
    }
    assert_eq!(
        ring(&kit(vec![piece("straight", &['E', 'W']), odd]), 1),
        None
    );
}

#[test]
fn a_kit_with_no_tokens_has_no_ring() {
    assert_eq!(ring(&kit(vec![piece("blank", &[])]), 1), None);
}
