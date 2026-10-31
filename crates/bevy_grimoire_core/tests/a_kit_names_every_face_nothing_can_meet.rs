//! **Kit stress test: a kit names every face no piece of it can meet** (#142, lens #4).
//!
//! A socket nothing in the kit presents back is a dead end: a hallway that cannot be capped, a jamb
//! with no partner. `adjacency::dead_ends` tries every piece at every quarter turn against every
//! declared face, through the same seam check the map uses, and names the faces nothing meets.

use bevy_grimoire_core::adjacency::dead_ends;
use bevy_grimoire_core::descriptor::{Descriptor, SubCell, Subgrid};
use bevy_grimoire_core::grid::SNAP;
use bevy_grimoire_core::library::Library;
use bevy_grimoire_core::wfc::E;

/// A piece two cells by two on the floor and one high, at one lattice cell per authoring cell.
fn piece(id: &str, cells: &[((u32, u32, u32), &str)]) -> Descriptor {
    let mut d = Descriptor {
        id: id.to_owned(),
        subgrid: Some(Subgrid {
            cells: cells
                .iter()
                .map(|(at, t)| SubCell {
                    at: *at,
                    edge: Some((*t).to_owned()),
                    ..SubCell::default()
                })
                .collect(),
        }),
        ..Descriptor::default()
    };
    d.extent.footprint = Some((2.0 * SNAP, 2.0 * SNAP));
    d.extent.height = Some(SNAP);
    d
}

fn kit(pieces: Vec<Descriptor>) -> Library {
    Library {
        descriptors: pieces,
        ..Library::default()
    }
}

/// The east face reads `[jamb_l jamb_r]` along +Z. Turned, the piece presents that pair reversed or
/// not at all, so nothing in a one-piece kit closes it.
fn jamb() -> Descriptor {
    piece("jamb", &[((1, 0, 0), "jamb_l"), ((1, 0, 1), "jamb_r")])
}

#[test]
fn a_socket_nothing_presents_back_is_named_with_its_tokens() {
    let found = dead_ends(&kit(vec![jamb()]), 1);
    let east = found
        .iter()
        .find(|d| d.piece == "jamb" && d.dir == E)
        .unwrap_or_else(|| panic!("the jamb's east face meets nothing: {found:#?}"));
    assert_eq!(
        east.face,
        vec![Some("jamb_l".to_owned()), Some("jamb_r".to_owned())]
    );
    assert!(
        east.message.contains("jamb") && east.message.contains("jamb_l jamb_r"),
        "{}",
        east.message
    );
}

#[test]
fn adding_the_partner_piece_clears_the_dead_end() {
    // Its west face presents the same pair in the same order, so placed east of the jamb it agrees.
    let partner = piece("partner", &[((0, 0, 0), "jamb_l"), ((0, 0, 1), "jamb_r")]);
    let found = dead_ends(&kit(vec![jamb(), partner]), 1);
    assert!(
        !found.iter().any(|d| d.piece == "jamb" && d.dir == E),
        "the partner meets the jamb's east face: {found:#?}"
    );
}

#[test]
fn a_face_that_reads_the_same_both_ways_is_met_by_its_own_piece_turned() {
    // Two hall ends kiss: a uniform face is closed by a second copy turned around.
    let end = piece("hall_end", &[((1, 0, 0), "stone"), ((1, 0, 1), "stone")]);
    let found = dead_ends(&kit(vec![end]), 1);
    assert!(
        !found.iter().any(|d| d.piece == "hall_end" && d.dir == E),
        "{found:#?}"
    );
}

#[test]
fn a_kit_with_no_tokens_has_no_dead_ends() {
    assert_eq!(dead_ends(&kit(vec![piece("blank", &[])]), 1), Vec::new());
}
