//! **Which flat piece lies on top where two of them share ground.**
//!
//! Two footprint markers laid over the same patch of floor have top faces at the same height, so the
//! depth buffer decides their pixels by coin toss — the z-fight. The fix is a depth separation between
//! them, and the separation has to be *predictable to an author* and *the same in every consumer*, so
//! it is decided here, once, from the map alone.
//!
//! **The rule.** Pieces are ordered bottom to top by three keys, in this order:
//!
//! 1. **Authored paint**, lowest first. [`crate::map::Placed::paint`] is the author's explicit order;
//!    it always wins.
//! 2. **Area, largest first.** A smaller flat piece laid on a larger one is the ground detail — a
//!    road on a field, a rug on the floor — so the smaller piece draws on top.
//! 3. **Placement order, earliest first.** Equal paint and equal area: the piece placed later is
//!    drawn on top.
//!
//! A piece's **level** is how far up the order it has to sit to clear every piece below it that it
//! overlaps: `0` for a piece that overlaps nothing beneath it, otherwise one more than the highest
//! level among the overlapping pieces below. Levels are relative: they only need to be strictly
//! increasing up every overlapping pair, which the chain construction guarantees.
//!
//! Overlap is an open-interval test: pieces that only share an edge do not overlap, so two roads
//! meeting end to end are not ordered against each other.

/// How far, in metres, one draw level lifts a flat piece's top face above the floor.
///
/// Levels must reach the screen as **world height**, not as a depth bias. A bias step is a comparison
/// nudge, and at the Map camera's ~2,000 m depth range a field-sized quad's interpolation error is
/// larger than one or two steps, so two coplanar quads trade pixels in stair steps (the B-015
/// crossing, `target/loop/2026-10-21-98be109/look/B-015-pair.png`). One centimetre is about forty
/// depth steps at that range and is invisible to an author; the game itself lifts roads by 0.15 m
/// (Sanctuary `src/prototype/town.rs:60`). The lift is render-only: it never reaches placement data
/// or `stack::resolve_y`.
pub const LEVEL_LIFT: f32 = 0.01;

/// One flat piece's footprint on the floor, as the draw order sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    /// The four corners of the footprint, in floor plan metres, in order around the rectangle.
    pub corners: [(f32, f32); 4],
    /// Footprint area in square metres. Larger areas are drawn lower.
    pub area: f32,
    /// The author's explicit paint order. Lower is drawn lower.
    pub paint: i8,
}

/// Pieces must be at least this far apart, in metres, to be treated as not overlapping. Edge contact
/// is not overlap.
const EPSILON: f32 = 1e-4;

/// The most grid cells one piece may be filed under before it is kept aside and compared with
/// everything instead. A field-sized piece over a town's cell size would otherwise be filed under
/// tens of thousands of cells it may share with nothing — the insertion cost the grid exists to avoid.
const MAX_CELLS_PER_PIECE: i64 = 256;

thread_local! {
    /// **How many separating-axis tests this thread's `draw_levels` has run** — E-020's measure.
    ///
    /// A thread, not a process: libtest gives every test its own thread. The levels themselves do not
    /// change; only the number of pairs the order has to look at does, which is the thing that was
    /// wrong — a whole map's change compared every standing pair when almost none of them can
    /// overlap.
    static OVERLAP_TESTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// **The pairwise overlap tests `draw_levels` has made on this thread**, for the test that pins the
/// cost of a whole-map redraw.
///
/// Reads a count rather than a clock: a level answer is the same however it was reached, so a
/// comparison of the returned levels cannot see the wasted work at all — which is why it survived.
pub fn overlap_tests_for_test() -> usize {
    OVERLAP_TESTS.with(std::cell::Cell::get)
}

/// The axis-aligned box a footprint occupies: `(min x, min z, max x, max z)`.
fn bounds(corners: &[(f32, f32); 4]) -> (f32, f32, f32, f32) {
    corners.iter().fold(
        (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        |(x0, y0, x1, y1), &(x, y)| (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
    )
}

/// **The side of one grid cell, in metres** — the mean footprint span, so a typical piece touches a
/// handful of cells. Falls back to one metre rather than zero, which would divide by nothing.
fn cell_size(order: &[usize], ground: &[Option<Ground>]) -> f32 {
    let mut span = 0.0f32;
    let mut n = 0usize;
    for &i in order {
        if let Some(g) = ground[i].as_ref() {
            let (x0, y0, x1, y1) = bounds(&g.corners);
            span += (x1 - x0).max(y1 - y0);
            n += 1;
        }
    }
    let mean = if n == 0 { 0.0 } else { span / n as f32 };
    if mean.is_finite() && mean > 0.0 {
        mean
    } else {
        1.0
    }
}

/// One piece's cells, as an inclusive integer range, or `None` when its box cannot be placed in the
/// grid (a non-finite corner, or a span too large to file).
fn cell_range(
    corners: &[(f32, f32); 4],
    cell: f32,
) -> Option<(i32, i32, i32, i32)> {
    let (x0, y0, x1, y1) = bounds(corners);
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
        return None;
    }
    let (cx0, cy0) = ((x0 / cell).floor(), (y0 / cell).floor());
    let (cx1, cy1) = ((x1 / cell).floor(), (y1 / cell).floor());
    let span = ((cx1 - cx0 + 1.0) * (cy1 - cy0 + 1.0)) as f64;
    if !(span.is_finite() && span >= 1.0 && span <= MAX_CELLS_PER_PIECE as f64) {
        return None;
    }
    Some((cx0 as i32, cy0 as i32, cx1 as i32, cy1 as i32))
}

/// The level of each piece under the rule in the module docs. `None` is a piece with no footprint:
/// it draws at level `0` and is ordered against nothing.
///
/// **Only pieces that can overlap are compared.** A level is a maximum over the pieces below that
/// overlap, and two footprints that overlap must share a grid cell, so each piece asks the cells it
/// covers rather than every piece ordered before it — E-020's second half. On the shipped town this
/// is a few dozen tests where the pairwise loop made hundreds of thousands per whole-map change.
pub fn draw_levels(ground: &[Option<Ground>]) -> Vec<i8> {
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
    let cell = cell_size(&order, ground);
    // **The cells already filled, by footprint**, and the pieces too large to file. Both are keyed by
    // position in `order`, so every candidate is a piece that is already below the one being levelled.
    let mut cells: std::collections::HashMap<(i32, i32), Vec<usize>> =
        std::collections::HashMap::new();
    let mut aside: Vec<usize> = Vec::new();
    // One stamp per piece, so a piece filed under several cells is still tested once.
    let mut seen = vec![0u64; order.len()];
    let mut candidates: Vec<usize> = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        let gi = ground[i].as_ref().expect("filtered to footprints");
        let stamp = k as u64 + 1;
        candidates.clear();
        let range = cell_range(&gi.corners, cell);
        match range {
            Some((cx0, cy0, cx1, cy1)) => {
                // **A piece kept aside may overlap this one anywhere**, so it is always a candidate.
                candidates.extend_from_slice(&aside);
                for cx in cx0..=cx1 {
                    for cy in cy0..=cy1 {
                        if let Some(bucket) = cells.get(&(cx, cy)) {
                            candidates.extend_from_slice(bucket);
                        }
                    }
                }
            }
            // **Too large (or unplaceable) to file**: nothing about its box prunes anything, so it
            // compares with every piece below it, exactly as the pairwise loop did.
            None => candidates.extend(0..k),
        }
        let mut level = 0i8;
        for j in candidates.drain(..) {
            if seen[j] == stamp {
                continue;
            }
            seen[j] = stamp;
            let gj = ground[order[j]].as_ref().expect("filtered to footprints");
            if overlaps(&gi.corners, &gj.corners) {
                level = level.max(levels[order[j]].saturating_add(1));
            }
        }
        levels[i] = level;
        match range {
            Some((cx0, cy0, cx1, cy1)) => {
                for cx in cx0..=cx1 {
                    for cy in cy0..=cy1 {
                        cells.entry((cx, cy)).or_default().push(k);
                    }
                }
            }
            None => aside.push(k),
        }
    }
    levels
}

/// Whether two convex quadrilaterals overlap with positive area, by the separating-axis test: they
/// are disjoint exactly when some edge normal of either one separates the two projections.
fn overlaps(a: &[(f32, f32); 4], b: &[(f32, f32); 4]) -> bool {
    OVERLAP_TESTS.with(|n| n.set(n.get() + 1));
    !separated(a, b) && !separated(b, a)
}

/// Whether some edge of `poly` has `other` entirely on its outer side.
fn separated(poly: &[(f32, f32); 4], other: &[(f32, f32); 4]) -> bool {
    (0..4).any(|i| {
        let (x0, y0) = poly[i];
        let (x1, y1) = poly[(i + 1) % 4];
        // Outward-facing axis is the edge normal; any consistent normal works for a separation test.
        let (nx, ny) = (y1 - y0, x0 - x1);
        let (pmin, pmax) = project(poly, nx, ny);
        let (omin, omax) = project(other, nx, ny);
        omax <= pmin + EPSILON || pmax <= omin + EPSILON
    })
}

fn project(points: &[(f32, f32); 4], nx: f32, ny: f32) -> (f32, f32) {
    points
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &(x, y)| {
            let d = x * nx + y * ny;
            (lo.min(d), hi.max(d))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(cx: f32, cy: f32, half: f32) -> [(f32, f32); 4] {
        [
            (cx - half, cy - half),
            (cx + half, cy - half),
            (cx + half, cy + half),
            (cx - half, cy + half),
        ]
    }

    fn piece(corners: [(f32, f32); 4], area: f32, paint: i8) -> Option<Ground> {
        Some(Ground {
            corners,
            area,
            paint,
        })
    }

    #[test]
    fn disjoint_pieces_are_both_level_zero() {
        let levels = draw_levels(&[
            piece(square(0.0, 0.0, 1.0), 4.0, 0),
            piece(square(10.0, 0.0, 1.0), 4.0, 0),
        ]);
        assert_eq!(levels, vec![0, 0]);
    }

    #[test]
    fn edge_contact_is_not_overlap() {
        let levels = draw_levels(&[
            piece(square(0.0, 0.0, 1.0), 4.0, 0),
            piece(square(2.0, 0.0, 1.0), 4.0, 0),
        ]);
        assert_eq!(levels, vec![0, 0]);
    }

    #[test]
    fn the_smaller_piece_draws_on_top_of_the_larger() {
        // Road (small) over field (large): the field is index 0, the road index 1.
        let levels = draw_levels(&[
            piece(square(0.0, 0.0, 6.0), 144.0, 0),
            piece(square(0.0, 0.0, 2.0), 16.0, 0),
        ]);
        assert!(
            levels[1] > levels[0],
            "smaller piece must be higher: {levels:?}"
        );
    }

    #[test]
    fn equal_area_later_placement_draws_on_top() {
        let levels = draw_levels(&[
            piece(square(0.0, 0.0, 1.0), 4.0, 0),
            piece(square(0.0, 0.0, 1.0), 4.0, 0),
        ]);
        assert!(
            levels[1] > levels[0],
            "later placement must be higher: {levels:?}"
        );
    }

    #[test]
    fn authored_paint_beats_area() {
        // The large piece is painted above the small one: paint wins.
        let levels = draw_levels(&[
            piece(square(0.0, 0.0, 6.0), 144.0, 1),
            piece(square(0.0, 0.0, 2.0), 16.0, 0),
        ]);
        assert!(
            levels[0] > levels[1],
            "painted piece must be higher: {levels:?}"
        );
    }

    #[test]
    fn a_chain_of_overlaps_stacks_levels() {
        let levels = draw_levels(&[
            piece(square(0.0, 0.0, 6.0), 144.0, 0),
            piece(square(0.0, 0.0, 4.0), 64.0, 0),
            piece(square(0.0, 0.0, 2.0), 16.0, 0),
        ]);
        assert_eq!(levels, vec![0, 1, 2]);
    }

    #[test]
    fn a_piece_without_footprint_is_ignored() {
        let levels = draw_levels(&[None, piece(square(0.0, 0.0, 1.0), 4.0, 0)]);
        assert_eq!(levels, vec![0, 0]);
    }
}
