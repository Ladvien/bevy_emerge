//! **A claim is a lease** (#91): held by one agent, renewed while it works, expired when it is not,
//! and two notes whose reach overlaps are worked one at a time, in send order.

use bevy_grimoire_core::map::Map;
use bevy_grimoire_core::notes::{Notes, State, What};

/// A sent area note at `cells` (margin 1), sent at the clock's `t`.
fn sent(notes: &mut Notes, cells: (i32, i32, i32, i32)) -> String {
    let id = notes.add_area(&Map::default(), cells, 1.0, (0.0, 0.0, 0.0, 0.0, 0.0), "do it").unwrap();
    notes.send(&id).unwrap();
    id
}

#[test]
fn a_held_note_refuses_a_second_claim_naming_the_holder() {
    let mut n = Notes::default();
    let a = sent(&mut n, (0, 0, 1, 1));
    n.claim(&a, "omp", 1000).unwrap();
    let e = n.claim(&a, "cursor", 1001).unwrap_err();
    assert!(e.contains("claimed by omp"), "{e}");
    assert_eq!(n.get(&a).unwrap().holder(), Some("omp"));
}

#[test]
fn an_unrenewed_lease_expires_and_returns_the_note_to_sent() {
    let mut n = Notes { lease_secs: 60, ..Notes::default() };
    let a = sent(&mut n, (0, 0, 1, 1));
    assert_eq!(n.claim(&a, "omp", 1000).unwrap(), 1060);
    assert_eq!(n.renew(&a, "omp", 1050).unwrap(), 1110, "renewing runs the lease on from now");
    assert!(n.expire(1109).is_empty(), "still held one second before it runs out");
    assert_eq!(n.expire(1110), vec![a.clone()]);
    assert_eq!(n.get(&a).unwrap().state, State::Open);
    assert_eq!(n.get(&a).unwrap().history.last().unwrap().what, What::Expired);
    assert!(n.renew(&a, "omp", 1111).is_err(), "an expired lease cannot be renewed");
    n.claim(&a, "cursor", 1111).expect("and another agent may now claim it");
}

#[test]
fn only_the_holder_renews_or_releases_but_the_person_may_release() {
    let mut n = Notes::default();
    let a = sent(&mut n, (0, 0, 1, 1));
    n.claim(&a, "omp", 10).unwrap();
    assert!(n.renew(&a, "cursor", 11).unwrap_err().contains("claimed by omp"));
    assert!(n.release(&a, Some("cursor"), 11).is_err());
    n.release(&a, None, 12).unwrap();
    assert_eq!(n.get(&a).unwrap().state, State::Open);
}

#[test]
fn distant_notes_are_claimed_together_and_overlapping_ones_in_send_order() {
    let mut n = Notes::default();
    let a = sent(&mut n, (0, 0, 1, 1));
    let b = sent(&mut n, (10, 10, 11, 11));
    let c = sent(&mut n, (2, 0, 3, 1)); // within a's margin: overlaps a
    n.claim(&a, "omp", 100).unwrap();
    n.claim(&b, "cursor", 100).expect("a note far from the held one is claimed too");
    let e = n.claim(&c, "claude", 100).unwrap_err();
    assert!(e.contains(&format!("overlaps {a}")) && e.contains("omp"), "{e}");
    n.resolve(&a, vec![], "done").unwrap();
    n.claim(&c, "claude", 101).expect("once the overlapping note resolves it can be claimed");
}

#[test]
fn an_overlapping_note_sent_later_waits_for_the_one_sent_first() {
    let mut n = Notes::default();
    let first = sent(&mut n, (0, 0, 1, 1));
    let second = sent(&mut n, (2, 0, 3, 1));
    let e = n.claim(&second, "omp", 5).unwrap_err();
    assert!(e.contains("sent first"), "{e}");
    n.claim(&first, "omp", 5).unwrap();
}

#[test]
fn a_draft_or_a_resolved_note_cannot_be_claimed() {
    let mut n = Notes::default();
    let d = n.add_area(&Map::default(), (0, 0, 0, 0), 1.0, (0.0, 0.0, 0.0, 0.0, 0.0), "x").unwrap();
    assert!(n.claim(&d, "omp", 1).unwrap_err().contains("draft"));
    n.send(&d).unwrap();
    n.claim(&d, "omp", 1).unwrap();
    n.resolve(&d, vec![], "ok").unwrap();
    assert!(n.claim(&d, "omp", 2).unwrap_err().contains("resolved"));
}
