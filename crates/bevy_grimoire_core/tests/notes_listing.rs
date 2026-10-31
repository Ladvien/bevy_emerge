//! **The Notes panel's list** (#90): every note, newest first, filterable by status, and a resolved
//! note is never dropped from it.

use bevy_grimoire_core::notes::{Notes, Target};

fn three() -> Notes {
    let mut n = Notes::default();
    n.add(vec![Target::Cell(0, 0)], "first").unwrap();
    n.add(vec![Target::Cell(1, 1)], "second").unwrap();
    n.add(vec![Target::Cell(2, 2)], "third").unwrap();
    // `add` writes sent notes; the third is put back to a draft the way a person's note starts.
    n.notes[2].state = bevy_grimoire_core::notes::State::Draft;
    n.resolve("n2", vec!["op1".to_owned()], "done it").unwrap();
    n
}

fn ids(v: &[&bevy_grimoire_core::notes::Note]) -> Vec<String> {
    v.iter().map(|n| n.id.clone()).collect()
}

#[test]
fn the_list_is_newest_first_and_keeps_resolved_notes() {
    let n = three();
    assert_eq!(ids(&n.listing(None)), ["n3", "n2", "n1"]);
    assert_eq!(n.get("n1").unwrap().status(), "sent");
    assert_eq!(n.get("n2").unwrap().status(), "done");
    assert_eq!(n.get("n3").unwrap().status(), "draft");
}

#[test]
fn a_status_filter_keeps_only_that_status() {
    let n = three();
    assert_eq!(ids(&n.listing(Some("done"))), ["n2"]);
    assert_eq!(ids(&n.listing(Some("draft"))), ["n3"]);
    assert!(n.listing(Some("kept")).is_empty());
}

#[test]
fn a_note_on_cells_frames_their_rectangle_and_one_on_pieces_frames_by_id() {
    let mut n = Notes::default();
    n.add(vec![Target::Cell(0, 0), Target::Cell(2, 1)], "cells").unwrap();
    n.add(vec![Target::Placement("p1".to_owned())], "piece").unwrap();
    let t = bevy_grimoire_core::grid::TILE;
    assert_eq!(n.get("n1").unwrap().frame_rect(), Some([0.0, 0.0, 3.0 * t, 2.0 * t]));
    assert_eq!(n.get("n2").unwrap().frame_rect(), None);
}
