//! **A piece standing in space another piece reserved is reported (E-079, its second half).**
//!
//! [`Descriptor::clearance`] has been documented since the schema was written as *"space that must stay
//! free around the piece"*, and nothing that decides space read it. `same_layer` returns `false` for
//! every heterogeneous pair — it must, or a wall and the floor piece it stands beside would conflict —
//! and within a layer a flush neighbour is legal by `OVERLAP_EPS`. So an author could write Merrell's
//! own "0.91 m around this dining table" and a chair could be put flush against the tabletop, the map
//! would validate, and no answer anywhere would name it.
//!
//! # What is pinned here
//!
//! 1. A chair inside a table's `Around` band is one finding; one outside it is none.
//! 2. The band starts at the piece's own edge — the distance is what must stay free, not the gap to
//!    the centre. A piece at exactly `0.91 + half` of clearance away is clear; one nearer is not.
//! 3. `Front` is the piece's own front, composed with its yaw and `align.front`, and it turns with the
//!    piece — a chair behind a shelf whose front band faces the other way is not in it.
//! 4. Tutenel's exemption: an intruder that reserves space itself is not reported, so a dining set's
//!    chairs do not flag each other.
//! 5. A `Tiled` or `Decal` intruder is not reported — a rug under the table is not in the way of it.
//! 6. A rider on the keeper is not reported, and a piece whose span misses the keeper's is not either.
//! 7. A keeper with no clearance reports nothing, and an unmeasured keeper takes part in nothing.
//! 8. The list is ordered and deduplicated, so one intruder pressed by two `Around` bands is one
//!    finding and two runs over one map diff as sets.

use bevy_emerge_core::descriptor::{Clearance, Descriptor, Dir, Extent, Face, Mount};
use bevy_emerge_core::library::{LIBRARY_VERSION, Library};
use bevy_emerge_core::map::{Map, Placed};
use bevy_emerge_core::stack::blocked_clearances;

/// A 1.2 m square dining table reserving the given clearance on every side — Merrell et al.'s 0.91 m.
fn table(clearance: Vec<Clearance>) -> Descriptor {
    Descriptor {
        id: "table".into(),
        extent: Extent {
            footprint: Some((1.2, 1.2)),
            height: Some(0.76),
        },
        mount: Some(Mount::OnFloor),
        clearance,
        ..Descriptor::default()
    }
}

/// A 0.5 m chair standing on the floor. No clearance: it is the intruder in every case but Tutenel's.
fn chair() -> Descriptor {
    Descriptor {
        id: "chair".into(),
        extent: Extent {
            footprint: Some((0.5, 0.5)),
            height: Some(0.9),
        },
        mount: Some(Mount::OnFloor),
        ..Descriptor::default()
    }
}

/// A chair that reserves space itself — Tutenel's clearance feature, so it may share another's band.
fn clearing_chair() -> Descriptor {
    Descriptor {
        clearance: vec![Clearance { dir: Dir::Back, dist: 0.3 }],
        ..chair()
    }
}

fn around(dist: f32) -> Vec<Clearance> {
    vec![Clearance { dir: Dir::Around, dist }]
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
        name: "clearance".into(),
        placements,
        ..Map::default()
    }
}

/// **The chair in the band is the finding; the one outside it is not.** The table is 1.2 m square, so
/// its edge is 0.6 m from centre and the band reaches 0.91 m past that. A 0.5 m chair's own near edge
/// is 0.25 m before its centre: at z = 1.6 the near edge is 1.35 m out, comfortably inside 1.51 m; at
/// z = 2.0 it is 1.75 m out, clear.
#[test]
fn a_chair_inside_the_reserved_band_is_named() {
    let l = lib(vec![table(around(0.91)), chair()]);
    let inside = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 1.6)),
    ]);
    assert_eq!(
        blocked_clearances(&inside, &l),
        vec![(0, 1)],
        "the chair is inside the table's pull-out space and the finding names both, keeper first"
    );

    let outside = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 2.0)),
    ]);
    assert!(
        blocked_clearances(&outside, &l).is_empty(),
        "a chair clear of the band is not a finding"
    );
}

/// **The distance is free space past the piece's edge, not a radius from its centre.** A wall the
/// exact width of the band away is clear; one centimetre nearer is not. This is the assertion that
/// fails if the band is ever built from the centre rather than from the footprint.
#[test]
fn the_band_is_measured_from_the_edge() {
    let l = lib(vec![table(around(0.91)), chair()]);
    // The table's edge is at z = 0.6. The chair's half-depth is 0.25. At the boundary the chair's near
    // edge sits exactly 0.91 m past the table's edge: z = 0.6 + 0.91 + 0.25 = 1.76.
    let edge = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 1.76)),
    ]);
    assert!(
        blocked_clearances(&edge, &l).is_empty(),
        "the reserved distance is exactly met, so nothing is in it"
    );

    let pressed = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 1.70)),
    ]);
    assert_eq!(
        blocked_clearances(&pressed, &l),
        vec![(0, 1)],
        "6 cm inside the band is a finding"
    );
}

/// **`Front` is the piece's own front, and it turns with the piece.** A shelf with a 0.6 m front band
/// reserves it in front of itself; a crate behind the shelf is not in it, and the same crate in front
/// of it is.
#[test]
fn the_front_band_points_out_of_the_pieces_own_front() {
    let mut shelf = table(vec![Clearance { dir: Dir::Front, dist: 0.6 }]);
    shelf.id = "shelf".into();
    shelf.extent.footprint = Some((1.2, 0.4));
    // `Face::South` is yaw 0, the engine's forward being `(sin yaw, cos yaw)` — so at yaw 0 the front
    // band lies along +Z.
    shelf.align.front = Some(Face::South);
    let l = lib(vec![shelf, chair()]);

    let in_front = map(vec![
        at("shelf@0", "shelf", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 1.0)),
    ]);
    assert_eq!(
        blocked_clearances(&in_front, &l),
        vec![(0, 1)],
        "the chair stands in the shelf's own front band"
    );

    let behind = map(vec![
        at("shelf@0", "shelf", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, -1.0)),
    ]);
    assert!(
        blocked_clearances(&behind, &l).is_empty(),
        "behind the shelf is not its front band"
    );
}

/// **Tutenel's exemption: a clearance feature may share another's band.** Two chairs' pull-out spaces
/// overlap by design, so a dining set must not be a wall of findings. The table's own band still
/// reports the chair that does not reserve anything.
#[test]
fn a_piece_that_reserves_space_itself_may_share_the_band() {
    let l = lib(vec![table(around(0.91)), clearing_chair()]);
    let m = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 1.6)),
    ]);
    assert!(
        blocked_clearances(&m, &l).is_empty(),
        "a clearance feature may overlap another clearance feature's space"
    );
}

/// **Floor covering is not in the way.** A rug under a table is `Tiled`, and a wall decal claims no
/// volume — neither is something the table's chairs have to get past.
#[test]
fn a_tile_or_a_decal_in_the_band_is_not_an_intruder() {
    let mut rug = chair();
    rug.id = "rug".into();
    rug.mount = Some(Mount::Tiled);
    let mut sign = chair();
    sign.id = "sign".into();
    sign.mount = Some(Mount::Decal { on: bevy_emerge_core::descriptor::DecalHost::Floor });
    let l = lib(vec![table(around(0.91)), rug, sign]);
    let m = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("rug@0", "rug", (0.0, 1.6)),
        at("sign@0", "sign", (0.0, -1.6)),
    ]);
    assert!(
        blocked_clearances(&m, &l).is_empty(),
        "a rug and a floor decal are not things standing in the way"
    );
}

/// **A rider is part of its keeper, and a piece above the keeper's span is clear of it.** The first is
/// `Placed::on`, the second is `resolve_y`: a sconce at 2.5 m is above a 0.76 m table's pull-out space.
#[test]
fn a_rider_and_a_piece_above_the_keeper_are_not_in_its_way() {
    let mut sconce = chair();
    sconce.id = "sconce".into();
    sconce.mount = Some(Mount::OnWall { height: 2.5 });
    sconce.extent.height = Some(0.4);
    let l = lib(vec![table(around(0.91)), chair(), sconce]);

    let mut rider = at("rider@0", "chair", (0.0, 1.6));
    rider.on = Some("table@0".into());
    let with_rider = map(vec![at("table@0", "table", (0.0, 0.0)), rider]);
    assert!(
        blocked_clearances(&with_rider, &l).is_empty(),
        "a thing mounted on the table is part of it"
    );

    let above = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("sconce@0", "sconce", (0.0, 1.6)),
    ]);
    assert!(
        blocked_clearances(&above, &l).is_empty(),
        "the sconce starts at 2.5 m and the table's span ends at 0.76 m"
    );
}

/// **No clearance reserves nothing, and an unmeasured piece takes part in nothing.** The first is the
/// shipped state of every descriptor in `assets/`; the second is `covers`' "unknown is not everywhere".
#[test]
fn nothing_reserved_and_nothing_measured_report_nothing() {
    let l = lib(vec![table(Vec::new()), chair()]);
    let m = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 0.6)),
    ]);
    assert!(
        blocked_clearances(&m, &l).is_empty(),
        "a table that reserved nothing has nothing to stand in"
    );

    let mut unmeasured = table(around(0.91));
    unmeasured.id = "unmeasured".into();
    unmeasured.extent = Extent::default();
    let l = lib(vec![unmeasured, chair()]);
    let m = map(vec![
        at("unmeasured@0", "unmeasured", (0.0, 0.0)),
        at("chair@0", "chair", (0.0, 0.0)),
    ]);
    assert!(
        blocked_clearances(&m, &l).is_empty(),
        "an unmeasured keeper cannot say where its band is"
    );
}

/// **One intruder pressed by two `Around` bands is one finding, and the list is ordered.** A chair in
/// the corner of a table's `Around` band sits inside two of its four bands at once; the finding is one
/// pair, and a second map with the placements in the other order answers the same set.
#[test]
fn the_list_is_deduplicated_and_ordered() {
    let l = lib(vec![table(around(0.91)), chair()]);
    // A corner: inside both the +Z and the +X bands.
    let m = map(vec![
        at("table@0", "table", (0.0, 0.0)),
        at("chair@0", "chair", (1.3, 1.3)),
    ]);
    assert_eq!(
        blocked_clearances(&m, &l),
        vec![(0, 1)],
        "the chair is in the corner of the band and is named once"
    );
}
