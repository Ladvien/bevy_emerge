//! **Relations, not coordinates: balances** (#185, research lens #2).
//!
//! Every other relation is about a pair (or a path), so three stalls each `Near` the well can all
//! crowd one side of it and every relation still holds. Merrell et al. (*Interactive Furniture
//! Layout Using Interior Design Guidelines*, §2.2) name visual balance the most widely known
//! principle of composition: the mean of the distribution of visual weight sits at the centre of
//! the composition, evaluated on the ground plane. `Kind::Balances` keeps the mean of the centres
//! of a placement and its `with` group within `within` metres of `about`'s centre. Every placement
//! weighs the same (the issue's assumed decision: relations read rows, never the kit's areas).
//! `check` measures the distance from the mean to that centre; `solve` moves the relation's own
//! placement so the mean lands on it, keeping its heading.
//!
//! Rows are a map's, yaw in degrees, through `check_map` and `solve_map`.

use bevy_emerge_core::map::Placed;
use bevy_emerge_core::relations::{check_map, solve_map, Kind, Outcome, Relation, Relations, RELATIONS_VERSION};

fn piece(id: &str, at: (f32, f32), yaw_deg: f32) -> Placed {
    Placed {
        id: id.to_owned(),
        descriptor: id.to_owned(),
        at,
        yaw: yaw_deg,
        ..Placed::default()
    }
}

/// `stall_a` balances with `with` about `about`, within `within` metres.
fn balances(with: &[&str], about: &str, within: f32) -> Relations {
    Relations {
        version: RELATIONS_VERSION,
        relations: vec![Relation {
            of: "stall_a".into(),
            kind: Kind::Balances {
                with: with.iter().map(|s| (*s).to_owned()).collect(),
                about: about.into(),
                within,
            },
            hard: false,
        }],
    }
}

/// A well away from the origin at (10, 5); `stall_b` east of it, `stall_c` south; `stall_a` at
/// `at`, turned to `a_deg`. The three balance about the well when `stall_a` stands at (8, 3).
fn square(at: (f32, f32), a_deg: f32) -> Vec<Placed> {
    vec![
        piece("well", (10.0, 5.0), 0.0),
        piece("stall_a", at, a_deg),
        piece("stall_b", (12.0, 5.0), 90.0),
        piece("stall_c", (10.0, 7.0), 180.0),
    ]
}

fn outcome(at: (f32, f32), within: f32) -> Outcome {
    check_map(&square(at, 0.0), &balances(&["stall_b", "stall_c"], "well", within)).remove(0)
}

fn measured(o: &Outcome) -> f32 {
    match o {
        Outcome::Met { measured } | Outcome::Unmet { measured, .. } => *measured,
        Outcome::Missing { id } => panic!("{id} missing"),
    }
}

#[test]
fn a_group_whose_mean_is_on_the_point_is_met_and_measures_nothing() {
    let o = outcome((8.0, 3.0), 0.5);
    assert!(matches!(o, Outcome::Met { .. }), "{o:?}");
    assert!(measured(&o) < 1e-4, "{o:?}");
}

#[test]
fn a_lopsided_group_is_unmet_by_how_far_its_mean_stands_off() {
    // Mean of (11, 6), (12, 5), (10, 7) is (11, 6): √2 off the well.
    let o = outcome((11.0, 6.0), 1.0);
    assert!(matches!(o, Outcome::Unmet { by: None, .. }), "{o:?}");
    assert!((measured(&o) - 2f32.sqrt()).abs() < 1e-4, "{o:?}");
    // Each stall is within 2.3 m of the well, so a Near on each would hold: only the group is off.
    for (stall, at) in [("stall_a", (11.0, 6.0)), ("stall_b", (12.0, 5.0)), ("stall_c", (10.0, 7.0))] {
        let d = ((at.0 - 10.0f32).powi(2) + (at.1 - 5.0f32).powi(2)).sqrt();
        assert!(d < 2.3, "{stall} {d}");
    }
    // Within a wider bound the same group is met.
    let o = outcome((11.0, 6.0), 1.5);
    assert!(matches!(o, Outcome::Met { .. }), "{o:?}");
    assert!((measured(&o) - 2f32.sqrt()).abs() < 1e-4, "{o:?}");
}

#[test]
fn a_missing_member_or_centre_is_missing() {
    let rows = square((8.0, 3.0), 0.0);
    let o = check_map(&rows, &balances(&["stall_b", "stall_z"], "well", 1.0)).remove(0);
    assert_eq!(o, Outcome::Missing { id: "stall_z".into() });
    let o = check_map(&rows, &balances(&["stall_b", "stall_c"], "fountain", 1.0)).remove(0);
    assert_eq!(o, Outcome::Missing { id: "fountain".into() });
}

#[test]
fn solve_moves_only_its_own_piece_so_the_mean_lands_on_the_centre_keeping_its_heading() {
    let before = square((13.0, 9.0), 37.0);
    let rel = balances(&["stall_b", "stall_c"], "well", 0.25);
    let solved = solve_map(&before, &rel);
    assert_eq!(solved.unmet_before, 1);
    assert_eq!(solved.unmet_after, 0, "{:?}", solved.unsatisfied);
    assert_eq!(solved.moved, vec!["stall_a".to_owned()]);
    let a = solved.placements.iter().find(|p| p.id == "stall_a").unwrap();
    assert!((a.at.0 - 8.0).abs() < 1e-3 && (a.at.1 - 3.0).abs() < 1e-3, "{:?}", a.at);
    assert!((a.yaw - 37.0).abs() < 1e-3, "kept its heading: {}", a.yaw);
    for i in [0, 2, 3] {
        assert_eq!(before[i], solved.placements[i], "{} moved", before[i].id);
    }
    assert!(matches!(check_map(&solved.placements, &rel).remove(0), Outcome::Met { .. }));

    // The centre may itself be one of the group: the well weighs in like a stall.
    let rel = balances(&["stall_b", "well"], "well", 0.25);
    let solved = solve_map(&before, &rel);
    assert_eq!(solved.unmet_after, 0, "{:?}", solved.unsatisfied);
    let a = solved.placements.iter().find(|p| p.id == "stall_a").unwrap();
    // 3 · (10, 5) − (12, 5) − (10, 5) = (8, 5).
    assert!((a.at.0 - 8.0).abs() < 1e-3 && (a.at.1 - 5.0).abs() < 1e-3, "{:?}", a.at);
}

#[test]
fn a_piece_that_is_its_own_centre_is_solved_to_the_mean_of_the_others() {
    // The well at the centre of its stalls: it is the centre and one of the group, so moving it
    // moves the centre too, and it lands on the others' mean.
    let rows = vec![
        piece("well", (0.0, 0.0), 15.0),
        piece("stall_a", (8.0, 3.0), 0.0),
        piece("stall_b", (12.0, 5.0), 0.0),
        piece("stall_c", (10.0, 7.0), 0.0),
    ];
    let rel = Relations {
        version: RELATIONS_VERSION,
        relations: vec![Relation {
            of: "well".into(),
            kind: Kind::Balances {
                with: vec!["stall_a".into(), "stall_b".into(), "stall_c".into()],
                about: "well".into(),
                within: 0.25,
            },
            hard: false,
        }],
    };
    let o = check_map(&rows, &rel).remove(0);
    // Mean of the four is (7.5, 3.75): its distance from the well at the origin.
    assert!((measured(&o) - 7.5f32.hypot(3.75)).abs() < 1e-3, "{o:?}");
    let solved = solve_map(&rows, &rel);
    assert_eq!(solved.unmet_after, 0, "{:?}", solved.unsatisfied);
    let well = solved.placements.iter().find(|p| p.id == "well").unwrap();
    assert!((well.at.0 - 10.0).abs() < 1e-3 && (well.at.1 - 5.0).abs() < 1e-3, "{:?}", well.at);
    assert!((well.yaw - 15.0).abs() < 1e-3, "kept its heading: {}", well.yaw);
}

#[test]
fn an_empty_or_self_naming_group_and_a_negative_bound_are_refused() {
    let text = |with: &str, within: &str| {
        format!("(version: {RELATIONS_VERSION}, relations: [(of: \"stall_a\", kind: Balances(with: [{with}], about: \"well\", within: {within}))])")
    };
    let err = Relations::parse(&text("", "1.0")).expect_err("an empty group is Near, not a balance");
    assert!(err.contains("with"), "{err}");
    let err = Relations::parse(&text("\"stall_b\", \"stall_a\"", "1.0")).expect_err("a piece counted twice");
    assert!(err.contains("stall_a"), "{err}");
    let err = Relations::parse(&text("\"stall_b\"", "-1.0")).expect_err("a negative within must not read");
    assert!(err.contains("-1"), "{err}");
    let err = Relations::parse(&text("\"stall_b\", \"stall_c\", \"stall_b\"", "1.0")).expect_err("one stall weighing double");
    assert!(err.contains("stall_b") && err.contains("twice"), "{err}");
    let r = Relations::parse(&text("\"stall_b\", \"stall_c\"", "1.0")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r, balances(&["stall_b", "stall_c"], "well", 1.0));
}
