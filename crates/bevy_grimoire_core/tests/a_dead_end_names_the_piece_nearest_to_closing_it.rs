//! **A dead end names the piece nearest to closing it** (#145, lens #4, after #142).
//!
//! Knowing that nothing in the kit meets a face is half the answer; the author's next question is
//! what nearly does. `adjacency::dead_ends` keeps, for each dead end, the trial whose seam disagrees
//! with the face in the fewest cells, and names the cells it gets wrong.

use bevy_grimoire_core::adjacency::{dead_ends, Miss};
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

/// East face `[jamb_l jamb_r]` along +Z.
fn jamb() -> Descriptor {
    piece("jamb", &[((1, 0, 0), "jamb_l"), ((1, 0, 1), "jamb_r")])
}

#[test]
fn a_partner_one_token_off_is_named_with_the_cell_it_gets_wrong() {
    // Its west face reads `[jamb_l jamb_x]`: right in the first cell, wrong in the second.
    let near = piece("b_near", &[((0, 0, 0), "jamb_l"), ((0, 0, 1), "jamb_x")]);
    let found = dead_ends(&kit(vec![jamb(), near]), 1);
    let east = found
        .iter()
        .find(|d| d.piece == "jamb" && d.dir == E)
        .unwrap_or_else(|| panic!("the jamb's east face is still a dead end: {found:#?}"));
    let closest = east
        .closest
        .as_ref()
        .unwrap_or_else(|| panic!("a partner one token off is named: {east:#?}"));
    assert_eq!(closest.piece, "b_near", "{closest:#?}");
    assert_eq!(closest.turn, 0, "unturned, its west face lines up: {closest:#?}");
    assert_eq!(
        closest.misses,
        vec![Miss {
            at: 1,
            want: Some("jamb_r".to_owned()),
            found: Some("jamb_x".to_owned()),
        }],
        "only the second cell differs"
    );
    assert!(
        east.message.contains("b_near") && east.message.contains("wants jamb_r finds jamb_x"),
        "the sentence says what nearly meets it: {}",
        east.message
    );
}

#[test]
fn the_fewest_misses_win_over_the_piece_id() {
    // `a_far` sorts first but misses both cells; `b_near` misses one.
    let far = piece("a_far", &[((0, 0, 0), "oak"), ((0, 0, 1), "oak")]);
    let near = piece("b_near", &[((0, 0, 0), "jamb_l"), ((0, 0, 1), "jamb_x")]);
    let found = dead_ends(&kit(vec![jamb(), far, near]), 1);
    let east = found
        .iter()
        .find(|d| d.piece == "jamb" && d.dir == E)
        .unwrap_or_else(|| panic!("{found:#?}"));
    let closest = east.closest.as_ref().unwrap_or_else(|| panic!("{east:#?}"));
    assert_eq!(closest.piece, "b_near", "{closest:#?}");
    assert_eq!(closest.misses.len(), 1, "{closest:#?}");
}

#[test]
fn a_met_face_carries_no_dead_end_and_so_no_closest() {
    let partner = piece("partner", &[((0, 0, 0), "jamb_l"), ((0, 0, 1), "jamb_r")]);
    let found = dead_ends(&kit(vec![jamb(), partner]), 1);
    assert!(
        !found.iter().any(|d| d.piece == "jamb" && d.dir == E),
        "{found:#?}"
    );
}
