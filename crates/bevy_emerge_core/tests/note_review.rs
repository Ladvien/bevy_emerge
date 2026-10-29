//! **Reviewing a done note** (#93): keep it, or undo its work as one step unless a later operation
//! depends on it.

use bevy_emerge_core::map::Map;
use bevy_emerge_core::notes::{Footprint, Notes, Resolution, State, What};

fn done() -> (Notes, String) {
    let mut n = Notes::default();
    let id = n.add_area(&Map::default(), (4, 4, 5, 5), 1.0, (0.0, 0.0, 0.0, 0.0, 0.0), "three lamps").unwrap();
    n.send(&id).unwrap();
    n.claim(&id, "omp", 1000).unwrap();
    let r = Resolution {
        summary: "placed three lamps".into(),
        assumptions: vec!["lamps".into()],
        ops: vec!["op-1".into(), "op-2".into(), "op-3".into()],
        captures: vec!["before.png".into(), "after.png".into()],
    };
    n.resolve_record(&id, "omp", r, 1010).unwrap();
    (n, id)
}

fn fp(op: &str, p: &str, cell: (i32, i32)) -> Footprint {
    Footprint { op: op.into(), placements: vec![p.into()], cells: vec![cell] }
}

fn note_work() -> Vec<Footprint> {
    vec![fp("op-1", "p1", (4, 4)), fp("op-2", "p2", (4, 5)), fp("op-3", "p3", (5, 5))]
}

#[test]
fn an_unrelated_later_change_does_not_block_the_undo_and_the_note_reopens_with_its_history() {
    let (mut n, id) = done();
    let later = vec![fp("op-9", "far", (40, 40))];
    n.undo_work(&id, &note_work(), &later, 2000).unwrap();
    let note = n.get(&id).unwrap();
    assert_eq!(note.state, State::Open);
    assert!(note.stands(), "the sign is back");
    assert_eq!(note.history.last().unwrap().what, What::WorkUndone);
    assert!(note.history.iter().any(|e| e.what == What::Resolved), "the whole history is kept");
}

#[test]
fn a_piece_moved_by_hand_after_the_note_refuses_and_names_the_move() {
    let (mut n, id) = done();
    let later = vec![Footprint { op: "op-7".into(), placements: vec!["p2".into()], cells: vec![] }];
    let e = n.undo_work(&id, &note_work(), &later, 2000).unwrap_err();
    assert!(e.contains("op-7") && e.contains("placement p2"), "{e}");
    assert!(matches!(n.get(&id).unwrap().state, State::Resolved { .. }), "a refusal changes nothing");
}

#[test]
fn a_later_change_on_the_same_cell_blocks_too() {
    let (mut n, id) = done();
    let later = vec![Footprint { op: "op-8".into(), placements: vec![], cells: vec![(5, 5)] }];
    assert!(n.undo_work(&id, &note_work(), &later, 2000).unwrap_err().contains("cell (5,5)"));
}

#[test]
fn keep_archives_the_note_and_unkeep_brings_the_record_back() {
    let (mut n, id) = done();
    let before = n.get(&id).unwrap().state.clone();
    n.keep(&id, 2000).unwrap();
    let note = n.get(&id).unwrap();
    assert_eq!(note.status(), "kept");
    assert!(!note.stands(), "its sign goes away");
    n.unkeep(&id, 2001).unwrap();
    assert_eq!(n.get(&id).unwrap().state, before);
    assert!(n.keep("nope", 1).is_err());
    assert!(n.unkeep(&id, 2002).is_err(), "only a kept note is unkept");
}
