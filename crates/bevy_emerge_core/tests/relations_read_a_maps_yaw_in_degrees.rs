//! **A map's yaw is degrees, and the relations read it so** (#167).
//!
//! `Placed::yaw` is degrees wherever the editor touches it: the viewport turns a placement by
//! `Quat::from_rotation_y(yaw.to_radians())`, so at 0 a piece faces +z and at 90 it faces +x.
//! `relations::check_map` is the boundary the editor reads a map's rows through; these are the
//! issue's own acceptance cases, written in the unit the map stores.

use bevy_emerge_core::map::Placed;
use bevy_emerge_core::relations::{check_map, Kind, Outcome, Relation, Relations, RELATIONS_VERSION};

fn piece(id: &str, at: (f32, f32), yaw_deg: f32) -> Placed {
    Placed {
        id: id.to_owned(),
        descriptor: id.to_owned(),
        at,
        yaw: yaw_deg,
        ..Placed::default()
    }
}

fn one(of: &str, kind: Kind) -> Relations {
    Relations {
        version: RELATIONS_VERSION,
        relations: vec![Relation { of: of.into(), kind, hard: false }],
    }
}

/// Two pews two metres apart; the second is kept on the first's heading.
fn aligned(first_deg: f32, second_deg: f32) -> Outcome {
    let rows = [piece("pew_1", (0.0, 0.0), first_deg), piece("pew_2", (2.0, 0.0), second_deg)];
    check_map(&rows, &one("pew_2", Kind::AlignedWith("pew_1".into()))).remove(0)
}

/// A chair at the origin turned to `yaw_deg`, kept facing a table at `table`.
fn faces(yaw_deg: f32, table: (f32, f32)) -> Outcome {
    let rows = [piece("chair", (0.0, 0.0), yaw_deg), piece("table", table, 0.0)];
    check_map(&rows, &one("chair", Kind::Faces("table".into()))).remove(0)
}

fn measured(o: &Outcome) -> f32 {
    match o {
        Outcome::Met { measured } | Outcome::Unmet { measured, .. } => *measured,
        Outcome::Missing { id } => panic!("{id} missing"),
    }
}

#[test]
fn pews_twenty_degrees_apart_are_aligned() {
    // Read as radians, 20 is 66 degrees off 0 and unmet.
    let o = aligned(0.0, 20.0);
    assert!(matches!(o, Outcome::Met { .. }), "{o:?}");
    assert!((measured(&o) - 20.0).abs() < 1e-3, "{o:?}");
}

#[test]
fn pews_a_quarter_turn_apart_are_not_aligned() {
    let o = aligned(0.0, 90.0);
    assert!(matches!(o, Outcome::Unmet { by: None, .. }), "{o:?}");
    assert!((measured(&o) - 90.0).abs() < 1e-3, "{o:?}");
}

#[test]
fn pews_at_350_and_10_degrees_are_aligned_the_short_way_round() {
    let o = aligned(350.0, 10.0);
    assert!(matches!(o, Outcome::Met { .. }), "{o:?}");
    assert!((measured(&o) - 20.0).abs() < 1e-3, "{o:?}");
    // A full turn is the same heading as drawn: 360 against 0 is dead-on, not 92 degrees off.
    let o = aligned(0.0, 360.0);
    assert!(matches!(o, Outcome::Met { .. }), "{o:?}");
    assert!(measured(&o) < 1e-3, "{o:?}");
}

#[test]
fn a_chair_faces_what_the_viewport_draws_it_facing() {
    // Yaw 0 draws the chair facing +z: the table straight ahead on +z is faced, one on +x is not.
    assert!(matches!(faces(0.0, (0.0, 3.0)), Outcome::Met { .. }), "{:?}", faces(0.0, (0.0, 3.0)));
    assert!(matches!(faces(0.0, (3.0, 0.0)), Outcome::Unmet { .. }), "{:?}", faces(0.0, (3.0, 0.0)));
    // Yaw 90 draws it facing +x: now the table on +x is faced and the one on +z is not.
    assert!(matches!(faces(90.0, (3.0, 0.0)), Outcome::Met { .. }), "{:?}", faces(90.0, (3.0, 0.0)));
    assert!(matches!(faces(90.0, (0.0, 3.0)), Outcome::Unmet { .. }), "{:?}", faces(90.0, (0.0, 3.0)));
    // Every quarter turn as drawn: 180 faces -z and 270 faces -x. Read as radians, 90 happens to
    // land within the tolerance of +x (116.6 degrees), but 180 and 270 point nowhere near.
    assert!(matches!(faces(180.0, (0.0, -3.0)), Outcome::Met { .. }), "{:?}", faces(180.0, (0.0, -3.0)));
    assert!(matches!(faces(270.0, (-3.0, 0.0)), Outcome::Met { .. }), "{:?}", faces(270.0, (-3.0, 0.0)));
}
