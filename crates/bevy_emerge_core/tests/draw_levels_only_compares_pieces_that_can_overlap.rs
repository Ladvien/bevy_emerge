//! **A whole-map redraw compares only the pieces that can overlap** — E-020's second half.
//!
//! # What was wrong
//!
//! `draw_levels` levelled each piece against **every** piece ordered before it: `O(n^2)`
//! separating-axis tests per call. `relevel_placements` calls `drawn_paints` — and so `draw_levels` —
//! whenever the open map changes, and `redraw_placements` and every spawn path call it too. On the
//! shipped town (727 pieces) that is about 0.5 M pair tests for an edit that touched one piece, and
//! on a map of scattered houses and fields almost none of those pairs are anywhere near each other.
//!
//! # The rule the fix must keep
//!
//! A level is a maximum over the overlapping pieces **below** the piece in the order, so pruning may
//! only drop pieces that provably cannot overlap. Two footprints that overlap share ground, so they
//! share a grid cell — and the fix files each piece under the cells its bounding box covers and asks
//! only those. The answer must therefore be **identical** to the pairwise order; only the work may
//! differ. These tests pin both halves: the same levels, and far fewer tests.
//!
//! # Why the count and not a clock
//!
//! A timing assertion on a shared machine is a coin toss (E-011, E-023). The count of pairwise
//! overlap tests is deterministic and is exactly the thing that was wrong.

use bevy_emerge_core::draw_order::{draw_levels, overlap_tests_for_test, Ground, LEVEL_LIFT};

/// The order the module docs describe, written the slow way: every earlier piece is a candidate.
fn levels_the_slow_way(ground: &[Option<Ground>]) -> Vec<i8> {
    let mut levels = vec![0i8; ground.len()];
    let mut order: Vec<usize> = (0..ground.len()).filter(|&i| ground[i].is_some()).collect();
    order.sort_by(|&a, &b| {
        let (ga, gb) = (ground[a].as_ref(), ground[b].as_ref());
        match (ga, gb) {
            (Some(ga), Some(gb)) => ga
                .paint
                .cmp(&gb.paint)
                .then(gb.area.total_cmp(&ga.area))
                .then(a.cmp(&b)),
            _ => a.cmp(&b),
        }
    });
    for (k, &i) in order.iter().enumerate() {
        let gi = ground[i].as_ref().expect("filtered to footprints");
        let mut level = 0i8;
        for &j in &order[..k] {
            let gj = ground[j].as_ref().expect("filtered to footprints");
            if overlaps_aabb_then_sat(&gi.corners, &gj.corners) {
                level = level.max(levels[j].saturating_add(1));
            }
        }
        levels[i] = level;
    }
    levels
}

/// The overlap test, copied out of the module so this file is an independent second opinion.
fn overlaps_aabb_then_sat(a: &[(f32, f32); 4], b: &[(f32, f32); 4]) -> bool {
    fn separated(poly: &[(f32, f32); 4], other: &[(f32, f32); 4]) -> bool {
        (0..4).any(|i| {
            let (x0, y0) = poly[i];
            let (x1, y1) = poly[(i + 1) % 4];
            let (nx, ny) = (y1 - y0, x0 - x1);
            let project = |points: &[(f32, f32); 4]| {
                points.iter().fold(
                    (f32::INFINITY, f32::NEG_INFINITY),
                    |(lo, hi), &(x, y)| {
                        let d = x * nx + y * ny;
                        (lo.min(d), hi.max(d))
                    },
                )
            };
            let (pmin, pmax) = project(poly);
            let (omin, omax) = project(other);
            omax <= pmin + 1e-4 || pmax <= omin + 1e-4
        })
    }
    !separated(a, b) && !separated(b, a)
}

fn square(cx: f32, cy: f32, half: f32) -> [(f32, f32); 4] {
    [
        (cx - half, cy - half),
        (cx + half, cy - half),
        (cx + half, cy + half),
        (cx - half, cy + half),
    ]
}

fn piece(cx: f32, cy: f32, half: f32, paint: i8) -> Option<Ground> {
    Some(Ground {
        corners: square(cx, cy, half),
        area: (half * 2.0) * (half * 2.0),
        paint,
    })
}

/// **A town of scattered pieces**: 400 one-metre posts on a 20 x 20 grid two metres apart, so no two
/// of them touch, plus a handful of overlapping rows to keep the level chain non-trivial. Nothing here
/// is authored by hand — the shape is Sanctuary's: many small pieces, most of them nowhere near each
/// other.
fn a_scattered_town() -> Vec<Option<Ground>> {
    let mut ground = Vec::new();
    for x in 0..20 {
        for z in 0..20 {
            ground.push(piece(x as f32 * 2.0, z as f32 * 2.0, 0.5, 0));
        }
    }
    // A road crossing a field, and a rug on the road: three levels where the town's own data has them.
    ground.push(piece(10.0, 10.0, 6.0, 0));
    ground.push(piece(10.0, 10.0, 2.0, 0));
    ground.push(piece(10.0, 10.0, 0.4, 0));
    ground
}

#[test]
fn the_pruned_order_gives_the_same_levels_as_the_slow_one() {
    let ground = a_scattered_town();
    assert_eq!(draw_levels(&ground), levels_the_slow_way(&ground));
}

#[test]
fn the_pruned_order_gives_the_same_levels_for_a_chain() {
    // Four nested squares: level 0, 1, 2, 3. The chain is where a dropped candidate would show up as
    // a level that is one too small.
    let ground: Vec<Option<Ground>> = [6.0f32, 4.0, 2.0, 1.0]
        .iter()
        .map(|&h| piece(0.0, 0.0, h, 0))
        .collect();
    assert_eq!(draw_levels(&ground), levels_the_slow_way(&ground));
    assert_eq!(draw_levels(&ground), vec![0, 1, 2, 3]);
}

#[test]
fn a_turn_does_not_change_which_pieces_are_compared() {
    // A quarter-turned strip has an axis-aligned box larger than its footprint. Sharing a cell is a
    // necessary condition, not a sufficient one, so the turn must not turn two disjoint pieces into an
    // order — the separating-axis test still has the last word.
    let mut ground = vec![
        Some(Ground {
            corners: [(0.0, 0.0), (6.0, 0.0), (6.0, 1.0), (0.0, 1.0)],
            area: 6.0,
            paint: 0,
        }),
        Some(Ground {
            corners: [(0.0, 0.0), (1.0, 0.0), (1.0, 6.0), (0.0, 6.0)],
            area: 6.0,
            paint: 0,
        }),
    ];
    assert_eq!(draw_levels(&ground), levels_the_slow_way(&ground));
    // Disjoint boxes, no overlap, both level 0.
    ground.reverse();
    assert_eq!(draw_levels(&ground), levels_the_slow_way(&ground));
    assert_eq!(draw_levels(&ground), vec![0, 0]);
}

#[test]
fn a_piece_with_no_footprint_is_still_ignored() {
    let ground = vec![None, piece(0.0, 0.0, 1.0, 0), None];
    assert_eq!(draw_levels(&ground), vec![0, 0, 0]);
}

#[test]
fn an_enormous_piece_still_orders_against_what_it_covers() {
    // A field 400 m across dwarfs the grid cell, so it is kept aside rather than filed under tens of
    // thousands of cells — and kept aside must still mean *compared with everything*, or a piece laid
    // on it would draw at level 0.
    let mut ground = vec![Some(Ground {
        corners: square(0.0, 0.0, 200.0),
        area: 160_000.0,
        paint: 0,
    })];
    for i in 0..50 {
        ground.push(piece(i as f32 * 7.0 - 170.0, 3.0, 1.0, 0));
    }
    assert_eq!(draw_levels(&ground), levels_the_slow_way(&ground));
    assert!(draw_levels(&ground)[1..].iter().all(|&l| l > 0));
}

#[test]
fn a_scattered_town_is_compared_far_less_than_pairwise() {
    let ground = a_scattered_town();
    let n = ground.len();
    let before = overlap_tests_for_test();
    draw_levels(&ground);
    let made = overlap_tests_for_test() - before;
    let pairwise = n * (n.saturating_sub(1)) / 2;
    assert!(
        made * 10 < pairwise,
        "the pairwise order would make {pairwise} overlap tests for {n} pieces and this made {made}"
    );
}

#[test]
fn the_lift_is_still_one_centimetre() {
    // The levels this file pins reach the screen multiplied by `LEVEL_LIFT`; a change to the step is a
    // change to every map's z-fight and must not ride along with a speed fix.
    assert!((LEVEL_LIFT - 0.01).abs() < f32::EPSILON);
}
