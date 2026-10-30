//! **Relations, not coordinates: a solver mends the relations a map breaks** (#137 slice 3).
//!
//! `relations::check` names what a map breaks; `relations::solve` nudges the pieces those relations
//! are about until they hold, never trading one broken relation for another, and says which pieces
//! it moved and how many relations stay broken.

use bevy_emerge_core::map::Placed;
use bevy_emerge_core::relations::{check, solve, Kind, Outcome, Relation, Relations, RELATIONS_VERSION};

fn at(id: &str, x: f32, z: f32, yaw_deg: f32) -> Placed {
    Placed {
        id: id.to_owned(),
        descriptor: id.to_owned(),
        at: (x, z),
        yaw: yaw_deg.to_radians(),
        ..Placed::default()
    }
}

fn rel(of: &str, kind: Kind) -> Relation {
    Relation {
        of: of.to_owned(),
        kind,
        hard: false,
    }
}

fn set(relations: Vec<Relation>) -> Relations {
    Relations {
        version: RELATIONS_VERSION,
        relations,
    }
}

fn unmet(placements: &[Placed], relations: &Relations) -> usize {
    check(placements, relations)
        .iter()
        .filter(|o| matches!(o, Outcome::Unmet { .. }))
        .count()
}

fn get<'a>(placements: &'a [Placed], id: &str) -> &'a Placed {
    placements.iter().find(|p| p.id == id).unwrap()
}

/// A table, a chair behind it with its back turned and too far away, a desk with its chair off to
/// one side, a door facing west with a crate in its swing, and a lamp resting on the crate.
fn room() -> Vec<Placed> {
    let mut lamp = at("lamp", 3.0, 0.2, 0.0);
    lamp.on = Some("crate".into());
    vec![
        at("table", 0.0, 0.0, 0.0),
        at("chair", 0.0, 3.0, 0.0),
        at("desk", -4.0, 0.0, 0.0),
        at("stool", -2.0, 0.0, 0.0),
        at("door", 4.0, 0.0, -90.0),
        at("crate", 3.0, 0.2, 0.0),
        lamp,
    ]
}

fn intent() -> Relations {
    set(vec![
        rel("chair", Kind::Faces("table".into())),
        rel(
            "chair",
            Kind::Near {
                to: "table".into(),
                within: 1.5,
            },
        ),
        rel(
            "stool",
            Kind::InFrontOf {
                of: "desk".into(),
                within: 1.0,
            },
        ),
        rel(
            "door",
            Kind::ClearInFront {
                depth: 1.5,
                width: 1.0,
            },
        ),
        rel(
            "crate",
            Kind::Apart {
                from: "table".into(),
                at_least: 2.0,
            },
        ),
    ])
}

#[test]
fn every_broken_relation_is_mended_and_none_is_broken_on_the_way() {
    let (placements, relations) = (room(), intent());
    assert_eq!(unmet(&placements, &relations), 4, "chair ×2, stool, door");

    let solved = solve(&placements, &relations);

    assert_eq!(solved.unmet_before, 4);
    assert_eq!(solved.unmet_after, 0, "{:?}", check(&solved.placements, &relations));
    assert_eq!(unmet(&solved.placements, &relations), 0);
    let mut moved = solved.moved.clone();
    moved.sort();
    assert_eq!(moved, ["chair", "crate", "stool"], "the door and the anchors stay put");
    for anchor in ["table", "desk", "door"] {
        assert_eq!(get(&solved.placements, anchor), get(&placements, anchor), "{anchor}");
    }
}

#[test]
fn what_rests_on_a_moved_piece_goes_with_it() {
    let (placements, relations) = (room(), intent());
    let solved = solve(&placements, &relations);
    let (crate_, lamp) = (get(&solved.placements, "crate"), get(&solved.placements, "lamp"));
    assert_ne!(crate_.at, (3.0, 0.2), "the crate left the door's swing");
    assert_eq!(lamp.at, crate_.at, "the lamp still stands on the crate");
    assert!(!solved.moved.iter().any(|id| id == "lamp"), "the lamp was carried, not placed");
}

#[test]
fn a_piece_resting_on_another_is_not_pulled_off_it() {
    let mut placements = room();
    placements.push({
        let mut cup = at("cup", 0.0, 0.0, 0.0);
        cup.on = Some("table".into());
        cup
    });
    let relations = set(vec![rel(
        "cup",
        Kind::Apart {
            from: "table".into(),
            at_least: 2.0,
        },
    )]);
    let solved = solve(&placements, &relations);
    assert_eq!(solved.unmet_after, 1, "the cup stays on its table, so the relation stays broken");
    assert!(solved.moved.is_empty());
    assert_eq!(solved.placements, placements);
}

#[test]
fn relations_that_cannot_all_hold_never_end_worse_than_they_began() {
    let placements = vec![at("a", 0.0, 0.0, 0.0), at("b", 1.0, 0.0, 0.0)];
    let relations = set(vec![
        rel(
            "a",
            Kind::Near {
                to: "b".into(),
                within: 0.5,
            },
        ),
        rel(
            "a",
            Kind::Apart {
                from: "b".into(),
                at_least: 2.0,
            },
        ),
    ]);
    let solved = solve(&placements, &relations);
    assert_eq!(solved.unmet_before, 2);
    assert_eq!(solved.unmet_after, 1, "one of the two contradictory relations is met");
    assert_eq!(unmet(&solved.placements, &relations), 1);
}

#[test]
fn a_relation_naming_a_missing_piece_moves_nothing() {
    let placements = room();
    let relations = set(vec![rel("chair", Kind::Faces("sofa".into()))]);
    let solved = solve(&placements, &relations);
    assert_eq!((solved.unmet_before, solved.unmet_after), (0, 0));
    assert!(solved.moved.is_empty());
    assert_eq!(solved.placements, placements);
}
