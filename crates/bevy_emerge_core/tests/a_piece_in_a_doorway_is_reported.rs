//! **A piece standing in a doorway is reported (E-079).**
//!
//! # The gap this closes
//!
//! [`Mount::InOpening`]'s `clear` is the `(width, height)` of the hole a door fills, and
//! `Descriptor::clearance` is documented as *"space that must stay free around the piece"*. Neither
//! was read by anything that decides space: `stack::same_layer` returns `false` for every
//! heterogeneous pair — which it must, or a door and its wall would conflict — and its own doc states
//! the consequence in as many words: *"neither can a door and a crate standing in the doorway"*.
//! `blocking`'s bounded claim has said since it was written that this was scoped out.
//!
//! So a crate could be dropped in a doorway, the click path accepted it, the map validated, and no
//! answer anywhere named it.
//!
//! # What is pinned here
//!
//! 1. A crate in a measured doorway is one finding, naming the door and the crate.
//! 2. A crate **beside** it is not — the hole is `clear`'s rectangle, not the frame's footprint, so
//!    the jambs do not make every adjacent piece an intruder.
//! 3. A crate standing **above** the hole's head is not, and neither is one that never reaches it:
//!    vertical is read because a doorway is the one case where both numbers are in the data.
//! 4. An opening whose `clear` is `None` reports nothing. "Nobody has measured it" is not "zero wide".
//! 5. Another opening in the same hole is not reported — that pair is `same_layer`'s question.
//! 6. A piece resting on the door is not reported; it is part of the door.
//! 7. The list is ordered and deterministic, so two maps' answers diff as sets.

use bevy_emerge_core::descriptor::{Align, Descriptor, Extent, Mount, Offers};
use bevy_emerge_core::library::{LIBRARY_VERSION, Library};
use bevy_emerge_core::map::{Map, Placed};
use bevy_emerge_core::stack::blocked_openings;

/// A door that fills a measured hole. Footprint `(0.2, 1.0)` is the shipped `site/wall_doorway`
/// shape: 0.2 m through the wall, 1.0 m of frame along it, framing a **0.6 m** hole.
fn door(clear: Option<(f32, f32)>) -> Descriptor {
    Descriptor {
        id: "door".into(),
        extent: Extent {
            footprint: Some((0.2, 1.0)),
            height: Some(2.0),
        },
        mount: Some(Mount::InOpening { clear }),
        ..Descriptor::default()
    }
}

/// A 0.5 m crate standing on the floor — the ordinary thing that ends up in a doorway.
fn crate_() -> Descriptor {
    Descriptor {
        id: "crate".into(),
        extent: Extent {
            footprint: Some((0.5, 0.5)),
            height: Some(0.5),
        },
        mount: Some(Mount::OnFloor),
        ..Descriptor::default()
    }
}

/// A crate with **no measured extents**, which takes part in nothing (on `covers`' rule: unknown is
/// not everywhere).
fn unmeasured() -> Descriptor {
    Descriptor {
        id: "unmeasured".into(),
        mount: Some(Mount::OnFloor),
        ..Descriptor::default()
    }
}

fn lib(descriptors: Vec<Descriptor>) -> Library {
    Library {
        version: LIBRARY_VERSION,
        note: None,
        descriptors,
    }
}

fn at(id: &str, descriptor: &str, at: (f32, f32)) -> Placed {
    Placed {
        id: id.into(),
        descriptor: descriptor.into(),
        at,
        ..Placed::default()
    }
}

fn map(placements: Vec<Placed>) -> Map {
    Map {
        name: "doorway".into(),
        placements,
        ..Map::default()
    }
}

/// **A crate in the hole is the finding.** The door is centred on the origin with its 0.6 m hole
/// running along Z (yaw 0, so `plan_box`'s second axis is +Z), which means the crate at `(0, 0)` is
/// exactly in it.
#[test]
fn a_crate_standing_in_a_measured_doorway_is_named() {
    let l = lib(vec![door(Some((0.6, 1.9))), crate_()]);
    let m = map(vec![at("door@0", "door", (0.0, 0.0)), at("crate@0", "crate", (0.0, 0.0))]);
    assert_eq!(
        blocked_openings(&m, &l),
        vec![(0, 1)],
        "the crate is in the doorway and the finding names both, opening first"
    );
}

/// **The hole is `clear`'s rectangle, not the frame's footprint.** The frame is 1.0 m of jambs around
/// a 0.6 m opening, so a crate 0.7 m off centre is beside the door leaf and inside the frame's own
/// extent — and it is NOT in the way. This is the assertion that fails if `hole_rect` ever takes the
/// door's whole footprint.
#[test]
fn a_crate_beside_the_doorway_is_not_in_it() {
    let l = lib(vec![door(Some((0.6, 1.9))), crate_()]);
    // Hole half-width is 0.3 along Z; the crate's half-depth is 0.25. At z = 0.7 its near edge is
    // 0.45 from centre, well clear of the hole's 0.3 edge — but only 0.2 m inside the frame's 0.5.
    let m = map(vec![at("door@0", "door", (0.0, 0.0)), at("crate@0", "crate", (0.0, 0.7))]);
    assert!(
        blocked_openings(&m, &l).is_empty(),
        "a piece standing beside the doorway is not standing in it"
    );
}

/// **Vertical is read.** A 0.5 m crate cannot reach above a 1.9 m hole, but a piece whose box starts
/// above the hole's head is clear of it — which is the case that would otherwise flag every lintel
/// and every wall piece sharing the plan.
#[test]
fn a_piece_wholly_above_the_hole_is_clear_of_it() {
    let mut high = crate_();
    high.id = "transom".into();
    high.mount = Some(Mount::OnWall { height: 2.5 });
    high.extent.height = Some(0.4);
    let l = lib(vec![door(Some((0.6, 1.9))), high]);
    let m = map(vec![
        at("door@0", "door", (0.0, 0.0)),
        at("transom@0", "transom", (0.0, 0.0)),
    ]);
    assert!(
        blocked_openings(&m, &l).is_empty(),
        "the hole is 1.9 m tall and this piece starts at 2.5 m"
    );
}

/// **An unmeasured opening reports nothing.** `clear: None` is the shipped `furniture_kenney.ron`
/// door's honest state, and flagging every crate near a row nobody has measured is exactly the false
/// positive that teaches an author to ignore the field.
#[test]
fn an_opening_nobody_has_measured_reports_nothing() {
    let l = lib(vec![door(None), crate_()]);
    let m = map(vec![at("door@0", "door", (0.0, 0.0)), at("crate@0", "crate", (0.0, 0.0))]);
    assert!(blocked_openings(&m, &l).is_empty());
}

/// **Two openings in one hole are `same_layer`'s question, not this one's.** A double door is two
/// leaves that must not be reported as each blocking the other.
#[test]
fn another_opening_in_the_hole_is_not_an_intruder() {
    let l = lib(vec![door(Some((0.6, 1.9)))]);
    let m = map(vec![
        at("door@0", "door", (0.0, 0.0)),
        at("door@1", "door", (0.0, 0.0)),
    ]);
    assert!(blocked_openings(&m, &l).is_empty());
}

/// **A piece resting on the door is part of it.** A mail slot on a door leaf is one object, and its
/// `on` names the door, which is the record that says so.
#[test]
fn a_piece_resting_on_the_door_is_not_in_its_way() {
    let l = lib(vec![door(Some((0.6, 1.9))), crate_()]);
    let mut slot = at("slot@0", "crate", (0.0, 0.0));
    slot.on = Some("door@0".into());
    let m = map(vec![at("door@0", "door", (0.0, 0.0)), slot]);
    assert!(
        blocked_openings(&m, &l).is_empty(),
        "a thing mounted on the door is not standing in it"
    );
}

/// **A door turned a quarter turn turns its hole with it.** The `clear` width runs along the wall, so
/// at yaw 90 the hole runs along X. A crate in it is named; the same crate at 90° yaw of the door is
/// then beside it.
#[test]
fn the_hole_turns_with_the_door() {
    let l = lib(vec![door(Some((0.6, 1.9))), crate_()]);
    let mut turned = at("door@0", "door", (0.0, 0.0));
    turned.yaw = 90.0;
    let in_hole = map(vec![turned.clone(), at("crate@0", "crate", (0.0, 0.0))]);
    assert_eq!(blocked_openings(&in_hole, &l), vec![(0, 1)]);
    // Turned, the hole now runs along X, so a crate 0.7 m along Z is beside it.
    let beside = map(vec![turned, at("crate@0", "crate", (0.0, 0.7))]);
    assert!(blocked_openings(&beside, &l).is_empty());
}

/// **Unmeasured extents take part in nothing**, and the answer is ordered.
#[test]
fn an_unmeasured_piece_is_not_an_intruder_and_the_list_is_ordered() {
    let l = lib(vec![door(Some((0.6, 1.9))), crate_(), unmeasured()]);
    let m = map(vec![
        at("door@0", "door", (0.0, 0.0)),
        at("door@1", "door", (4.0, 0.0)),
        at("unknown@0", "unmeasured", (0.0, 0.0)),
        at("crate@0", "crate", (4.0, 0.0)),
    ]);
    assert_eq!(
        blocked_openings(&m, &l),
        vec![(1, 3)],
        "only the crate in the second doorway: the unmeasured piece is silent, and the answer is in \
         map order"
    );
    // Deterministic: asking twice gives the same list.
    assert_eq!(blocked_openings(&m, &l), blocked_openings(&m, &l));
}
