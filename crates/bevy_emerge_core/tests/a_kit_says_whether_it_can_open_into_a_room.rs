//! **Kit stress test: can the kit open a hallway out into a room** (#162, lens #4).
//!
//! `adjacency::ring` proves a kit can loop a hallway back; a corridor kit of straights and corners
//! passes it and still never makes a room, because nothing in it meets a neighbour on three or four
//! sides. `adjacency::room` builds the smallest room with a middle, three rows of three pieces with
//! every inner seam declared and passing `faults`, or says there is none.

use bevy_emerge_core::adjacency::{faults, ring, room};
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

fn corridor() -> Vec<Descriptor> {
    vec![piece("straight", &['E', 'W']), piece("corner", &['E', 'S'])]
}

#[test]
fn a_corridor_kit_loops_back_but_cannot_open_into_a_room() {
    let lib = kit(corridor());
    assert!(ring(&lib, 1).is_some(), "the corridor kit closes a loop");
    assert_eq!(room(&lib, 1), None);
}

#[test]
fn walls_without_a_middle_piece_cannot_make_a_room() {
    // Every edge of the room can be a wall, but the middle meets a neighbour on all four sides.
    let mut pieces = corridor();
    pieces.push(piece("wall", &['E', 'S', 'W']));
    assert_eq!(room(&kit(pieces), 1), None);
}

#[test]
fn a_middle_piece_opens_the_kit_into_a_room_that_passes_the_map_check() {
    let mut pieces = corridor();
    pieces.push(piece("wall", &['E', 'S', 'W']));
    pieces.push(piece("middle", &['N', 'E', 'S', 'W']));
    let lib = kit(pieces);
    let found = room(&lib, 1).expect("a piece open on four sides opens the kit into a room");
    assert_eq!(
        found.pieces[1][1].0, "middle",
        "only the four-sided piece can stand in the middle: {:?}",
        found.pieces
    );
    assert_eq!(found.map.placements.len(), 9);
    assert_eq!(faults(&found.map, &lib, 1), Vec::new());
    let middle = found
        .map
        .placements
        .iter()
        .find(|p| p.id == "middle")
        .expect("the map names its middle placement `middle`");
    assert_eq!(
        middle.at,
        (3.0 * SNAP, 3.0 * SNAP),
        "the middle is one footprint in from each side"
    );
}

#[test]
fn a_kit_of_many_corridor_pieces_answers_without_trying_every_arrangement() {
    // Six look-alikes each of the straight, the corner and the wall: 72 options, and no piece for
    // the middle. A search that tries every arrangement of the first cells before finding the
    // middle empty passed 30 million seam checks at four look-alikes (#162 review, a model of
    // 273b5b0's search); pruning each cell's candidates first answers at once.
    let mut pieces = Vec::new();
    for c in 0..6 {
        pieces.push(piece(&format!("straight_{c}"), &['E', 'W']));
        pieces.push(piece(&format!("corner_{c}"), &['E', 'S']));
        pieces.push(piece(&format!("wall_{c}"), &['E', 'S', 'W']));
    }
    assert_eq!(room(&kit(pieces), 1), None);
}

#[test]
fn a_middle_whose_one_socket_is_off_centre_cannot_make_a_room() {
    // The north socket moves to the north-west corner cell (so the west face shows it too); turned
    // half way it sits on the south-east one, so two of these never meet across a seam, and nothing
    // else in the kit presents an off-centre socket. The middle cell has no piece to stand in it.
    let mut odd = piece("odd_middle", &['N', 'E', 'S', 'W']);
    if let Some(g) = odd.subgrid.as_mut() {
        g.cells[0].at = (0, 0, 0);
    }
    let mut pieces = corridor();
    pieces.push(piece("wall", &['E', 'S', 'W']));
    pieces.push(odd);
    assert_eq!(room(&kit(pieces), 1), None);
}

#[test]
fn a_kit_with_no_tokens_has_no_room() {
    assert_eq!(room(&kit(vec![piece("blank", &[])]), 1), None);
}
