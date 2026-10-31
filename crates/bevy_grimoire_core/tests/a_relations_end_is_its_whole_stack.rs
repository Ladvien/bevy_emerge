//! **A relation's end is its whole stack** (#188).
//!
//! What rests on a relation's end, and what that end rests on, are part of the end, as they already
//! are for `Kind::ReachableFrom`. So a jug on the counter is not in the counter's own working
//! corridor (`Kind::ClearInFront`), and the stall a sign hangs on does not hide the sign from the
//! gate (`Kind::VisibleFrom`). Before the fix both read as broken on any dressed map, and `solve`
//! could not mend them: moving the end's stack carries the end along.

use bevy_grimoire_core::map::Placed;
use bevy_grimoire_core::relations::{check, solve, Kind, Outcome, Relation, Relations, RELATIONS_VERSION};

fn at(id: &str, x: f32, z: f32) -> Placed {
    Placed {
        id: id.to_owned(),
        descriptor: id.to_owned(),
        at: (x, z),
        ..Placed::default()
    }
}

fn on(mut p: Placed, host: &str) -> Placed {
    p.on = Some(host.to_owned());
    p
}

fn one(of: &str, kind: Kind) -> Relations {
    Relations {
        version: RELATIONS_VERSION,
        relations: vec![Relation {
            of: of.into(),
            kind,
            hard: false,
        }],
    }
}

/// The counter at the origin facing +z (yaw 0), its working side 1.5 m deep and 1 m wide.
fn counter_clear() -> Relations {
    one("counter", Kind::ClearInFront { depth: 1.5, width: 1.0 })
}

/// The sign must be seen from the gate, 10 m away along +x, with 1 m either side of the line clear.
fn sign_seen_from_gate() -> Relations {
    one(
        "sign",
        Kind::VisibleFrom {
            from: "gate".into(),
            clearance: 1.0,
        },
    )
}

#[test]
fn a_rider_on_the_subject_does_not_block_its_corridor() {
    let map = vec![at("counter", 0.0, 0.0), on(at("jug", 0.0, 0.2), "counter")];
    assert_eq!(check(&map, &counter_clear()), vec![Outcome::Met { measured: 1.5 }]);
    let solved = solve(&map, &counter_clear());
    assert_eq!((solved.unmet_before, solved.unmet_after), (0, 0));
    assert!(solved.moved.is_empty(), "nothing to mend, nothing moves: {:?}", solved.moved);
}

#[test]
fn a_rider_two_deep_or_beside_the_subject_on_its_host_does_not_block_it() {
    // The counter stands on a dais; a lamp rides the dais beside it, a tray rides the counter and a
    // cup rides the tray: each is part of the counter's stack.
    let map = vec![
        at("dais", 0.0, -0.1),
        on(at("counter", 0.0, 0.0), "dais"),
        on(at("lamp", 0.3, 0.4), "dais"),
        on(at("tray", 0.0, 0.3), "counter"),
        on(at("cup", 0.0, 0.6), "tray"),
    ];
    assert_eq!(check(&map, &counter_clear()), vec![Outcome::Met { measured: 1.5 }]);
}

#[test]
fn a_stranger_in_the_corridor_still_blocks_it_beside_a_rider() {
    let map = vec![
        at("counter", 0.0, 0.0),
        on(at("jug", 0.0, 0.2), "counter"),
        at("stool", 0.1, 0.9),
    ];
    match check(&map, &counter_clear()).remove(0) {
        Outcome::Unmet { measured, by } => {
            assert_eq!(by.as_deref(), Some("stool"), "the stranger is named, not the jug");
            assert!((measured - 0.9).abs() < 1e-4, "measures how far in it stands: {measured}");
        }
        other => panic!("a stool in the corridor must break it: {other:?}"),
    }
    let solved = solve(&map, &counter_clear());
    assert_eq!((solved.unmet_before, solved.unmet_after), (1, 0));
    assert_eq!(solved.moved, vec!["stool".to_owned()], "the counter stays where it stands");
}

#[test]
fn the_host_an_end_rests_on_does_not_block_the_sight_line() {
    let map = vec![
        at("gate", 0.0, 0.0),
        at("stall", 9.6, 0.0),
        on(at("sign", 10.0, 0.0), "stall"),
    ];
    assert_eq!(check(&map, &sign_seen_from_gate()), vec![Outcome::Met { measured: 1.0 }]);
    let solved = solve(&map, &sign_seen_from_gate());
    assert_eq!((solved.unmet_before, solved.unmet_after), (0, 0));
}

#[test]
fn what_shares_the_viewers_host_does_not_block_the_sight_line() {
    // The viewer stands on a tower; a banner on the same tower sits just ahead on the line.
    let map = vec![
        at("tower", -0.2, 0.0),
        on(at("gate", 0.0, 0.0), "tower"),
        on(at("banner", 0.4, 0.1), "tower"),
        at("sign", 10.0, 0.0),
    ];
    assert_eq!(check(&map, &sign_seen_from_gate()), vec![Outcome::Met { measured: 1.0 }]);
}

#[test]
fn a_stranger_on_the_line_still_blocks_beside_the_ends_host_and_solve_clears_it() {
    let map = vec![
        at("gate", 0.0, 0.0),
        at("stall", 9.6, 0.0),
        on(at("sign", 10.0, 0.0), "stall"),
        at("cart", 5.0, -0.3),
    ];
    match check(&map, &sign_seen_from_gate()).remove(0) {
        Outcome::Unmet { measured, by } => {
            assert_eq!(by.as_deref(), Some("cart"), "the cart is named, not the stall");
            assert!((measured - 0.3).abs() < 1e-4, "{measured}");
        }
        other => panic!("a cart on the line must break it: {other:?}"),
    }
    let solved = solve(&map, &sign_seen_from_gate());
    assert_eq!((solved.unmet_before, solved.unmet_after), (1, 0));
    assert_eq!(solved.moved, vec!["cart".to_owned()], "the stall and the sign stay");
}
